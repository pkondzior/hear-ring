use std::f32::consts::{PI, TAU};
#[cfg(target_os = "macos")]
use std::sync::{Arc, Mutex};

#[cfg(target_os = "macos")]
use screencapturekit::prelude::*;

use crate::types::{ChannelEnergies, ChannelLayout, Sector8};

pub trait AudioSource {
    fn layout(&self) -> ChannelLayout;
    fn next_energies(&mut self, dt: f32) -> ChannelEnergies;
    fn state(&self) -> AudioSourceState {
        AudioSourceState::Running
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioSourceState {
    Running,
    Starting,
    PermissionDenied,
    UnsupportedPlatform,
    Error(String),
}

impl AudioSourceState {
    pub fn label(&self) -> &str {
        match self {
            AudioSourceState::Running => "Running",
            AudioSourceState::Starting => "Starting",
            AudioSourceState::PermissionDenied => "Permission denied",
            AudioSourceState::UnsupportedPlatform => "Unsupported platform",
            AudioSourceState::Error(_) => "Error",
        }
    }

    pub fn detail(&self) -> Option<&str> {
        match self {
            AudioSourceState::Error(message) => Some(message.as_str()),
            _ => None,
        }
    }
}

/// Demo source that sweeps energy around the ring so the UI and estimator logic can be tested
/// before you wire real loopback capture into the same pipeline.
pub struct DemoSource {
    layout: ChannelLayout,
    phase: f32,
    pulse: f32,
}

impl DemoSource {
    pub fn new(layout: ChannelLayout) -> Self {
        Self {
            layout,
            phase: 0.0,
            pulse: 0.0,
        }
    }

    pub fn set_layout(&mut self, layout: ChannelLayout) {
        self.layout = layout;
    }

    fn directional_blob(&self, angle: f32, center: f32, width: f32) -> f32 {
        let delta = wrapped_angle(angle - center).abs();
        (1.0 - delta / width).clamp(0.0, 1.0)
    }
}

impl AudioSource for DemoSource {
    fn layout(&self) -> ChannelLayout {
        self.layout
    }

    fn next_energies(&mut self, dt: f32) -> ChannelEnergies {
        self.phase = (self.phase + dt * 0.60) % TAU;
        self.pulse = (self.pulse + dt * 1.7) % TAU;

        let source_angle = -PI * 0.5 + self.phase;
        let loudness = 0.12 + 0.88 * (0.5 + 0.5 * self.pulse.sin());

        let mut energies = ChannelEnergies::default();

        match self.layout {
            ChannelLayout::Stereo => {
                let left = self.directional_blob(source_angle, Sector8::FL.angle(), PI * 0.80)
                    + self.directional_blob(source_angle, Sector8::L.angle(), PI * 0.65) * 0.35;
                let right = self.directional_blob(source_angle, Sector8::FR.angle(), PI * 0.80)
                    + self.directional_blob(source_angle, Sector8::R.angle(), PI * 0.65) * 0.35;

                energies.fl = loudness * left;
                energies.fr = loudness * right;
            }
            ChannelLayout::Surround71 => {
                energies.fl =
                    loudness * self.directional_blob(source_angle, Sector8::FL.angle(), PI * 0.55);
                energies.fr =
                    loudness * self.directional_blob(source_angle, Sector8::FR.angle(), PI * 0.55);
                energies.c =
                    loudness * self.directional_blob(source_angle, Sector8::F.angle(), PI * 0.55);
                energies.sl =
                    loudness * self.directional_blob(source_angle, Sector8::L.angle(), PI * 0.55);
                energies.sr =
                    loudness * self.directional_blob(source_angle, Sector8::R.angle(), PI * 0.55);
                energies.rl =
                    loudness * self.directional_blob(source_angle, Sector8::BL.angle(), PI * 0.55);
                energies.rr =
                    loudness * self.directional_blob(source_angle, Sector8::BR.angle(), PI * 0.55);
                energies.lfe = loudness * 0.18;
            }
        }

        energies
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone)]
struct SharedCaptureState {
    energies: ChannelEnergies,
    state: AudioSourceState,
}

#[cfg(target_os = "macos")]
impl Default for SharedCaptureState {
    fn default() -> Self {
        Self {
            energies: ChannelEnergies::default(),
            state: AudioSourceState::Starting,
        }
    }
}

#[cfg(target_os = "macos")]
pub struct ScreenCaptureSource {
    shared: Arc<Mutex<SharedCaptureState>>,
    _stream: Option<SCStream>,
}

#[cfg(target_os = "macos")]
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

        let display = content
            .displays()
            .into_iter()
            .next()
            .ok_or_else(|| AudioSourceState::Error("No display available for capture".to_owned()))?;

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
        let lower = message.to_ascii_lowercase();

        if lower.contains("permission")
            || lower.contains("screen recording")
            || lower.contains("not authorized")
            || lower.contains("denied")
        {
            AudioSourceState::PermissionDenied
        } else {
            AudioSourceState::Error(message)
        }
    }
}

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
struct AudioCaptureHandler {
    shared: Arc<Mutex<SharedCaptureState>>,
}

#[cfg(target_os = "macos")]
impl SCStreamOutputTrait for AudioCaptureHandler {
    fn did_output_sample_buffer(&self, _sample: CMSampleBuffer, _type: SCStreamOutputType) {
        // Placeholder implementation for initial integration.
        //
        // The next step is to read PCM data from `CMSampleBuffer`, compute per-channel RMS,
        // and store that in `SharedCaptureState.energies`.
        //
        // For now we keep the stream alive and expose state wiring through the app.
        if let Ok(mut guard) = self.shared.lock() {
            guard.state = AudioSourceState::Running;
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub struct ScreenCaptureSource;

#[cfg(not(target_os = "macos"))]
impl ScreenCaptureSource {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(target_os = "macos"))]
impl AudioSource for ScreenCaptureSource {
    fn layout(&self) -> ChannelLayout {
        ChannelLayout::Stereo
    }

    fn next_energies(&mut self, _dt: f32) -> ChannelEnergies {
        ChannelEnergies::default()
    }

    fn state(&self) -> AudioSourceState {
        AudioSourceState::UnsupportedPlatform
    }
}

fn wrapped_angle(mut angle: f32) -> f32 {
    while angle > PI {
        angle -= TAU;
    }
    while angle < -PI {
        angle += TAU;
    }
    angle
}
