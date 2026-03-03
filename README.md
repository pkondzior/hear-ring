# Sound Hearing Aid

Rust prototype for a directional audio UI.

## What’s included
- Core types: `ChannelEnergies`, `DirectionFrame`, `Sector8`
- `DirectionEstimator` trait
- `StereoEstimator` that fills the front/side portion of the ring

## Status
Runnable scaffold with the core types and stereo estimator in place. The 7.1 estimator, pipeline, source, and UI are still missing.