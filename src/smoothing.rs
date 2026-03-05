use crate::types::{DirectionFrame, SECTOR_COUNT};

pub struct DirectionSmoother {
    displayed: DirectionFrame,
    attack_alpha: f32,
    decay_alpha: f32,
}

impl DirectionSmoother {
    pub fn new() -> Self {
        Self {
            displayed: DirectionFrame::empty(),
            attack_alpha: 0.28,
            decay_alpha: 0.08,
        }
    }

    pub fn update(&mut self, raw: DirectionFrame) -> DirectionFrame {
        let alpha = if raw.active {
            self.attack_alpha
        } else {
            self.decay_alpha
        };

        for i in 0..SECTOR_COUNT {
            self.displayed.scores[i] =
                alpha * raw.scores[i] + (1.0 - alpha) * self.displayed.scores[i];
        }

        self.displayed.confidence =
            alpha * raw.confidence + (1.0 - alpha) * self.displayed.confidence;
        self.displayed.intensity = alpha * raw.intensity + (1.0 - alpha) * self.displayed.intensity;
        self.displayed.active = self.displayed.intensity > 0.01;

        self.displayed.clone()
    }
}
