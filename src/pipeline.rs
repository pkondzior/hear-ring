use crate::estimators::{DirectionEstimator, StereoEstimator, StereoTuning, Surround71Estimator};
use crate::smoothing::{DirectionSmoother, SmootherTuning};
use crate::types::{ChannelEnergies, ChannelLayout, DirectionFrame};

pub struct ProcessingPipeline {
    layout: ChannelLayout,
    stereo_estimator: StereoEstimator,
    surround_estimator: Surround71Estimator,
    smoother: DirectionSmoother,
}

#[derive(Debug, Clone, Copy)]
pub struct PipelineTuning {
    pub stereo: StereoTuning,
    pub smoother: SmootherTuning,
}

impl ProcessingPipeline {
    pub fn new(layout: ChannelLayout) -> Self {
        Self {
            layout,
            stereo_estimator: StereoEstimator::new(),
            surround_estimator: Surround71Estimator::new(),
            smoother: DirectionSmoother::new(),
        }
    }

    pub fn set_layout(&mut self, layout: ChannelLayout) {
        self.layout = layout;
    }

    pub fn tuning(&self) -> PipelineTuning {
        PipelineTuning {
            stereo: self.stereo_estimator.tuning(),
            smoother: self.smoother.tuning(),
        }
    }

    pub fn set_tuning(&mut self, tuning: PipelineTuning) {
        self.stereo_estimator.set_tuning(tuning.stereo);
        self.smoother.set_tuning(tuning.smoother);
    }

    pub fn stereo_smoothed_pan(&self) -> f32 {
        self.stereo_estimator.smoothed_pan()
    }

    pub fn update(&mut self, energies: &ChannelEnergies) -> DirectionFrame {
        let raw = match self.layout {
            ChannelLayout::Stereo => self.stereo_estimator.estimate(energies),
            ChannelLayout::Surround71 => self.surround_estimator.estimate(energies),
        };

        self.smoother.update(raw)
    }
}
