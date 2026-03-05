use crate::estimators::{DirectionEstimator, StereoEstimator, Surround71Estimator};
use crate::smoothing::DirectionSmoother;
use crate::types::{ChannelEnergies, ChannelLayout, DirectionFrame};

pub struct ProcessingPipeline {
    layout: ChannelLayout,
    stereo_estimator: StereoEstimator,
    surround_estimator: Surround71Estimator,
    smoother: DirectionSmoother,
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

    pub fn update(&mut self, energies: &ChannelEnergies) -> DirectionFrame {
        let raw = match self.layout {
            ChannelLayout::Stereo => self.stereo_estimator.estimate(energies),
            ChannelLayout::Surround71 => self.surround_estimator.estimate(energies),
        };

        self.smoother.update(raw)
    }
}
