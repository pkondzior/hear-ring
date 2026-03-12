use crate::estimators::DirectionEstimator;
use crate::types::{ChannelEnergies, DirectionFrame, Sector8, SECTOR_COUNT};

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

        let mut scores = [0.0; SECTOR_COUNT];

        scores[Sector8::F.index()] += 0.80 * energies.c;
        scores[Sector8::FL.index()] += 0.10 * energies.c;
        scores[Sector8::FR.index()] += 0.10 * energies.c;

        scores[Sector8::FL.index()] += 0.80 * energies.fl;
        scores[Sector8::F.index()] += 0.10 * energies.fl;
        scores[Sector8::L.index()] += 0.10 * energies.fl;

        scores[Sector8::FR.index()] += 0.80 * energies.fr;
        scores[Sector8::F.index()] += 0.10 * energies.fr;
        scores[Sector8::R.index()] += 0.10 * energies.fr;

        scores[Sector8::L.index()] += 0.80 * energies.sl;
        scores[Sector8::FL.index()] += 0.10 * energies.sl;
        scores[Sector8::BL.index()] += 0.10 * energies.sl;

        scores[Sector8::R.index()] += 0.80 * energies.sr;
        scores[Sector8::FR.index()] += 0.10 * energies.sr;
        scores[Sector8::BR.index()] += 0.10 * energies.sr;

        scores[Sector8::BL.index()] += 0.80 * energies.rl;
        scores[Sector8::L.index()] += 0.10 * energies.rl;
        scores[Sector8::B.index()] += 0.10 * energies.rl;

        scores[Sector8::BR.index()] += 0.80 * energies.rr;
        scores[Sector8::R.index()] += 0.10 * energies.rr;
        scores[Sector8::B.index()] += 0.10 * energies.rr;

        scores[Sector8::B.index()] += 0.25 * energies.rl.min(energies.rr);

        let gate = ((directional_total - self.min_energy) / (self.max_energy - self.min_energy))
            .clamp(0.0, 1.0);

        let sum: f32 = scores.iter().sum::<f32>().max(1e-6);
        for score in &mut scores {
            *score = (*score / sum) * gate;
        }

        let left_sum =
            scores[Sector8::FL.index()] + scores[Sector8::L.index()] + scores[Sector8::BL.index()];
        let right_sum =
            scores[Sector8::FR.index()] + scores[Sector8::R.index()] + scores[Sector8::BR.index()];
        let front_sum =
            scores[Sector8::FL.index()] + scores[Sector8::F.index()] + scores[Sector8::FR.index()];
        let rear_sum =
            scores[Sector8::BL.index()] + scores[Sector8::B.index()] + scores[Sector8::BR.index()];

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
