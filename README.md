# Sound Hearing Aid

Small Rust prototype for a directional audio UI.

## What’s included
- 8-sector ring UI
- `StereoEstimator` that fills the honest front/side portion of the ring
- `Surround71Estimator` that projects 7.1 channel energies into the full ring
- `DirectionSmoother` for attack/decay smoothing
- `ProcessingPipeline` to run estimator + smoothing
- `DemoSource` that simulates moving sound energy
- App wiring with `eframe`

## Status
Runnable prototype with the core types, estimators, smoothing, pipeline, demo audio source, ring UI, and app wiring in place.

## What’s not included yet
- Real loopback capture
- PCM frame parsing from an actual device
- Platform-specific channel layout discovery

## Run
cargo run

## Next step
Replace `DemoSource` with a real audio source that produces `ChannelEnergies` from PCM frames.  
Pipeline stays the same:

Audio source -> ChannelEnergies -> estimator -> smoother -> UI