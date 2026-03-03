use crate::estimators::DirectionEstimator;
use crate::types::{ChannelEnergies, DirectionFrame, Sector8, SECTOR_COUNT};

pub struct StereoEstimator {
    min_energy: f32,
    max_energy: f32,
}

impl StereoEstimator {
    pub fn new() -> Self {
        Self {
            min_energy: 0.02,
            max_energy: 1.25,
        }
    }
}

impl DirectionEstimator for StereoEstimator {
    fn estimate(&mut self, energies: &ChannelEnergies) -> DirectionFrame {
        let left = energies.fl;
        let right = energies.fr;
        let total = left + right;

        if total <= f32::EPSILON {
            return DirectionFrame::empty();
        }

        let balance = (left - right) / (total + 1e-6);
        let mut scores = [0.0; SECTOR_COUNT];

        let x = 1.0 - balance;
        let front_left = (1.0 - (x - 0.0).abs()).max(0.0);
        let front = (1.0 - (x - 1.0).abs()).max(0.0);
        let front_right = (1.0 - (x - 2.0).abs()).max(0.0);

        scores[Sector8::FL.index()] = 0.65 * front_left;
        scores[Sector8::F.index()] = 0.90 * front;
        scores[Sector8::FR.index()] = 0.65 * front_right;

        let left_strength = balance.max(0.0);
        let right_strength = (-balance).max(0.0);
        scores[Sector8::L.index()] = left_strength * 0.55;
        scores[Sector8::R.index()] = right_strength * 0.55;

        let gate =
            ((total - self.min_energy) / (self.max_energy - self.min_energy)).clamp(0.0, 1.0);
        for score in &mut scores {
            *score *= gate;
        }

        let confidence = (balance.abs() * 0.75 + gate * 0.25).clamp(0.0, 1.0);

        DirectionFrame {
            scores,
            confidence,
            intensity: gate,
            active: gate > 0.01,
        }
    }
}
