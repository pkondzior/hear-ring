use std::fmt;
use std::mem::size_of;
use std::sync::{Arc, Mutex};

use screencapturekit::prelude::*;
use screencapturekit::CMFormatDescription;

use super::{AudioSource, AudioSourceState};
use crate::types::{ChannelEnergies, ChannelLayout};

#[derive(Debug, Clone, Copy, Default)]
struct StereoAnalysis {
    left_rms: f32,
    right_rms: f32,
    pan: f32,
    width: f32,
}

#[derive(Debug, Clone, Copy)]
struct PcmFormat {
    channel_count: usize,
    bytes_per_frame: usize,
}

#[derive(Debug)]
enum StereoDecodeError {
    InvalidSampleBuffer,
    SampleNotReady(i32),
    MissingFormatDescription,
    UnsupportedAudioFormat,
}

impl fmt::Display for StereoDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StereoDecodeError::InvalidSampleBuffer => write!(f, "Audio sample buffer is invalid"),
            StereoDecodeError::SampleNotReady(status) => {
                write!(f, "Audio sample not ready (status {status})")
            }
            StereoDecodeError::MissingFormatDescription => {
                write!(f, "Audio sample missing format description")
            }
            StereoDecodeError::UnsupportedAudioFormat => {
                write!(f, "Audio format is not supported PCM")
            }
        }
    }
}

trait SampleBytesExt {
    fn try_samples(&self) -> Result<&[f32], StereoDecodeError>;
}

impl SampleBytesExt for [u8] {
    fn try_samples(&self) -> Result<&[f32], StereoDecodeError> {
        bytemuck::try_cast_slice(self).map_err(|_| StereoDecodeError::UnsupportedAudioFormat)
    }
}

#[derive(Clone)]
struct InnerCaptureState {
    energies: ChannelEnergies,
    state: AudioSourceState,
}

impl Default for InnerCaptureState {
    fn default() -> Self {
        Self {
            energies: ChannelEnergies::default(),
            state: AudioSourceState::Starting,
        }
    }
}

#[derive(Clone)]
struct CaptureState(Arc<Mutex<InnerCaptureState>>);

impl CaptureState {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(InnerCaptureState::default())))
    }

    fn update(&self, update: impl FnOnce(&mut InnerCaptureState)) {
        if let Ok(mut state) = self.0.lock() {
            update(&mut state);
        }
    }

    fn energies(&self) -> ChannelEnergies {
        self.0
            .lock()
            .map(|state| state.energies)
            .unwrap_or_default()
    }

    fn source_state(&self) -> AudioSourceState {
        self.0
            .lock()
            .map(|state| state.state.clone())
            .unwrap_or_else(|_| AudioSourceState::Error("Capture state lock poisoned".to_owned()))
    }
}

pub struct ScreenCaptureSource {
    shared: CaptureState,
    _stream: Option<SCStream>,
}

struct StreamErrorHandler {
    shared: CaptureState,
}

struct NoopScreenHandler;

impl ScreenCaptureSource {
    pub fn new() -> Self {
        let shared = CaptureState::new();

        let stream = match Self::start_capture(shared.clone()) {
            Ok(stream) => {
                shared.update(|state| {
                    state.state = AudioSourceState::Running;
                });
                Some(stream)
            }
            Err(state) => {
                shared.update(|shared_state| {
                    shared_state.state = state;
                });
                None
            }
        };

        Self {
            shared,
            _stream: stream,
        }
    }

    fn start_capture(shared: CaptureState) -> Result<SCStream, AudioSourceState> {
        let content = SCShareableContent::get().map_err(Self::map_error)?;

        let display = content.displays().into_iter().next().ok_or_else(|| {
            AudioSourceState::Error("No display available for capture".to_owned())
        })?;

        let filter = SCContentFilter::create()
            .with_display(&display)
            .with_excluding_windows(&[])
            .build();

        let config = SCStreamConfiguration::new()
            .with_width(1)
            .with_height(1)
            .with_pixel_format(PixelFormat::BGRA)
            .with_fps(1)
            .with_captures_audio(true)
            .with_excludes_current_process_audio(true)
            .with_sample_rate(48_000)
            .with_channel_count(2);

        let mut stream = SCStream::new_with_delegate(
            &filter,
            &config,
            StreamErrorHandler {
                shared: shared.clone(),
            },
        );
        let handler = AudioCaptureHandler { shared };

        stream.add_output_handler(NoopScreenHandler, SCStreamOutputType::Screen);
        stream.add_output_handler(handler, SCStreamOutputType::Audio);
        stream.start_capture().map_err(Self::map_error)?;

        Ok(stream)
    }

    fn map_error(error: impl std::fmt::Display) -> AudioSourceState {
        AudioSourceState::from_capture_error(error.to_string())
    }
}

impl AudioSource for ScreenCaptureSource {
    fn layout(&self) -> ChannelLayout {
        ChannelLayout::Stereo
    }

    fn next_energies(&mut self, _dt: f32) -> ChannelEnergies {
        self.shared.energies()
    }

    fn state(&self) -> AudioSourceState {
        self.shared.source_state()
    }
}

impl SCStreamDelegateTrait for StreamErrorHandler {
    fn did_stop_with_error(&self, error: SCError) {
        let message = error.to_string();

        self.shared.update(|state| {
            state.state = if AudioSourceState::is_capture_permission_denied(&message) {
                AudioSourceState::PermissionDenied
            } else {
                AudioSourceState::Error(format!("ScreenCaptureKit stream error: {message}"))
            };
        });
    }

    fn stream_did_stop(&self, error: Option<String>) {
        self.shared.update(|state| {
            state.state = AudioSourceState::from_capture_error(
                error.unwrap_or_else(|| "ScreenCaptureKit stream stopped".to_owned()),
            );
        });
    }
}

impl SCStreamOutputTrait for NoopScreenHandler {
    fn did_output_sample_buffer(&self, _sample: CMSampleBuffer, _type: SCStreamOutputType) {}
}

struct AudioCaptureHandler {
    shared: CaptureState,
}

impl SCStreamOutputTrait for AudioCaptureHandler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, _type: SCStreamOutputType) {
        match StereoAnalysis::try_from(&sample) {
            Ok(analysis) => {
                self.shared.update(|state| {
                    state.energies = analysis.into();
                    state.state = AudioSourceState::Running;
                });
            }
            Err(error) => {
                self.shared.update(|state| {
                    state.state = AudioSourceState::Error(error.to_string());
                });
            }
        }
    }
}

impl TryFrom<&CMFormatDescription> for PcmFormat {
    type Error = StereoDecodeError;

    fn try_from(format: &CMFormatDescription) -> Result<Self, Self::Error> {
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

        Ok(Self {
            channel_count,
            bytes_per_frame,
        })
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
        let pcm = PcmFormat::try_from(&format)?;

        let buffers = sample
            .audio_buffer_list()
            .ok_or(StereoDecodeError::UnsupportedAudioFormat)?;

        let analysis = match buffers.num_buffers() {
            1 => analyze_interleaved(
                buffers
                    .get(0)
                    .ok_or(StereoDecodeError::UnsupportedAudioFormat)?
                    .data(),
                pcm.channel_count,
                pcm.bytes_per_frame,
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
    let left = left_bytes.try_samples()?;
    let right = right_bytes.try_samples()?;

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
    if samples_per_frame < channel_count {
        return Err(StereoDecodeError::UnsupportedAudioFormat);
    }

    let samples = bytes.try_samples()?;

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
