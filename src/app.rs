use std::time::{Duration, Instant};

use eframe::egui::{self, Vec2};

use crate::pipeline::ProcessingPipeline;
use crate::source::{AudioSource, DemoSource};
use crate::types::{ChannelEnergies, ChannelLayout, DirectionFrame, ORDERED_SECTORS};
use crate::ui::ring::{draw_direction_ring, energy_bar};

pub struct SoundHearingAidApp {
    source: DemoSource,
    pipeline: ProcessingPipeline,
    last_tick: Instant,
    latest_energies: ChannelEnergies,
    latest_frame: DirectionFrame,
}

impl SoundHearingAidApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let layout = ChannelLayout::Stereo;
        Self {
            source: DemoSource::new(layout),
            pipeline: ProcessingPipeline::new(layout),
            last_tick: Instant::now(),
            latest_energies: ChannelEnergies::default(),
            latest_frame: DirectionFrame::empty(),
        }
    }

    fn set_layout(&mut self, layout: ChannelLayout) {
        self.source.set_layout(layout);
        self.pipeline.set_layout(layout);
    }
}

impl eframe::App for SoundHearingAidApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = (now - self.last_tick).as_secs_f32().max(1.0 / 240.0);
        self.last_tick = now;

        self.latest_energies = self.source.next_energies(dt);
        self.latest_frame = self.pipeline.update(&self.latest_energies);

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Sound Hearing Aid");
                ui.separator();
                ui.label("Layout:");

                let mut current = self.source.layout();
                ui.selectable_value(&mut current, ChannelLayout::Stereo, "Stereo");
                ui.selectable_value(&mut current, ChannelLayout::Surround71, "7.1");
                if current != self.source.layout() {
                    self.set_layout(current);
                }

                ui.separator();
                ui.label(format!(
                    "Dominant sector: {}",
                    self.latest_frame
                        .dominant_sector()
                        .map(|sector| sector.label())
                        .unwrap_or("-")
                ));
                ui.separator();
                ui.label(format!("Confidence: {:.2}", self.latest_frame.confidence));
                ui.separator();
                ui.label(format!("Intensity: {:.2}", self.latest_frame.intensity));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(2, |columns| {
                columns[0].vertical_centered(|ui| {
                    ui.heading("Unified 8-sector UI");
                    ui.label(
                        "Stereo only fills the part of the ring it can honestly support. 7.1 fills the full circle.",
                    );
                    ui.add_space(12.0);

                    let size = Vec2::new(520.0, 520.0);
                    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    draw_direction_ring(ui.painter(), rect, &self.latest_frame);
                });

                columns[1].vertical(|ui| {
                    ui.heading("Current channel energies");
                    ui.label(format!("Input mode: {}", self.source.layout().label()));
                    ui.add_space(8.0);

                    energy_bar(ui, "FL / L", self.latest_energies.fl.max(self.latest_energies.sl));
                    energy_bar(ui, "FR / R", self.latest_energies.fr.max(self.latest_energies.sr));
                    energy_bar(ui, "C", self.latest_energies.c);
                    energy_bar(ui, "SL", self.latest_energies.sl);
                    energy_bar(ui, "SR", self.latest_energies.sr);
                    energy_bar(ui, "RL", self.latest_energies.rl);
                    energy_bar(ui, "RR", self.latest_energies.rr);
                    energy_bar(ui, "LFE", self.latest_energies.lfe);

                    ui.add_space(16.0);
                    ui.heading("Sector scores");
                    for sector in ORDERED_SECTORS {
                        let value = self.latest_frame.scores[sector.index()];
                        energy_bar(ui, sector.label(), value);
                    }


                });
            });
        });

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}
