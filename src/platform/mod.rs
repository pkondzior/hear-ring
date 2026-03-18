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

#[cfg(not(target_os = "macos"))]
impl WindowExt for gpui::Window {}
