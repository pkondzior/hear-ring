use crate::estimators::DirectionEstimator;
use crate::types::{ChannelEnergies, DirectionFrame, Sector8, SECTOR_COUNT};

const STEREO_ARC: [(f32, Sector8); 5] = [
    (-1.0, Sector8::L),
    (-0.5, Sector8::FL),
    (0.0, Sector8::F),
    (0.5, Sector8::FR),
    (1.0, Sector8::R),
];

pub struct StereoEstimator {
    min_energy: f32,
    max_energy: f32,
    pan_gain: f32,
    smoothed_pan: f32,
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

    pub fn new() -> Self {
        Self {
            min_energy: 0.0,
            max_energy: 2.5,
            pan_gain: 1.6,
            smoothed_pan: 0.0,
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
}

impl DirectionEstimator for StereoEstimator {
    fn estimate(&mut self, energies: &ChannelEnergies) -> DirectionFrame {
        let left = energies.fl;
        let right = energies.fr;
        let total = left + right;

        if total <= f32::EPSILON {
            self.smoothed_pan = 0.0;
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
        let pan_abs = pan.abs();
        let width = if energies.stereo_width > 0.0 {
            energies.stereo_width
        } else {
            (pan_abs * 0.6).clamp(0.0, 1.0)
        };

        let mut scores = [0.0; SECTOR_COUNT];
        if pan <= STEREO_ARC[0].0 {
            scores[Sector8::L.index()] = 1.0;
        } else if pan >= STEREO_ARC[STEREO_ARC.len() - 1].0 {
            scores[Sector8::R.index()] = 1.0;
        } else {
            for pair in STEREO_ARC.windows(2) {
                let (start_pos, start_sector) = pair[0];
                let (end_pos, end_sector) = pair[1];

                if pan >= start_pos && pan <= end_pos {
                    let t = ((pan - start_pos) / (end_pos - start_pos)).clamp(0.0, 1.0);
                    scores[start_sector.index()] = 1.0 - t;
                    scores[end_sector.index()] = t;
                    break;
                }
            }
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
