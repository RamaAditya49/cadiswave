# Device settings design

Rama approved all ten proposed feature areas on 2026-10-04.
The application must remain responsive during audio and USB operations.

## Scope

1. Add direct software low-cut controls for an explicitly selected microphone source.
2. Add meeting, podcast, and streaming effect presets, plus named user presets.
3. Add a bounded microphone recording test, peak measurements, explicit playback, and cancellation.
4. Show observed model, firmware, API version, connection state, and capture sample rate.
5. Explain feature support for the exact selected USB profile.
6. Add a direct hardware monitor mix slider where the protocol supports it.
7. Add Clipguard controls only where primary protocol evidence defines their encoding.
8. Add hardware low-cut controls only where primary protocol evidence defines their encoding.
9. Add LED controls only where primary protocol evidence defines their encoding.
10. Investigate sample-rate negotiation and hardware persistence before enabling either operation.

## Boundaries

Use Rust 1.98.1, GTK 4.14+, libadwaita 1.5+, and the checked-in dependency lockfile.
Keep state validation in cadiswave-core, worker ownership in cadiswave-runtime, and presentation in cadiswave-desktop.
Use existing typed commands and immutable snapshots.
Never derive unsupported offsets from another model.
Preserve phantom power, reserved fields, the default audio input, and unrelated settings.
Keep recordings in bounded worker memory.
Start recording and playback only after their explicit button actions.
Discard recordings when their session closes or their source identity changes.
Cancel and reap owned audio children during shutdown.
Use an exact capture identity and an explicitly selected playback output.
Do not use an automatic output fallback after disconnect.

## Presentation

Use native preferences groups with concise labels and translated explanations.
Show software effects before hardware features.
Keep software source selection explicit when several sources use one device.
Preserve current effects when changing low-cut alone.
Keep saved user presets separate from built-in presets.
Show observed hardware values separately from pending edits.
Use the existing debounce and readback paths for device sliders.
Do not submit commands while rendering snapshots.
Display actual sample-rate evidence and identify unavailable observations.
Keep device persistence separate from application persistence.

## Window size

Rama requested an adaptive initial window after the first 800 by 600 layout clipped the main controls.
Fit the initial and restored size to the monitor's logical dimensions.
Use up to 90 percent of each monitor dimension, subject to the minimum of 1024 by 720.
Rama increased the minimum after the first host update to keep the three-column layout spacious.
Preserve saved dimensions when they fit the current monitor.
Keep manual resizing and maximized state.
Use compact horizontal gain and knob controls in the narrow single-column layout.
Keep mute, gain, and the gain slider visible at the minimum size.

## Verification

Test source identity, preset validation, recording bounds, cancellation, and stale completion rejection.
Test exact hardware encodings and preservation of unrelated bytes.
Test GTK callbacks and repeated refreshes in the private display and audio namespace.
Run formatting, workspace lint, workspace tests, release builds, and the installed smoke.
Inspect rendered English and Indonesian settings at narrow and normal sizes.
Record protocol references and physical test limits separately.
Controls without sufficient evidence remain unavailable with a specific explanation.
