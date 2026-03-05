# Sound Hearing Aid

Rust prototype for a directional audio UI.

## What’s included
- Core types: `ChannelEnergies`, `DirectionFrame`, `Sector8`
- `DirectionEstimator` trait
- `StereoEstimator` that fills the front/side portion of the ring
- `Surround71Estimator` that projects 7.1 channel energies into the ring
- `DirectionSmoother` for attack/decay smoothing
- `ProcessingPipeline` to run estimator + smoothing

## Status
Runnable scaffold with the core types, stereo estimator, 7.1 estimator, smoothing, and pipeline in place. Audio source and UI are still missing.