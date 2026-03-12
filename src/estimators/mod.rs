use crate::types::{ChannelEnergies, DirectionFrame};

pub mod stereo;
pub mod surround71;

pub use stereo::StereoEstimator;
pub use stereo::StereoTuning;
pub use surround71::Surround71Estimator;

pub trait DirectionEstimator {
    fn estimate(&mut self, energies: &ChannelEnergies) -> DirectionFrame;
}
