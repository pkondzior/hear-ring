# Sound Hearing Aid

Rust prototype for a directional audio UI.

## What’s included
- Core types: `ChannelEnergies`, `DirectionFrame`, `Sector8`
- `DirectionEstimator` trait
- `StereoEstimator` that fills the front/side portion of the ring
- `Surround71Estimator` that projects 7.1 channel energies into the ring

## Status
Runnable scaffold with the core types, stereo estimator, and 7.1 estimator in place. Pipeline, source, and UI are still missing.