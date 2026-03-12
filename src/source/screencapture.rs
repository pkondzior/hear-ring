use std::mem::size_of;
use std::sync::{Arc, Mutex};

use screencapturekit::prelude::*;

use super::{AudioSource, AudioSourceState};
use crate::types::{ChannelEnergies, ChannelLayout};

#[derive(Debug, Clone, Copy, Default)]
struct StereoAnalysis {
    left_rms: f32,
    right_rms: f32,
    pan: f32,
    width: f32,
}

#[derive(Debug)]
enum StereoDecodeError {
    InvalidSampleBuffer,
    SampleNotReady(i32),
    MissingFormatDescription,
    UnsupportedAudioFormat,
}

impl StereoDecodeError {
    fn message(&self) -> String {
        match self {
            StereoDecodeError::InvalidSampleBuffer => "Audio sample buffer is invalid".to_owned(),
            StereoDecodeError::SampleNotReady(status) => {
                format!("Audio sample not ready (status {status})")
            }
            StereoDecodeError::MissingFormatDescription => {
                "Audio sample missing format description".to_owned()
            }
            StereoDecodeError::UnsupportedAudioFormat => {
                "Audio format is not supported PCM".to_owned()
            }
        }
    }
}

#[derive(Clone)]
struct SharedCaptureState {
    energies: ChannelEnergies,
    state: AudioSourceState,
}

impl Default for SharedCaptureState {
    fn default() -> Self {
        Self {
            energies: ChannelEnergies::default(),
            state: AudioSourceState::Starting,
        }
    }
}

pub struct ScreenCaptureSource {
    shared: Arc<Mutex<SharedCaptureState>>,
    _stream: Option<SCStream>,
}

impl ScreenCaptureSource {
    pub fn new() -> Self {
        let shared = Arc::new(Mutex::new(SharedCaptureState::default()));

        let stream = match Self::start_capture(shared.clone()) {
            Ok(stream) => {
                if let Ok(mut guard) = shared.lock() {
                    guard.state = AudioSourceState::Running;
                }
                Some(stream)
            }
            Err(state) => {
                if let Ok(mut guard) = shared.lock() {
                    guard.state = state;
                }
                None
            }
        };

        Self {
            shared,
            _stream: stream,
        }
    }

    fn start_capture(shared: Arc<Mutex<SharedCaptureState>>) -> Result<SCStream, AudioSourceState> {
        let content = SCShareableContent::get().map_err(Self::map_error)?;

        let display = content.displays().into_iter().next().ok_or_else(|| {
            AudioSourceState::Error("No display available for capture".to_owned())
        })?;

        let filter = SCContentFilter::create()
            .with_display(&display)
            .with_excluding_windows(&[])
            .build();

        let config = SCStreamConfiguration::new()
            .with_width(display.width())
            .with_height(display.height())
            .with_captures_audio(true)
            .with_sample_rate(48_000)
            .with_channel_count(2);

        let mut stream = SCStream::new(&filter, &config);
        let handler = AudioCaptureHandler { shared };

        stream.add_output_handler(handler, SCStreamOutputType::Audio);
        stream.start_capture().map_err(Self::map_error)?;

        Ok(stream)
    }

    fn map_error(error: impl std::fmt::Display) -> AudioSourceState {
        let message = error.to_string();
        let lower = message.to_lowercase();

        // ScreenCaptureKit fails early when Screen Recording permission has not been granted.
        // The exact error type/message varies across macOS versions and host apps, so we
        // conservatively match common substrings.
        if lower.contains("screen recording")
            || lower.contains("not authorized")
            || lower.contains("not authorised")
            || (lower.contains("permission") && lower.contains("denied"))
            || (lower.contains("access") && lower.contains("denied"))
        {
            return AudioSourceState::PermissionDenied;
        }

        AudioSourceState::Error(message)
    }
}

impl AudioSource for ScreenCaptureSource {
    fn layout(&self) -> ChannelLayout {
        ChannelLayout::Stereo
    }

    fn next_energies(&mut self, _dt: f32) -> ChannelEnergies {
        self.shared
            .lock()
            .map(|guard| guard.energies)
            .unwrap_or_default()
    }

    fn state(&self) -> AudioSourceState {
        self.shared
            .lock()
            .map(|guard| guard.state.clone())
            .unwrap_or_else(|_| AudioSourceState::Error("Capture state lock poisoned".to_owned()))
    }
}

struct AudioCaptureHandler {
    shared: Arc<Mutex<SharedCaptureState>>,
}

impl SCStreamOutputTrait for AudioCaptureHandler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, _type: SCStreamOutputType) {
        match StereoAnalysis::try_from(&sample) {
            Ok(analysis) => {
                if let Ok(mut guard) = self.shared.lock() {
                    guard.energies = analysis.into();
                    guard.state = AudioSourceState::Running;
                }
            }
            Err(error) => {
                if let Ok(mut guard) = self.shared.lock() {
                    guard.state = AudioSourceState::Error(error.message());
                }
            }
        }
    }
}

impl TryFrom<&CMSampleBuffer> for StereoAnalysis {
    type Error = StereoDecodeError;

    fn try_from(sample: &CMSampleBuffer) -> Result<Self, Self::Error> {
        if !sample.is_valid() {
            return Err(StereoDecodeError::InvalidSampleBuffer);
        }

        if !sample.is_data_ready() {
            sample
                .make_data_ready()
                .map_err(StereoDecodeError::SampleNotReady)?;
        }

        let format = sample
            .format_description()
            .ok_or(StereoDecodeError::MissingFormatDescription)?;

        if !format.is_audio() || !format.is_pcm() {
            return Err(StereoDecodeError::UnsupportedAudioFormat);
        }

        let channel_count = format
            .audio_channel_count()
            .ok_or(StereoDecodeError::UnsupportedAudioFormat)? as usize;

        if channel_count == 0 {
            return Err(StereoDecodeError::UnsupportedAudioFormat);
        }

        let bits_per_channel = format
            .audio_bits_per_channel()
            .ok_or(StereoDecodeError::UnsupportedAudioFormat)?
            as usize;

        let bytes_per_frame = format
            .audio_bytes_per_frame()
            .ok_or(StereoDecodeError::UnsupportedAudioFormat)?
            as usize;

        if bits_per_channel != 32 || bytes_per_frame == 0 || !format.audio_is_float() {
            return Err(StereoDecodeError::UnsupportedAudioFormat);
        }

        let buffers = sample
            .audio_buffer_list()
            .ok_or(StereoDecodeError::UnsupportedAudioFormat)?;

        let analysis = match buffers.num_buffers() {
            1 => analyze_interleaved(
                buffers
                    .get(0)
                    .ok_or(StereoDecodeError::UnsupportedAudioFormat)?
                    .data(),
                channel_count,
                bytes_per_frame,
            )?,
            count if count >= 2 => analyze_planar(
                buffers
                    .get(0)
                    .ok_or(StereoDecodeError::UnsupportedAudioFormat)?
                    .data(),
                buffers
                    .get(1)
                    .ok_or(StereoDecodeError::UnsupportedAudioFormat)?
                    .data(),
            )?,
            _ => return Err(StereoDecodeError::UnsupportedAudioFormat),
        };

        Ok(analysis)
    }
}

impl From<StereoAnalysis> for ChannelEnergies {
    fn from(analysis: StereoAnalysis) -> Self {
        Self {
            fl: analysis.left_rms,
            fr: analysis.right_rms,
            stereo_pan: analysis.pan,
            stereo_width: analysis.width,
            ..Self::default()
        }
    }
}

/// Analyze left/right PCM data stored in separate planar buffers.
fn analyze_planar(
    left_bytes: &[u8],
    right_bytes: &[u8],
) -> Result<StereoAnalysis, StereoDecodeError> {
    let left = samples(left_bytes)?;
    let right = samples(right_bytes)?;

    let frame_count = left.len().min(right.len());
    if frame_count == 0 {
        return Ok(StereoAnalysis::default());
    }

    Ok(analyze_pairs(
        left.iter()
            .take(frame_count)
            .copied()
            .zip(right.iter().take(frame_count).copied()),
    ))
}

/// Analyze left/right PCM data stored in a single interleaved buffer.
/// Treat the first channel as left and the second as right.
fn analyze_interleaved(
    bytes: &[u8],
    channel_count: usize,
    bytes_per_frame: usize,
) -> Result<StereoAnalysis, StereoDecodeError> {
    if bytes.is_empty() {
        return Ok(StereoAnalysis::default());
    }

    if bytes.len() % bytes_per_frame != 0 {
        return Err(StereoDecodeError::UnsupportedAudioFormat);
    }

    let samples_per_frame = bytes_per_frame / size_of::<f32>();
    if samples_per_frame < channel_count || channel_count == 0 {
        return Err(StereoDecodeError::UnsupportedAudioFormat);
    }

    let samples = samples(bytes)?;

    let frame_count = bytes.len() / bytes_per_frame;
    if frame_count == 0 {
        return Ok(StereoAnalysis::default());
    }

    Ok(analyze_pairs((0..frame_count).map(|frame| {
        let base = frame * samples_per_frame;
        let left = samples[base];
        let right = samples[base + (1usize.min(channel_count.saturating_sub(1)))];
        (left, right)
    })))
}

fn samples(bytes: &[u8]) -> Result<&[f32], StereoDecodeError> {
    if bytes.len() % size_of::<f32>() != 0 {
        return Err(StereoDecodeError::UnsupportedAudioFormat);
    }

    let sample_count = bytes.len() / size_of::<f32>();
    Ok(unsafe { std::slice::from_raw_parts(bytes.as_ptr().cast::<f32>(), sample_count) })
}

fn analyze_pairs<I>(pairs: I) -> StereoAnalysis
where
    I: IntoIterator<Item = (f32, f32)>,
{
    let mut frame_count = 0usize;
    let mut left_sum = 0.0f32;
    let mut right_sum = 0.0f32;
    let mut mid_sum = 0.0f32;
    let mut side_sum = 0.0f32;

    for (left, right) in pairs {
        frame_count += 1;

        let sum = left + right;
        let diff = left - right;

        left_sum += left * left;
        right_sum += right * right;
        mid_sum += 0.25 * sum * sum;
        side_sum += 0.25 * diff * diff;
    }

    if frame_count == 0 {
        return StereoAnalysis::default();
    }

    let frames = frame_count as f32;

    let left_power = left_sum / frames;
    let right_power = right_sum / frames;
    let mid_power = mid_sum / frames;
    let side_power = side_sum / frames;

    let left_rms = left_power.sqrt();
    let right_rms = right_power.sqrt();
    let mid_rms = mid_power.sqrt();
    let side_rms = side_power.sqrt();
    // Convert a power ratio into decibels for stereo pan estimation.
    let pan_db = 10.0 * ((right_power + 1e-12) / (left_power + 1e-12)).log10();

    StereoAnalysis {
        left_rms,
        right_rms,
        pan: (pan_db / 18.0).clamp(-1.0, 1.0),
        width: (side_rms / (mid_rms + side_rms + 1e-6)).clamp(0.0, 1.0),
    }
}
