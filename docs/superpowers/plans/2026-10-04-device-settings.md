# Device Settings Implementation Plan

> **For agentic workers:** Use focused parallel tasks for independent modules. Keep shared controller edits under one owner.

**Goal:** Implement the ten approved feature areas with verified device controls and responsive native settings.

**Architecture:** Reuse the existing controller, source effects, serialized device workers, and atomic configuration store.
Add bounded microphone test workers and exact-profile capability explanations.
Render completion events and current observations through native preferences controls.

**Tech Stack:** Rust 1.98.1, GTK 4.14+, libadwaita 1.5+, PipeWire, libusb, Fluent.

**Spec:** `docs/superpowers/specs/2026-10-04-device-settings-design.md`

## Global Constraints

- Use Rust 1.98.1, GTK 4.14+, libadwaita 1.5+, and the checked-in dependency lockfile.
- Preserve phantom power, reserved fields, the default audio input, and unrelated settings.
- Keep recordings in bounded worker memory.
- Start recording and playback only after their explicit button actions.
- Use an exact capture identity and an explicitly selected playback output.
- Never derive unsupported offsets from another model.
- Keep English and Indonesian product facts consistent.

## Review Focus

- Source or output replacement must cancel a recording or playback session.
- Several sources or devices must never cause an implicit target selection.
- Repeated GTK refreshes must preserve drafts and must not submit commands.
- Malformed preset stores must remain unchanged and reject replacement.
- Partial USB writes must preserve unrelated state and report actual completion.

### Task 1: Protocol evidence and exact-profile controls

Files: core profiles, protocol, capabilities, device worker, hardware tests, protocol documentation.

- [x] Inspect primary protocol sources at fixed revisions.
- [x] Record encodings, model limits, transfer details, and unresolved fields.
- [x] Write failing protocol tests for supported new settings and reserved-byte preservation.
- [x] Run `cargo test --locked -p cadiswave-core --test protocol --test hardware_support`.
- [x] Implement only fields supported by exact-profile evidence.
- [x] Extend worker readback comparisons and capability explanations.
- [x] Run core and runtime device tests.

### Task 2: Software settings and voice presets

Files: core device settings projection, preset types, preferences, controller mutations, core and runtime tests.

Interfaces: source resolution consumes `AppSnapshot` and returns explicit microphone source candidates.
Presets produce validated `FxSettings` for `AppCommand::SetFx`.
Named presets persist through the existing atomic preferences store.

- [x] Write failing tests for exact source resolution and preset validation.
- [x] Verify missing APIs fail before implementation.
- [x] Implement meeting, podcast, streaming, and named presets.
- [x] Test corrupt stores, name limits, and preservation of existing preferences.
- [x] Add typed save and delete commands with atomic persistence.
- [x] Run the core and controller mutation suites.

### Task 3: Microphone test

Files: core microphone test state, runtime worker, controller integration, standalone desktop microphone test widget, tests.

Interfaces: `AppSnapshot::mic_test` contains metadata and phases, never PCM.
`AppCommand::RecordMicTest` selects a source and duration.
`AppCommand::PlayMicTest` selects a session and output.
`AppCommand::CancelMicTest` releases the session.

- [x] Write failing analysis, duration, target identity, and cancellation tests.
- [x] Add recording and playback workers with bounded memory and process deadlines.
- [x] Validate capture and output identities before each operation.
- [x] Reject stale completions and clear recording data after cancellation.
- [x] Integrate worker startup, events, and shutdown.
- [x] Add native record, playback, output selection, and stop controls.
- [x] Run analysis, worker, and controller tests.

### Task 4: Native settings presentation

Files: desktop device settings modules, Fluent catalogs, CSS, GTK tests.

- [x] Write GTK regressions for rendering, source selection, presets, and direct monitor controls.
- [x] Add microphone source selection, low-cut controls, and preset actions.
- [x] Add observed device details and capability explanations.
- [x] Add direct controls for verified hardware fields.
- [x] Connect microphone testing to the standalone native widget.
- [x] Inspect English and Indonesian renders at narrow and normal sizes.
- [x] Verify pending, failure, reconnect, and disabled-animation behavior.

### Task 5: Verification and review

- [x] Run `cargo fmt --all -- --check`.
- [x] Run `cargo clippy --locked --workspace --all-targets -- -D warnings`.
- [x] Run `cargo test --locked --workspace`.
- [x] Build locked release binaries and runtime examples.
- [x] Run installed smoke and ignored GTK cases in the private namespace.
- [x] Obtain an independent review and correct important findings.
- [x] Update bilingual documentation with implemented controls and remaining physical test limits.
- [x] Commit changes with the required CADIS trailer.

### Task 6: Adaptive initial window

- [x] Test saved dimensions, small monitors, logical dimensions, and invalid monitor data.
- [x] Fit initial and restored dimensions to the current monitor.
- [x] Use the same 640 by 480 minimum in preference loading, controller updates, and window construction.
- [x] Compact the narrow device layout without changing device commands.
- [x] Inspect 800 by 600 and 640 by 480 renders and verify primary controls remain visible.

## Execution notes

Work uses branch `feat/device-settings` from `0300a4f`.
The initial checkout was clean.
Titen project resolution returned `project_not_registered`; no project or memory was created.
Rama authorized implementation of the proposed feature list; no publication or system installation was requested.

## Evidence and limits

Rama uses Linux only. No Windows or macOS capture was required.
Original Wave XLR monitor mix uses pinned WaveController Linux knob evidence.
Wave:3 Clipguard requires observed API 5.3 or 5.4 at admission and before writes.
Dock MK.2 processing uses exact block masks and verified readback paths.
LED, rate-change, and explicit hardware-save protocols remain unavailable.
No host USB writes or host installation changes occurred.
The private installed test passed recording, measurements, explicit playback, discard, routing, and child cleanup.
The final full private release smoke passed with three complete microphone cycles.
All 42 private GTK cases passed. Final locale changes passed the integrated gallery.
The full workspace passed 485 tests and reported 59 ignored cases.
The other 17 ignored cases remain outside these local passes.
The independent review found no remaining important issues after corrections.
One process test assertion failed transiently; a documented terminal-state parser regression now passes.
Production cleanup was not changed.
Transient PipeWire discovery changes previously expired a valid microphone recording.
The controller now retains exact known identities while the worker checks live targets before and after audio operations.
Two private runs passed 20 recording, playback, and discard cycles each.
A privilege test now retries only Busy after its known owner releases a shared installation lease.
Production lease behavior remains unchanged.
A successful fsync delayed by six seconds reproduced a setup fixture timeout.
The fixture now uses a 30-second total deadline; production setup remains unchanged.
