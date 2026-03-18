use gpui::{px, size, AnyWindowHandle, App, Global, WindowBackgroundAppearance};

use crate::{platform::WindowExt, ui::overlay_view::OverlayView};

pub struct OverlayWindow {
    handle: AnyWindowHandle,
    visible: bool,
    click_through: bool,
    always_on_top: bool,
}

impl Global for OverlayWindow {}

impl OverlayWindow {
    const WINDOW_SIZE: f32 = 420.0 / 3.0;

    pub fn register_global(cx: &mut App) {
        let handle = Self::setup_window(cx);
        let overlay = Self {
            handle,
            visible: true,
            click_through: true,
            always_on_top: true,
        };
        overlay.apply_interaction_mode(cx);
        overlay.apply_window_level(cx);
        cx.set_global(overlay);
    }

    fn setup_window(cx: &mut App) -> AnyWindowHandle {
        let bounds =
            gpui::Bounds::centered(None, size(px(Self::WINDOW_SIZE), px(Self::WINDOW_SIZE)), cx);
        let window_options = gpui::WindowOptions {
            titlebar: None,
            kind: gpui::WindowKind::PopUp,
            app_id: Some(crate::APP_IDENTIFIER.to_owned()),
            window_background: WindowBackgroundAppearance::Transparent,
            window_bounds: Some(gpui::WindowBounds::Windowed(bounds)),
            focus: false,
            ..Default::default()
        };

        *cx.open_window(window_options, |window, cx| {
            window.setup_overlay_window();
            OverlayView::new(cx)
        })
        .expect("Failed to open overlay window")
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn click_through(&self) -> bool {
        self.click_through
    }

    pub fn always_on_top(&self) -> bool {
        self.always_on_top
    }

    pub fn set_visible(&mut self, cx: &mut App, visible: bool) {
        self.visible = visible;
        self.handle
            .update(cx, move |_, window, _| window.set_hidden(!visible))
            .ok();
    }

    pub fn set_click_through(&mut self, cx: &mut App, click_through: bool) {
        self.click_through = click_through;
        self.apply_interaction_mode(cx);
    }

    pub fn set_always_on_top(&mut self, cx: &mut App, always_on_top: bool) {
        self.always_on_top = always_on_top;
        self.apply_window_level(cx);
    }

    fn apply_interaction_mode(&self, cx: &mut App) {
        self.handle
            .update(cx, |_, window, _| {
                window.set_ignore_cursor_events(self.click_through);
                window.set_window_draggable(!self.click_through);
            })
            .ok();
    }

    fn apply_window_level(&self, cx: &mut App) {
        self.handle
            .update(cx, |_, window, _| {
                window.set_window_topmost(self.always_on_top);
            })
            .ok();
    }
}
