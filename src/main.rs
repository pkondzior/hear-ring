mod app;
mod estimators;
mod pipeline;
mod smoothing;
mod source;
mod types;
mod ui;

use eframe::egui;

use crate::app::SoundHearingAidApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 640.0])
            .with_min_inner_size([720.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Sound Hearing Aid",
        options,
        Box::new(|cc| Ok(Box::new(SoundHearingAidApp::new(cc)))),
    )
}
