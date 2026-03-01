use eframe::egui;

struct ScaffoldApp;

impl eframe::App for ScaffoldApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(40.0);
                ui.heading("Sound Hearing Aid");
                ui.add_space(8.0);
                ui.label("Prototype scaffold");
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([720.0, 480.0])
            .with_min_inner_size([520.0, 360.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Sound Hearing Aid",
        options,
        Box::new(|_cc| Ok(Box::new(ScaffoldApp))),
    )
}
