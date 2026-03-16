use crate::estimators::DirectionEstimator;
use crate::types::{ChannelEnergies, Direction, DirectionFrame, DirectionScores};

pub struct Surround71Estimator {
    min_energy: f32,
    max_energy: f32,
}

impl Surround71Estimator {
    pub fn new() -> Self {
        Self {
            min_energy: 0.02,
            max_energy: 2.0,
        }
    }
}

impl DirectionEstimator for Surround71Estimator {
    fn estimate(&mut self, energies: &ChannelEnergies) -> DirectionFrame {
        let directional_total = energies.total_directional();
        if directional_total <= f32::EPSILON {
            return DirectionFrame::empty();
        }

        let mut scores = DirectionScores::default();

        scores[Direction::F] += 0.80 * energies.c;
        scores[Direction::FL] += 0.10 * energies.c;
        scores[Direction::FR] += 0.10 * energies.c;

        scores[Direction::FL] += 0.80 * energies.fl;
        scores[Direction::F] += 0.10 * energies.fl;
        scores[Direction::L] += 0.10 * energies.fl;

        scores[Direction::FR] += 0.80 * energies.fr;
        scores[Direction::F] += 0.10 * energies.fr;
        scores[Direction::R] += 0.10 * energies.fr;

        scores[Direction::L] += 0.80 * energies.sl;
        scores[Direction::FL] += 0.10 * energies.sl;
        scores[Direction::BL] += 0.10 * energies.sl;

        scores[Direction::R] += 0.80 * energies.sr;
        scores[Direction::FR] += 0.10 * energies.sr;
        scores[Direction::BR] += 0.10 * energies.sr;

        scores[Direction::BL] += 0.80 * energies.rl;
        scores[Direction::L] += 0.10 * energies.rl;
        scores[Direction::B] += 0.10 * energies.rl;

        scores[Direction::BR] += 0.80 * energies.rr;
        scores[Direction::R] += 0.10 * energies.rr;
        scores[Direction::B] += 0.10 * energies.rr;

        scores[Direction::B] += 0.25 * energies.rl.min(energies.rr);

        let gate = ((directional_total - self.min_energy) / (self.max_energy - self.min_energy))
            .clamp(0.0, 1.0);

        let sum: f32 = scores.iter().copied().sum::<f32>().max(1e-6);
        for score in scores.iter_mut() {
            *score = (*score / sum) * gate;
        }

        let left_sum = scores[Direction::FL] + scores[Direction::L] + scores[Direction::BL];
        let right_sum = scores[Direction::FR] + scores[Direction::R] + scores[Direction::BR];
        let front_sum = scores[Direction::FL] + scores[Direction::F] + scores[Direction::FR];
        let rear_sum = scores[Direction::BL] + scores[Direction::B] + scores[Direction::BR];

        let lateral_separation = (left_sum - right_sum).abs();
        let depth_separation = (front_sum - rear_sum).abs();
        let confidence =
            (0.45 * gate + 0.35 * lateral_separation + 0.20 * depth_separation).clamp(0.0, 1.0);

        DirectionFrame {
            scores,
            confidence,
            intensity: ((energies.total_with_lfe() - self.min_energy)
                / (self.max_energy - self.min_energy))
                .clamp(0.0, 1.0),
            active: gate > 0.01,
        }
    }
}
