use std::time::{Duration, Instant};

use eframe::egui::{self, Slider, Vec2};
#[cfg(target_os = "macos")]
use objc2::MainThreadMarker;
#[cfg(target_os = "macos")]
use objc2_app_kit::{
    NSNormalWindowLevel, NSPopUpMenuWindowLevel, NSView, NSWindow, NSWindowAnimationBehavior,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
#[cfg(target_os = "macos")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::pipeline::{PipelineTuning, ProcessingPipeline};
use crate::source::{AudioSource, AudioSourceState, DemoSource, ScreenCaptureSource};
use crate::types::{ChannelEnergies, ChannelLayout, DirectionFrame, ORDERED_SECTORS};
use crate::ui::ring::{draw_direction_ring, energy_bar};

#[cfg(target_os = "macos")]
fn with_ns_window<T>(frame: &eframe::Frame, f: impl FnOnce(&NSWindow) -> T) -> Result<T, String> {
    let window_handle = frame
        .window_handle()
        .map_err(|err| format!("window handle unavailable: {err}"))?;

    let RawWindowHandle::AppKit(handle) = window_handle.as_raw() else {
        return Err("not an AppKit window".to_owned());
    };

    // SAFETY: The pointer comes from the live eframe window handle and is used
    // immediately on the UI thread to access the current window.
    let ns_view: &NSView = unsafe { &*handle.ns_view.as_ptr().cast::<NSView>() };
    let ns_window = ns_view
        .window()
        .ok_or_else(|| "NSView is not attached to an NSWindow yet".to_owned())?;

    Ok(f(&ns_window))
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowProfile {
    Normal,
    Overlay,
}

#[cfg(target_os = "macos")]
fn current_window_diagnostics(
    frame: &eframe::Frame,
) -> Result<
    (
        bool,
        i64,
        NSWindowAnimationBehavior,
        NSWindowCollectionBehavior,
        NSWindowStyleMask,
        bool,
        bool,
        bool,
        bool,
        bool,
    ),
    String,
> {
    with_ns_window(frame, |ns_window| {
        (
            MainThreadMarker::new().is_some(),
            ns_window.level() as i64,
            ns_window.animationBehavior(),
            ns_window.collectionBehavior(),
            ns_window.styleMask(),
            ns_window.hidesOnDeactivate(),
            ns_window.canBecomeKeyWindow(),
            ns_window.canBecomeMainWindow(),
            ns_window.ignoresMouseEvents(),
            ns_window.hasShadow(),
        )
    })
}

#[cfg(target_os = "macos")]
fn apply_window_profile(frame: &eframe::Frame, profile: WindowProfile) -> Result<(), String> {
    with_ns_window(frame, |ns_window| match profile {
        WindowProfile::Normal => {
            ns_window.setLevel(NSNormalWindowLevel);
            ns_window.setAnimationBehavior(NSWindowAnimationBehavior::Default);
            ns_window.setHidesOnDeactivate(false);
            ns_window.setHasShadow(true);
            ns_window.setCollectionBehavior(NSWindowCollectionBehavior::Default);
            ns_window.setStyleMask(
                NSWindowStyleMask::Titled
                    | NSWindowStyleMask::Closable
                    | NSWindowStyleMask::Miniaturizable
                    | NSWindowStyleMask::Resizable,
            );
        }
        WindowProfile::Overlay => {
            ns_window.setLevel(NSPopUpMenuWindowLevel);
            ns_window.setAnimationBehavior(NSWindowAnimationBehavior::UtilityWindow);
            ns_window.setHidesOnDeactivate(false);
            ns_window.setHasShadow(false);
            ns_window.setCollectionBehavior(
                NSWindowCollectionBehavior::CanJoinAllApplications
                    | NSWindowCollectionBehavior::CanJoinAllSpaces
                    | NSWindowCollectionBehavior::FullScreenAuxiliary
                    | NSWindowCollectionBehavior::IgnoresCycle,
            );
            ns_window.setStyleMask(
                NSWindowStyleMask::Borderless
                    | NSWindowStyleMask::NonactivatingPanel
                    | NSWindowStyleMask::FullSizeContentView,
            );
            ns_window.orderFrontRegardless();
        }
    })?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn set_mouse_passthrough(frame: &eframe::Frame, enabled: bool) -> Result<(), String> {
    with_ns_window(frame, |ns_window| {
        ns_window.setIgnoresMouseEvents(enabled);
    })?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn format_window_diagnostics(
    on_main_thread: bool,
    level: i64,
    animation: NSWindowAnimationBehavior,
    behavior: NSWindowCollectionBehavior,
    style_mask: NSWindowStyleMask,
    hides_on_deactivate: bool,
    can_become_key: bool,
    can_become_main: bool,
    ignores_mouse_events: bool,
    has_shadow: bool,
) -> String {
    format!(
        "main_thread={on_main_thread}, level={level}, animation={animation:?}, aux={}, all_spaces={}, all_apps={}, ignores_cycle={}, nonactivating={}, borderless={}, shadow={has_shadow}, hides_on_deactivate={hides_on_deactivate}, can_key={can_become_key}, can_main={can_become_main}, click_through={ignores_mouse_events}, behavior_raw=0x{:x}, style_raw=0x{:x}, flags={behavior:?}",
        behavior.contains(NSWindowCollectionBehavior::FullScreenAuxiliary),
        behavior.contains(NSWindowCollectionBehavior::CanJoinAllSpaces),
        behavior.contains(NSWindowCollectionBehavior::CanJoinAllApplications),
        behavior.contains(NSWindowCollectionBehavior::IgnoresCycle),
        style_mask.contains(NSWindowStyleMask::NonactivatingPanel),
        !style_mask.contains(NSWindowStyleMask::Titled),
        behavior.bits()
        ,
        style_mask.bits()
    )
}

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
    overlay_mode: bool,
    click_through_mode: bool,
    window_collection_behavior_debug: String,
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
            overlay_mode: false,
            click_through_mode: false,
            window_collection_behavior_debug: "not queried yet".to_owned(),
        }
    }

    fn tuning(&self) -> PipelineTuning {
        self.pipeline.tuning()
    }

    fn set_tuning(&mut self, tuning: PipelineTuning) {
        self.pipeline.set_tuning(tuning);
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

    fn set_overlay_mode(&mut self, ctx: &egui::Context, frame: &eframe::Frame, enabled: bool) {
        self.overlay_mode = enabled;
        if !enabled {
            self.click_through_mode = false;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(!enabled));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(if enabled {
            egui::viewport::WindowLevel::AlwaysOnTop
        } else {
            egui::viewport::WindowLevel::Normal
        }));
        #[cfg(target_os = "macos")]
        if let Err(err) = apply_window_profile(
            frame,
            if enabled {
                WindowProfile::Overlay
            } else {
                WindowProfile::Normal
            },
        ) {
            eprintln!("failed to apply window profile: {err}");
        }
        #[cfg(target_os = "macos")]
        if let Err(err) = set_mouse_passthrough(frame, enabled && self.click_through_mode) {
            eprintln!("failed to update click-through mode: {err}");
        }
        self.refresh_window_debug_info(frame);
    }

    fn set_click_through_mode(&mut self, frame: &eframe::Frame, enabled: bool) {
        self.click_through_mode = enabled;
        #[cfg(target_os = "macos")]
        if let Err(err) = set_mouse_passthrough(frame, self.overlay_mode && enabled) {
            eprintln!("failed to update click-through mode: {err}");
        }
        self.refresh_window_debug_info(frame);
    }

    fn refresh_window_debug_info(&mut self, frame: &eframe::Frame) {
        #[cfg(target_os = "macos")]
        {
            self.window_collection_behavior_debug = match current_window_diagnostics(frame) {
                Ok((
                    on_main_thread,
                    level,
                    animation,
                    behavior,
                    style_mask,
                    hides_on_deactivate,
                    can_become_key,
                    can_become_main,
                    ignores_mouse_events,
                    has_shadow,
                )) => format_window_diagnostics(
                    on_main_thread,
                    level,
                    animation,
                    behavior,
                    style_mask,
                    hides_on_deactivate,
                    can_become_key,
                    can_become_main,
                    ignores_mouse_events,
                    has_shadow,
                ),
                Err(err) => format!("unavailable: {err}"),
            };
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = frame;
            self.window_collection_behavior_debug = "not available on this platform".to_owned();
        }
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
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = (now - self.last_tick).as_secs_f32().max(1.0 / 240.0);
        self.last_tick = now;

        if self.click_through_mode && ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.set_click_through_mode(frame, false);
        }

        self.latest_energies = self.source.next_energies(dt);
        self.latest_frame = self.pipeline.update(&self.latest_energies);
        self.refresh_window_debug_info(frame);

        let source_state = self.source.state();
        let source_mode = self.source.mode();
        let mut tuning = self.tuning();

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Sound Hearing Aid");
                ui.separator();

                let overlay_label = if self.overlay_mode {
                    "Exit overlay"
                } else {
                    "Overlay"
                };
                if ui.button(overlay_label).clicked() {
                    self.set_overlay_mode(ctx, frame, !self.overlay_mode);
                }
                if self.overlay_mode {
                    let click_through_label = if self.click_through_mode {
                        "Interactive"
                    } else {
                        "Click-through"
                    };
                    if ui.button(click_through_label).clicked() {
                        self.set_click_through_mode(frame, !self.click_through_mode);
                    }
                }
                if self.overlay_mode && ui.button("Drag").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }

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

                    // Keep the ring inside its half of the split view so the
                    // left and right panes do not visually overlap.
                    let ring_size = ui.available_width().min(520.0);
                    let size = Vec2::splat(ring_size);
                    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                    draw_direction_ring(ui.painter(), rect, &self.latest_frame);
                });

                columns[1].vertical(|ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.heading("Source");
                            ui.label(format!("Selected source: {}", source_mode.label()));
                            ui.label(format!(
                                "Source state: {}",
                                Self::source_state_message(&source_state)
                            ));
                            ui.label(Self::source_help_text(source_mode, &source_state));

                            ui.add_space(16.0);
                            ui.heading("Window diagnostics");
                            ui.label(format!(
                                "Overlay mode: {}",
                                if self.overlay_mode { "On" } else { "Off" }
                            ));
                            ui.label(format!(
                                "Click-through mode: {}",
                                if self.click_through_mode { "On" } else { "Off" }
                            ));
                            ui.label(format!(
                                "Collection behavior: {}",
                                self.window_collection_behavior_debug
                            ));

                            ui.add_space(16.0);
                            ui.heading("Current channel energies");
                            ui.label(format!("Input mode: {}", self.source.layout().label()));
                            ui.add_space(8.0);

                            energy_bar(
                                ui,
                                "FL / L",
                                self.latest_energies.fl.max(self.latest_energies.sl),
                            );
                            energy_bar(
                                ui,
                                "FR / R",
                                self.latest_energies.fr.max(self.latest_energies.sr),
                            );
                            energy_bar(ui, "C", self.latest_energies.c);
                            energy_bar(ui, "SL", self.latest_energies.sl);
                            energy_bar(ui, "SR", self.latest_energies.sr);
                            energy_bar(ui, "RL", self.latest_energies.rl);
                            energy_bar(ui, "RR", self.latest_energies.rr);
                            energy_bar(ui, "LFE", self.latest_energies.lfe);

                            if matches!(self.source.layout(), ChannelLayout::Stereo) {
                                ui.add_space(16.0);
                                ui.heading("Stereo diagnostics");
                                ui.label(format!("Pan: {:+.2}", self.latest_energies.stereo_pan));
                                ui.label(format!(
                                    "Smoothed pan: {:+.2}",
                                    self.pipeline.stereo_smoothed_pan()
                                ));
                                ui.label(format!("Width: {:.2}", self.latest_energies.stereo_width));
                            }

                            ui.add_space(16.0);
                            ui.heading("Sector scores");
                            for sector in ORDERED_SECTORS {
                                let value = self.latest_frame.scores[sector.index()];
                                energy_bar(ui, sector.label(), value);
                            }

                            ui.add_space(16.0);
                            ui.heading("Tuning");
                            ui.add(
                                Slider::new(&mut tuning.stereo.min_energy, 0.0..=0.25)
                                    .text("Stereo min energy"),
                            );
                            ui.add(
                                Slider::new(&mut tuning.stereo.max_energy, 0.1..=2.5)
                                    .text("Stereo max energy"),
                            );
                            ui.add(
                                Slider::new(&mut tuning.stereo.pan_gain, 0.5..=4.0)
                                    .text("Stereo pan gain"),
                            );
                            ui.add(
                                Slider::new(&mut tuning.smoother.attack_alpha, 0.0..=1.0)
                                    .text("Attack"),
                            );
                            ui.add(
                                Slider::new(&mut tuning.smoother.decay_alpha, 0.0..=1.0)
                                    .text("Decay"),
                            );

                            if tuning.stereo.max_energy <= tuning.stereo.min_energy {
                                tuning.stereo.max_energy = tuning.stereo.min_energy + 0.001;
                            }

                            self.set_tuning(tuning);

                            ui.add_space(16.0);
                            ui.heading("System audio");
                            ui.label("System audio startup and source state are wired in.");
                            ui.label("PCM-to-energy decoding is not implemented yet.");
                        });
                });
            });
        });

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}
