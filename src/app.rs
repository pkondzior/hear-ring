use std::time::{Duration, Instant};

use eframe::egui::{self, Vec2};

use crate::pipeline::ProcessingPipeline;
use crate::source::{AudioSource, AudioSourceState, DemoSource, ScreenCaptureSource};
use crate::types::{ChannelEnergies, ChannelLayout, DirectionFrame, ORDERED_SECTORS};
use crate::ui::ring::{draw_direction_ring, energy_bar};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceMode {
    Demo,
    SystemAudio,
}

impl SourceMode {
    fn label(self) -> &'static str {
        match self {
            SourceMode::Demo => "Demo",
            SourceMode::SystemAudio => "System Audio",
        }
    }
}

enum AppSource {
    Demo(DemoSource),
    SystemAudio(ScreenCaptureSource),
}

impl AppSource {
    fn demo(layout: ChannelLayout) -> Self {
        Self::Demo(DemoSource::new(layout))
    }

    fn system_audio() -> Self {
        Self::SystemAudio(ScreenCaptureSource::new())
    }

    fn mode(&self) -> SourceMode {
        match self {
            AppSource::Demo(_) => SourceMode::Demo,
            AppSource::SystemAudio(_) => SourceMode::SystemAudio,
        }
    }

    fn set_layout(&mut self, layout: ChannelLayout) {
        if let AppSource::Demo(source) = self {
            source.set_layout(layout);
        }
    }

    fn layout(&self) -> ChannelLayout {
        match self {
            AppSource::Demo(source) => source.layout(),
            AppSource::SystemAudio(source) => source.layout(),
        }
    }

    fn next_energies(&mut self, dt: f32) -> ChannelEnergies {
        match self {
            AppSource::Demo(source) => source.next_energies(dt),
            AppSource::SystemAudio(source) => source.next_energies(dt),
        }
    }

    fn state(&self) -> AudioSourceState {
        match self {
            AppSource::Demo(source) => source.state(),
            AppSource::SystemAudio(source) => source.state(),
        }
    }
}

pub struct SoundHearingAidApp {
    source: AppSource,
    pipeline: ProcessingPipeline,
    last_tick: Instant,
    latest_energies: ChannelEnergies,
    latest_frame: DirectionFrame,
}

impl SoundHearingAidApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let layout = ChannelLayout::Stereo;
        Self {
            source: AppSource::demo(layout),
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

    fn set_source_mode(&mut self, mode: SourceMode) {
        let current_layout = self.source.layout();

        self.source = match mode {
            SourceMode::Demo => AppSource::demo(current_layout),
            SourceMode::SystemAudio => AppSource::system_audio(),
        };

        self.pipeline.set_layout(self.source.layout());
        self.latest_energies = ChannelEnergies::default();
        self.latest_frame = DirectionFrame::empty();
    }

    fn source_state_message(state: &AudioSourceState) -> String {
        match state.detail() {
            Some(detail) => format!("{}: {}", state.label(), detail),
            None => state.label().to_owned(),
        }
    }

    fn source_help_text(mode: SourceMode, state: &AudioSourceState) -> &'static str {
        match (mode, state) {
            (SourceMode::Demo, _) => {
                "Demo source generates synthetic channel energy so the UI and estimator path can be tested."
            }
            (SourceMode::SystemAudio, AudioSourceState::Running) => {
                "System audio capture is active. Startup and source-state wiring are in place."
            }
            (SourceMode::SystemAudio, AudioSourceState::Starting) => {
                "Starting ScreenCaptureKit audio capture."
            }
            (SourceMode::SystemAudio, AudioSourceState::PermissionDenied) => {
                "Grant Screen Recording permission in Privacy & Security and restart the app."
            }
            (SourceMode::SystemAudio, AudioSourceState::UnsupportedPlatform) => {
                "System audio capture is only available on macOS."
            }
            (SourceMode::SystemAudio, AudioSourceState::Error(_)) => {
                "ScreenCaptureKit failed to start. Check permissions and try again."
            }
        }
    }
}

impl eframe::App for SoundHearingAidApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = (now - self.last_tick).as_secs_f32().max(1.0 / 240.0);
        self.last_tick = now;

        self.latest_energies = self.source.next_energies(dt);
        self.latest_frame = self.pipeline.update(&self.latest_energies);

        let source_state = self.source.state();
        let source_mode = self.source.mode();

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Sound Hearing Aid");
                ui.separator();

                ui.label("Source:");
                let mut selected_source = source_mode;
                ui.selectable_value(
                    &mut selected_source,
                    SourceMode::Demo,
                    SourceMode::Demo.label(),
                );
                ui.selectable_value(
                    &mut selected_source,
                    SourceMode::SystemAudio,
                    SourceMode::SystemAudio.label(),
                );
                if selected_source != source_mode {
                    self.set_source_mode(selected_source);
                }

                ui.separator();
                ui.label("Layout:");

                let mut current_layout = self.source.layout();
                let layout_locked = matches!(source_mode, SourceMode::SystemAudio);
                ui.add_enabled_ui(!layout_locked, |ui| {
                    ui.selectable_value(&mut current_layout, ChannelLayout::Stereo, "Stereo");
                    ui.selectable_value(&mut current_layout, ChannelLayout::Surround71, "7.1");
                });

                if current_layout != self.source.layout() {
                    self.set_layout(current_layout);
                }

                if layout_locked {
                    ui.label("(fixed by source)");
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
                    ui.heading("Source");
                    ui.label(format!("Selected source: {}", source_mode.label()));
                    ui.label(format!(
                        "Source state: {}",
                        Self::source_state_message(&source_state)
                    ));
                    ui.label(Self::source_help_text(source_mode, &source_state));

                    ui.add_space(16.0);
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

                    ui.add_space(16.0);
                    ui.heading("System audio");
                    ui.label("ScreenCaptureKit startup and source state are wired in.");
                    ui.label("PCM-to-energy decoding is not implemented yet.");
                });
            });
        });

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}
