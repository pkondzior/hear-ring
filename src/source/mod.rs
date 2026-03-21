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
    PermissionDenied,
    #[allow(dead_code)]
    UnsupportedPlatform,
    Error(String),
}

impl AudioSourceState {
    pub fn is_capture_permission_denied(message: &str) -> bool {
        let lower = message.to_lowercase();

        // ScreenCaptureKit permission errors vary across macOS versions and call sites, so
        // conservatively match the common permission/TCC variants we have observed.
        lower.contains("screen recording")
            || lower.contains("not authorized")
            || lower.contains("not authorised")
            || (lower.contains("permission") && lower.contains("denied"))
            || (lower.contains("access") && lower.contains("denied"))
            || (lower.contains("tcc") && (lower.contains("declined") || lower.contains("denied")))
    }

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

    pub fn from_capture_error(message: impl AsRef<str>) -> Self {
        let message = message.as_ref();

        if Self::is_capture_permission_denied(message) {
            Self::PermissionDenied
        } else {
            Self::Error(message.to_owned())
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
            AudioSourceState::UnsupportedPlatform
        }
    }
}
