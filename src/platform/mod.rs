pub trait WindowExt {
    fn setup_options_window(&self) {}

    fn setup_overlay_window(&self) {}

    fn set_hidden(&self, _hidden: bool) {}

    fn set_ignore_cursor_events(&self, _ignore: bool) {}

    fn set_window_draggable(&self, _draggable: bool) {}

    fn set_window_topmost(&self, _topmost: bool) {}
}

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub fn open_system_audio_preferences() -> std::io::Result<()> {
    macos::open_system_audio_preferences()
}

#[cfg(not(target_os = "macos"))]
pub fn open_system_audio_preferences() -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "System audio preferences are only available on macOS",
    ))
}

#[cfg(not(target_os = "macos"))]
impl WindowExt for gpui::Window {}
