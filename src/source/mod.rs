mod demo;
#[cfg(target_os = "macos")]
mod screencapture;

pub use demo::DemoSource;
#[cfg(target_os = "macos")]
pub use screencapture::ScreenCaptureSource;
#[cfg(not(target_os = "macos"))]
pub use screencapture_fallback::ScreenCaptureSource;

use crate::types::{ChannelEnergies, ChannelLayout};

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
    Error(String),
}

impl AudioSourceState {
    pub fn label(&self) -> &str {
        match self {
            AudioSourceState::Running => "Running",
            AudioSourceState::Starting => "Starting",
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

#[cfg(not(target_os = "macos"))]
mod screencapture_fallback {
    use super::{AudioSource, AudioSourceState};
    use crate::types::{ChannelEnergies, ChannelLayout};

    pub struct ScreenCaptureSource;

    impl ScreenCaptureSource {
        pub fn new() -> Self {
            Self
        }
    }

    impl AudioSource for ScreenCaptureSource {
        fn layout(&self) -> ChannelLayout {
            ChannelLayout::Stereo
        }

        fn next_energies(&mut self, _dt: f32) -> ChannelEnergies {
            ChannelEnergies::default()
        }

        fn state(&self) -> AudioSourceState {
            AudioSourceState::Error("System audio capture is only available on macOS".to_owned())
        }
    }
}
