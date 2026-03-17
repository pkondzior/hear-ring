# Sound Hearing Aid

A Rust prototype for visualizing directional audio as a small radar overlay.

## Overview

The app currently runs as a GPUI desktop app with two windows:

- an **options window** for source selection, tuning, and diagnostics
- a transparent **overlay window** that renders the radar ring

At the core, the app converts audio input into channel energies, estimates a direction distribution, smooths the result, and renders it as a ring.

## Preview

![Sound Hearig Aid preview](docs/Preview.jpg)

## Current features

- GPUI-based options window
- GPUI-based transparent overlay window
- demo audio source for testing without real capture
- macOS system-audio capture via ScreenCaptureKit
- stereo and 7.1 processing paths
- configurable tuning controls for stereo estimation and smoothing
- runtime diagnostics for channel energy and direction scores
- draggable or click-through overlay interaction modes

## Processing pipeline

Audio source -> `ChannelEnergies` -> estimator -> smoother -> overlay/UI

## Sources

### Demo
A synthetic source that moves energy around the ring so the full pipeline can be tested without real audio capture.

### System Audio
On macOS, the app can capture system audio through ScreenCaptureKit and feed the stereo analysis path.

## Running

Run the app with:

`cargo run`

## macOS permissions

ScreenCaptureKit-based system-audio capture requires **Screen Recording** permission for the host app you launch from, such as Terminal or iTerm.

Open:

`System Settings -> Privacy & Security -> Screen Recording`

Enable your terminal app, then restart it before launching the project again.

## Current limitations

- real system-audio capture is macOS-only
- real capture currently feeds the stereo path only
- audio format and buffer handling still needs hardening
- automatic channel-layout discovery is not implemented
- multi-display overlay management is not implemented yet

## Status

This is still a prototype, but the main architecture is now in place:

- shared runtime
- separate options and overlay windows
- directional estimators
- smoothing and tuning
- radar rendering in the overlay

## Next likely work

- harden the ScreenCaptureKit capture path
- improve options-window layout and polish
- expand overlay behavior toward full multi-display support
- improve diagnostics and controller ergonomics
