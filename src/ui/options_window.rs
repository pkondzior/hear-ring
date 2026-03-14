use gpui::{point, px, size, AnyWindowHandle, App, Global};

use crate::{platform::WindowExt, ui::options_view::OptionsView};

pub struct OptionsWindow {
    handle: AnyWindowHandle,
}

impl Global for OptionsWindow {}

impl OptionsWindow {
    pub fn register_global(cx: &mut App) {
        let handle = Self::setup_window(cx);
        cx.set_global(Self { handle });
    }

    fn setup_window(cx: &mut App) -> AnyWindowHandle {
        let titlebar = Some(gpui::TitlebarOptions {
            title: Some("Sound Radar - Options".into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(12.), px(12.))),
        });
        let bounds = gpui::Bounds::centered(None, size(px(480.), px(860.)), cx);
        let window_options = gpui::WindowOptions {
            titlebar,
            window_bounds: Some(gpui::WindowBounds::Windowed(bounds)),
            is_resizable: true,
            kind: gpui::WindowKind::PopUp,
            app_id: Some(crate::APP_IDENTIFIER.to_owned()),
            ..Default::default()
        };

        *cx.open_window(window_options, |window, cx| {
            window.setup_options_window();
            OptionsView::new(cx)
        })
        .expect("Failed to open options window")
    }

    pub fn handle(&self) -> AnyWindowHandle {
        self.handle
    }
}
