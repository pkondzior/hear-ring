use std::mem::size_of;
use std::sync::{Arc, Mutex};

use screencapturekit::prelude::*;

use super::{AudioSource, AudioSourceState};
use crate::types::{ChannelEnergies, ChannelLayout};

struct StereoLevels {
    left: f32,
    right: f32,
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
        AudioSourceState::Error(error.to_string())
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
        match StereoLevels::try_from(&sample) {
            Ok(levels) => {
                if let Ok(mut guard) = self.shared.lock() {
                    guard.energies = levels.into();
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

impl TryFrom<&CMSampleBuffer> for StereoLevels {
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

        let (left, right) = match buffers.num_buffers() {
            1 => rms_interleaved(
                buffers
                    .get(0)
                    .ok_or(StereoDecodeError::UnsupportedAudioFormat)?
                    .data(),
                channel_count,
                bytes_per_frame,
            )?,
            count if count >= 2 => (
                rms_planar(
                    buffers
                        .get(0)
                        .ok_or(StereoDecodeError::UnsupportedAudioFormat)?
                        .data(),
                )?,
                rms_planar(
                    buffers
                        .get(1)
                        .ok_or(StereoDecodeError::UnsupportedAudioFormat)?
                        .data(),
                )?,
            ),
            _ => return Err(StereoDecodeError::UnsupportedAudioFormat),
        };

        Ok(Self { left, right })
    }
}

impl From<StereoLevels> for ChannelEnergies {
    fn from(levels: StereoLevels) -> Self {
        Self {
            fl: levels.left,
            fr: levels.right,
            ..Self::default()
        }
    }
}

/// Compute root mean square for a single planar PCM buffer.
/// Each buffer contains samples for one channel only.
fn rms_planar(bytes: &[u8]) -> Result<f32, StereoDecodeError> {
    if bytes.is_empty() {
        return Ok(0.0);
    }

    if bytes.len() % size_of::<f32>() != 0 {
        return Err(StereoDecodeError::UnsupportedAudioFormat);
    }

    let sample_count = bytes.len() / size_of::<f32>();
    if sample_count == 0 {
        return Ok(0.0);
    }

    let samples = unsafe { std::slice::from_raw_parts(bytes.as_ptr().cast::<f32>(), sample_count) };

    Ok(rms(samples))
}

/// Compute left/right root mean square for a single interleaved PCM buffer.
/// Treat the first channel as left and the second as right.
fn rms_interleaved(
    bytes: &[u8],
    channel_count: usize,
    bytes_per_frame: usize,
) -> Result<(f32, f32), StereoDecodeError> {
    if bytes.is_empty() {
        return Ok((0.0, 0.0));
    }

    if bytes.len() % bytes_per_frame != 0 {
        return Err(StereoDecodeError::UnsupportedAudioFormat);
    }

    let samples_per_frame = bytes_per_frame / size_of::<f32>();
    if samples_per_frame < channel_count || channel_count == 0 {
        return Err(StereoDecodeError::UnsupportedAudioFormat);
    }

    let sample_count = bytes.len() / size_of::<f32>();
    let samples = unsafe { std::slice::from_raw_parts(bytes.as_ptr().cast::<f32>(), sample_count) };

    let frame_count = bytes.len() / bytes_per_frame;
    if frame_count == 0 {
        return Ok((0.0, 0.0));
    }

    let mut left_sum = 0.0f32;
    let mut right_sum = 0.0f32;

    for frame in 0..frame_count {
        let base = frame * samples_per_frame;
        let left = samples[base];
        let right = samples[base + (1usize.min(channel_count.saturating_sub(1)))];

        left_sum += left * left;
        right_sum += right * right;
    }

    Ok((
        (left_sum / frame_count as f32).sqrt(),
        (right_sum / frame_count as f32).sqrt(),
    ))
}

/// Compute root mean square for a slice of PCM samples.
/// This gives the UI pipeline a stable per-buffer energy value.
fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let energy = samples.iter().map(|sample| sample * sample).sum::<f32>();
    (energy / samples.len() as f32).sqrt()
}
