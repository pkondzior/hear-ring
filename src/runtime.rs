use std::time::{Duration, Instant};

use gpui::{App, AsyncApp, Global, Timer};

use crate::{
    pipeline::{PipelineTuning, ProcessingPipeline},
    source::{AudioSource, AudioSourceState, DemoSource, ScreenCaptureSource},
    types::{ChannelEnergies, ChannelLayout, DirectionFrame},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceMode {
    Demo,
    SystemAudio,
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
            Self::Demo(_) => SourceMode::Demo,
            Self::SystemAudio(_) => SourceMode::SystemAudio,
        }
    }

    fn set_layout(&mut self, layout: ChannelLayout) {
        if let Self::Demo(source) = self {
            source.set_layout(layout);
        }
    }

    fn layout(&self) -> ChannelLayout {
        match self {
            Self::Demo(source) => source.layout(),
            Self::SystemAudio(source) => source.layout(),
        }
    }

    fn next_energies(&mut self, dt: f32) -> ChannelEnergies {
        match self {
            Self::Demo(source) => source.next_energies(dt),
            Self::SystemAudio(source) => source.next_energies(dt),
        }
    }

    fn state(&self) -> AudioSourceState {
        match self {
            Self::Demo(source) => source.state(),
            Self::SystemAudio(source) => source.state(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeSnapshot {
    pub source_mode: SourceMode,
    pub source_state: AudioSourceState,
    pub layout: ChannelLayout,
    pub latest_energies: ChannelEnergies,
    pub latest_frame: DirectionFrame,
    pub stereo_smoothed_pan: f32,
}

pub struct RadarRuntime {
    source: AppSource,
    pipeline: ProcessingPipeline,
    last_tick: Instant,
    latest_energies: ChannelEnergies,
    latest_frame: DirectionFrame,
}

impl Global for RadarRuntime {}

impl RadarRuntime {
    const TICK_INTERVAL: Duration = Duration::from_millis(16);

    pub fn register_global(cx: &mut App) {
        let layout = ChannelLayout::Stereo;
        cx.set_global(Self {
            source: AppSource::demo(layout),
            pipeline: ProcessingPipeline::new(layout),
            last_tick: Instant::now(),
            latest_energies: ChannelEnergies::default(),
            latest_frame: DirectionFrame::empty(),
        });

        cx.spawn(async move |cx| Self::run(cx).await).detach();
    }

    async fn run(cx: &mut AsyncApp) {
        loop {
            Timer::after(Self::TICK_INTERVAL).await;

            if cx
                .update_global(|runtime: &mut Self, _cx| runtime.tick())
                .is_err()
            {
                break;
            }
        }
    }

    pub fn snapshot(&self) -> RuntimeSnapshot {
        RuntimeSnapshot {
            source_mode: self.source.mode(),
            source_state: self.source.state(),
            layout: self.source.layout(),
            latest_energies: self.latest_energies,
            latest_frame: self.latest_frame.clone(),
            stereo_smoothed_pan: self.pipeline.stereo_smoothed_pan(),
        }
    }

    pub fn tuning(&self) -> PipelineTuning {
        self.pipeline.tuning()
    }

    pub fn set_layout(&mut self, layout: ChannelLayout) {
        self.source.set_layout(layout);
        self.pipeline.set_layout(layout);
        self.last_tick = Instant::now();
        self.latest_energies = ChannelEnergies::default();
        self.latest_frame = DirectionFrame::empty();
    }

    pub fn set_source_mode(&mut self, mode: SourceMode) {
        let current_layout = self.source.layout();
        self.source = match mode {
            SourceMode::Demo => AppSource::demo(current_layout),
            SourceMode::SystemAudio => AppSource::system_audio(),
        };
        self.pipeline.set_layout(self.source.layout());
        self.last_tick = Instant::now();
        self.latest_energies = ChannelEnergies::default();
        self.latest_frame = DirectionFrame::empty();
    }

    pub fn set_stereo_min_energy(&mut self, value: f32) {
        self.update_tuning(|tuning| {
            tuning.stereo.min_energy = value.clamp(0.0, 0.25);
        });
    }

    pub fn set_stereo_max_energy(&mut self, value: f32) {
        self.update_tuning(|tuning| {
            tuning.stereo.max_energy = value.clamp(0.1, 2.5);
        });
    }

    pub fn set_stereo_pan_gain(&mut self, value: f32) {
        self.update_tuning(|tuning| {
            tuning.stereo.pan_gain = value.clamp(0.5, 4.0);
        });
    }

    pub fn set_attack_alpha(&mut self, value: f32) {
        self.update_tuning(|tuning| {
            tuning.smoother.attack_alpha = value.clamp(0.0, 1.0);
        });
    }

    pub fn set_decay_alpha(&mut self, value: f32) {
        self.update_tuning(|tuning| {
            tuning.smoother.decay_alpha = value.clamp(0.0, 1.0);
        });
    }

    fn update_tuning(&mut self, update: impl FnOnce(&mut PipelineTuning)) {
        let mut tuning = self.pipeline.tuning();
        update(&mut tuning);

        if tuning.stereo.max_energy <= tuning.stereo.min_energy {
            tuning.stereo.max_energy = (tuning.stereo.min_energy + 0.001).min(2.5);
        }

        self.pipeline.set_tuning(tuning);
    }

    fn tick(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_tick).as_secs_f32().max(1.0 / 240.0);
        self.last_tick = now;
        self.latest_energies = self.source.next_energies(dt);
        self.latest_frame = self.pipeline.update(&self.latest_energies);
    }
}
