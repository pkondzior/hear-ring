mod estimators;
mod pipeline;
mod platform;
mod runtime;
mod smoothing;
mod source;
mod types;
mod ui;

use gpui::{App, ReadGlobal};

use crate::{
    runtime::RadarRuntime,
    ui::{options_window::OptionsWindow, overlay_window::OverlayWindow},
};

const APP_IDENTIFIER: &str = "com.pk.sound-radar";

fn setup(cx: &mut App) {
    gpui_component::init(cx);
    RadarRuntime::register_global(cx);
    OverlayWindow::register_global(cx);
    OptionsWindow::register_global(cx);

    cx.on_window_closed(move |cx| {
        let no_options_window = cx
            .windows()
            .iter()
            .all(|handle| handle.window_id() != OptionsWindow::global(cx).handle().window_id());

        if no_options_window {
            cx.quit();
        }
    })
    .detach();

    cx.activate(true);
}

fn main() {
    gpui::Application::new().run(setup);
}
