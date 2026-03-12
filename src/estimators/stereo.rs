use crate::estimators::DirectionEstimator;
use crate::types::{ChannelEnergies, DirectionFrame, Sector8, SECTOR_COUNT};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanLatch {
    Center,
    Left,
    Right,
}

impl PanLatch {
    fn label(self) -> &'static str {
        match self {
            PanLatch::Center => "Center",
            PanLatch::Left => "Left",
            PanLatch::Right => "Right",
        }
    }
}

pub struct StereoEstimator {
    min_energy: f32,
    max_energy: f32,
    pan_gain: f32,
    smoothed_pan: f32,
    pan_latch: PanLatch,
}

#[derive(Debug, Clone, Copy)]
pub struct StereoTuning {
    pub min_energy: f32,
    pub max_energy: f32,
    /// Scales the perceived left/right balance before mapping it into sector scores.
    /// - 1.0 keeps the current behaviour.
    /// - >1.0 makes small L/R differences push toward FL/FR (and L/R) sooner.
    /// - <1.0 makes the estimator less sensitive around center.
    pub pan_gain: f32,
}

impl StereoEstimator {
    const PAN_SMOOTHING_ALPHA: f32 = 0.22;
    const CENTER_ENTER_THRESHOLD: f32 = 0.12;
    const SIDE_ENTER_THRESHOLD: f32 = 0.22;
    const OPPOSITE_SIDE_THRESHOLD: f32 = 0.28;

    pub fn new() -> Self {
        Self {
            min_energy: 0.0,
            max_energy: 2.5,
            pan_gain: 1.6,
            smoothed_pan: 0.0,
            pan_latch: PanLatch::Center,
        }
    }

    pub fn tuning(&self) -> StereoTuning {
        StereoTuning {
            min_energy: self.min_energy,
            max_energy: self.max_energy,
            pan_gain: self.pan_gain,
        }
    }

    pub fn set_tuning(&mut self, tuning: StereoTuning) {
        self.min_energy = tuning.min_energy.max(0.0);
        self.max_energy = tuning.max_energy.max(self.min_energy + 0.001);
        self.pan_gain = tuning.pan_gain.clamp(0.1, 10.0);
    }

    pub fn smoothed_pan(&self) -> f32 {
        self.smoothed_pan
    }

    pub fn pan_latch_label(&self) -> &'static str {
        self.pan_latch.label()
    }
}

impl DirectionEstimator for StereoEstimator {
    fn estimate(&mut self, energies: &ChannelEnergies) -> DirectionFrame {
        let left = energies.fl;
        let right = energies.fr;
        let total = left + right;

        if total <= f32::EPSILON {
            self.smoothed_pan = 0.0;
            self.pan_latch = PanLatch::Center;
            return DirectionFrame::empty();
        }

        let gate =
            ((total - self.min_energy) / (self.max_energy - self.min_energy)).clamp(0.0, 1.0);

        let fallback_pan = ((right - left) / (total + 1e-6)).clamp(-1.0, 1.0);
        let raw_pan = if energies.stereo_pan != 0.0 || energies.stereo_width != 0.0 {
            energies.stereo_pan
        } else {
            fallback_pan
        };

        self.smoothed_pan = Self::PAN_SMOOTHING_ALPHA * raw_pan
            + (1.0 - Self::PAN_SMOOTHING_ALPHA) * self.smoothed_pan;

        let pan = (self.smoothed_pan * self.pan_gain).clamp(-1.0, 1.0);
        let raw_pan_abs = pan.abs();
        let width = if energies.stereo_width > 0.0 {
            energies.stereo_width
        } else {
            (raw_pan_abs * 0.6).clamp(0.0, 1.0)
        };

        self.pan_latch = match self.pan_latch {
            PanLatch::Center => {
                if pan <= -Self::SIDE_ENTER_THRESHOLD {
                    PanLatch::Left
                } else if pan >= Self::SIDE_ENTER_THRESHOLD {
                    PanLatch::Right
                } else {
                    PanLatch::Center
                }
            }
            PanLatch::Left => {
                if pan >= Self::OPPOSITE_SIDE_THRESHOLD {
                    PanLatch::Right
                } else if pan.abs() <= Self::CENTER_ENTER_THRESHOLD {
                    PanLatch::Center
                } else {
                    PanLatch::Left
                }
            }
            PanLatch::Right => {
                if pan <= -Self::OPPOSITE_SIDE_THRESHOLD {
                    PanLatch::Left
                } else if pan.abs() <= Self::CENTER_ENTER_THRESHOLD {
                    PanLatch::Center
                } else {
                    PanLatch::Right
                }
            }
        };

        let effective_pan = match self.pan_latch {
            PanLatch::Center => pan,
            PanLatch::Left => pan.min(-Self::CENTER_ENTER_THRESHOLD),
            PanLatch::Right => pan.max(Self::CENTER_ENTER_THRESHOLD),
        };
        let pan_abs = effective_pan.abs();

        // Once the signal is meaningfully off-center, suppress front so brief
        // balance collapses do not flash F over an already-established side.
        let side_commit = ((pan_abs - 0.16) / 0.22).clamp(0.0, 1.0);
        let front_suppression = 1.0 - 0.85 * side_commit;

        let front_strength =
            (1.0 - pan_abs * (0.85 + 0.15 * width)).clamp(0.0, 1.0) * front_suppression;
        let diagonal_strength = (1.0 - ((pan_abs - 0.38) / 0.30).abs()).clamp(0.0, 1.0);
        let side_strength = ((pan_abs - 0.58) / 0.30).clamp(0.0, 1.0) * (0.55 + 0.45 * width);

        let mut scores = [0.0; SECTOR_COUNT];
        scores[Sector8::F.index()] = 0.95 * front_strength;

        if effective_pan >= 0.0 {
            scores[Sector8::FR.index()] = 0.90 * diagonal_strength;
            scores[Sector8::R.index()] = 0.85 * side_strength;
        } else {
            scores[Sector8::FL.index()] = 0.90 * diagonal_strength;
            scores[Sector8::L.index()] = 0.85 * side_strength;
        }

        for score in &mut scores {
            *score *= gate;
        }

        let confidence = (0.25 + 0.50 * pan_abs + 0.25 * width).clamp(0.0, 1.0) * gate;

        DirectionFrame {
            scores,
            confidence,
            intensity: gate,
            active: gate > 0.01,
        }
    }
}
