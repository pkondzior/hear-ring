# Sound Hearing Aid

Small Rust prototype for a directional audio UI.

## What’s included
- 8-sector ring UI
- `StereoEstimator` that fills the honest front/side portion of the ring
- `Surround71Estimator` that projects 7.1 channel energies into the full ring
- `DirectionSmoother` for attack/decay smoothing
- `ProcessingPipeline` to run estimator + smoothing
- `DemoSource` that simulates moving sound energy
- `ScreenCaptureSource` for macOS system audio capture
- Source selection and source-state UI
- App wiring with `eframe`

## Status
Runnable prototype with the core types, estimators, smoothing, pipeline, demo audio source, ring UI, and app wiring in place.

## Current limitations
- Real capture is currently macOS-only via ScreenCaptureKit
- Real capture is currently focused on stereo energy extraction
- Audio buffer and format handling still needs hardening
- Platform-specific channel layout discovery is not implemented

## Run
cargo run

## macOS permission note
If you run this from Terminal, iTerm, or another terminal app, macOS Screen Recording permission must be granted to that host app for ScreenCaptureKit-based capture to work.
Go to System Settings -> Privacy & Security -> Screen Recording, enable the terminal app you are using, then restart it before running `cargo run` again.

## TODO
Harden the real capture path by improving audio format handling, validating buffer layouts, and tuning energy extraction for real-world input.  
Pipeline stays the same:

Audio source -> ChannelEnergies -> estimator -> smoother -> UI