mod estimators;
mod pipeline;
mod platform;
mod runtime;
mod smoothing;
mod source;
mod types;
mod ui;

use gpui::{actions, App, KeyBinding, Menu, MenuItem, SystemMenuType};

use crate::{
    runtime::RadarRuntime,
    ui::{options_window::OptionsWindow, overlay_window::OverlayWindow},
};

const APP_IDENTIFIER: &str = "com.pk.sound-radar";

actions!(app, [OpenPreferences, Quit]);

fn setup(cx: &mut App) {
    gpui_component::init(cx);
    RadarRuntime::register_global(cx);
    OverlayWindow::register_global(cx);
    OptionsWindow::register_global(cx);

    cx.on_action(|_: &OpenPreferences, cx| OptionsWindow::show(cx));
    cx.on_action(|_: &Quit, cx| cx.quit());

    cx.bind_keys([
        KeyBinding::new("cmd-,", OpenPreferences, None),
        KeyBinding::new("cmd-q", Quit, None),
    ]);

    cx.set_menus(vec![Menu {
        name: "Sound Radar".into(),
        items: vec![
            MenuItem::action("Preferences…", OpenPreferences),
            MenuItem::separator(),
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Quit", Quit),
        ],
    }]);

    cx.activate(true);
}

fn main() {
    let app = gpui::Application::new();
    app.on_reopen(|cx| OptionsWindow::show(cx));
    app.run(setup);
}
