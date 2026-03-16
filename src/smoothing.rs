use crate::types::DirectionFrame;

#[derive(Debug, Clone, Copy)]
pub struct SmootherTuning {
    pub attack_alpha: f32,
    pub decay_alpha: f32,
}

pub struct DirectionSmoother {
    displayed: DirectionFrame,
    attack_alpha: f32,
    decay_alpha: f32,
}

impl DirectionSmoother {
    pub fn new() -> Self {
        Self {
            displayed: DirectionFrame::empty(),
            attack_alpha: 0.44,
            decay_alpha: 0.37,
        }
    }

    pub fn tuning(&self) -> SmootherTuning {
        SmootherTuning {
            attack_alpha: self.attack_alpha,
            decay_alpha: self.decay_alpha,
        }
    }

    pub fn set_tuning(&mut self, tuning: SmootherTuning) {
        self.attack_alpha = tuning.attack_alpha.clamp(0.0, 1.0);
        self.decay_alpha = tuning.decay_alpha.clamp(0.0, 1.0);
    }

    pub fn update(&mut self, raw: DirectionFrame) -> DirectionFrame {
        let alpha = if raw.active {
            self.attack_alpha
        } else {
            self.decay_alpha
        };

        for (displayed, raw) in self.displayed.scores.iter_mut().zip(raw.scores.iter()) {
            *displayed = alpha * *raw + (1.0 - alpha) * *displayed;
        }

        self.displayed.confidence =
            alpha * raw.confidence + (1.0 - alpha) * self.displayed.confidence;
        self.displayed.intensity = alpha * raw.intensity + (1.0 - alpha) * self.displayed.intensity;
        self.displayed.active = self.displayed.intensity > 0.01;

        self.displayed.clone()
    }
}
