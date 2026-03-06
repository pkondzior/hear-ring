use std::f32::consts::{PI, TAU};

use crate::types::{ChannelEnergies, ChannelLayout, Sector8};

pub trait AudioSource {
    fn layout(&self) -> ChannelLayout;
    fn next_energies(&mut self, dt: f32) -> ChannelEnergies;
}

/// Demo source that sweeps energy around the ring so the UI and estimator logic can be tested
/// before you wire real loopback capture into the same pipeline.
pub struct DemoSource {
    layout: ChannelLayout,
    phase: f32,
    pulse: f32,
}

impl DemoSource {
    pub fn new(layout: ChannelLayout) -> Self {
        Self {
            layout,
            phase: 0.0,
            pulse: 0.0,
        }
    }

    pub fn set_layout(&mut self, layout: ChannelLayout) {
        self.layout = layout;
    }

    fn directional_blob(&self, angle: f32, center: f32, width: f32) -> f32 {
        let delta = wrapped_angle(angle - center).abs();
        (1.0 - delta / width).clamp(0.0, 1.0)
    }
}

impl AudioSource for DemoSource {
    fn layout(&self) -> ChannelLayout {
        self.layout
    }

    fn next_energies(&mut self, dt: f32) -> ChannelEnergies {
        self.phase = (self.phase + dt * 0.60) % TAU;
        self.pulse = (self.pulse + dt * 1.7) % TAU;

        let source_angle = -PI * 0.5 + self.phase;
        let loudness = 0.12 + 0.88 * (0.5 + 0.5 * self.pulse.sin());

        let mut energies = ChannelEnergies::default();

        match self.layout {
            ChannelLayout::Stereo => {
                let left = self.directional_blob(source_angle, Sector8::FL.angle(), PI * 0.80)
                    + self.directional_blob(source_angle, Sector8::L.angle(), PI * 0.65) * 0.35;
                let right = self.directional_blob(source_angle, Sector8::FR.angle(), PI * 0.80)
                    + self.directional_blob(source_angle, Sector8::R.angle(), PI * 0.65) * 0.35;

                energies.fl = loudness * left;
                energies.fr = loudness * right;
            }
            ChannelLayout::Surround71 => {
                energies.fl = loudness * self.directional_blob(source_angle, Sector8::FL.angle(), PI * 0.55);
                energies.fr = loudness * self.directional_blob(source_angle, Sector8::FR.angle(), PI * 0.55);
                energies.c = loudness * self.directional_blob(source_angle, Sector8::F.angle(), PI * 0.55);
                energies.sl = loudness * self.directional_blob(source_angle, Sector8::L.angle(), PI * 0.55);
                energies.sr = loudness * self.directional_blob(source_angle, Sector8::R.angle(), PI * 0.55);
                energies.rl = loudness * self.directional_blob(source_angle, Sector8::BL.angle(), PI * 0.55);
                energies.rr = loudness * self.directional_blob(source_angle, Sector8::BR.angle(), PI * 0.55);
                energies.lfe = loudness * 0.18;
            }
        }

        energies
    }
}

fn wrapped_angle(mut angle: f32) -> f32 {
    while angle > PI {
        angle -= TAU;
    }
    while angle < -PI {
        angle += TAU;
    }
    angle
}
