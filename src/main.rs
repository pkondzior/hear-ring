mod estimators;
mod pipeline;
mod smoothing;
mod source;
mod types;
mod ui;

use eframe::egui::{self, Vec2};

use crate::types::DirectionFrame;
use crate::ui::ring::{draw_direction_ring, energy_bar};

struct ScaffoldApp {
    frame: DirectionFrame,
}

impl Default for ScaffoldApp {
    fn default() -> Self {
        let mut frame = DirectionFrame::empty();
        frame.scores = [0.15, 0.35, 0.10, 0.05, 0.0, 0.0, 0.20, 0.70];
        frame.confidence = 0.82;
        frame.intensity = 0.58;
        frame.active = true;

        Self { frame }
    }
}

impl eframe::App for ScaffoldApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Sound Hearing Aid");
                ui.separator();
                ui.label("Prototype scaffold");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(2, |columns| {
                columns[0].vertical_centered(|ui| {
                    ui.heading("Direction ring");
                    ui.label("Static preview of the ring UI wired into the scaffold.");
                    ui.add_space(12.0);

                    let size = Vec2::new(420.0, 420.0);
                    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    draw_direction_ring(ui.painter(), rect, &self.frame);
                });

                columns[1].vertical(|ui| {
                    ui.heading("Preview values");
                    ui.add_space(8.0);

                    energy_bar(ui, "FL", self.frame.scores[7]);
                    energy_bar(ui, "F", self.frame.scores[0]);
                    energy_bar(ui, "FR", self.frame.scores[1]);
                    energy_bar(ui, "L", self.frame.scores[6]);
                    energy_bar(ui, "R", self.frame.scores[2]);
                    energy_bar(ui, "BL", self.frame.scores[5]);
                    energy_bar(ui, "B", self.frame.scores[4]);
                    energy_bar(ui, "BR", self.frame.scores[3]);

                    ui.add_space(16.0);
                    ui.label(format!("Confidence: {:.2}", self.frame.confidence));
                    ui.label(format!("Intensity: {:.2}", self.frame.intensity));
                    ui.label(format!(
                        "Dominant sector: {}",
                        self.frame
                            .dominant_sector()
                            .map(|sector| sector.label())
                            .unwrap_or("-")
                    ));
                });
            });
        });
    }
}

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
        Box::new(|_cc| Ok(Box::new(ScaffoldApp::default()))),
    )
}
