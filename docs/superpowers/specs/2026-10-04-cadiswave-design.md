# CadisWave desktop design

Status: approved by Rama on 2026-10-04. See [implementation verification](../../verification.md) for current results.

## Purpose

CadisWave provides daily Wave XLR control and audio mixing on Linux.
Rama uses the device daily and requires responsive controls, stable capture, and clear device feedback.
The application supports English and Bahasa Indonesia.
The supplied design defines the visual direction and interaction details.
The public repository must remain easy to understand and maintain.

## Accepted requirements

- Use CadisWave as the application name.
- Use `cadiswave` as the repository, directory, and desktop command name.
- Use Rust for the application and background service.
- Support English and Bahasa Indonesia throughout the desktop interface.
- Apply the supplied design in detail.
- Keep the design archive, reference images, and reference runtime outside the repository.
- Preserve the MIT license and upstream credits.
- Keep hardware operations separate from presentation.
- Verify behavior before describing the application as stable.

## Current evidence

The local branch contains five custom commits after the shared `v0.1.5` ancestor.
The inspected local revision is `9e96e5e6bda5b4039ecfa76855fda8c84633f062`.
The existing GitHub repository is `RamaAditya49/openwave`.
GitHub reports that repository as independent, with `fork: false`.
The existing remotes are `origin` for rikkichy and `rama` for Rama.
The working directory was clean before this document was added.

Upstream now provides a Rust workspace.
The inspected upstream release is `v1.2.0`, revision `4172c71db3e0b929571d02c06bbcb3456726e5e2`.
Its crates separate core rules, runtime operations, and GTK presentation.
Its existing behavior includes routing, scenes, DSP, diagnostics, and capture monitoring.
These functions need local verification before adoption.

The connected device identifies as `0fd9:007d`, Wave XLR.
Read-only ALSA inspection reports microphone gain from 0 to 75 dB.
Read-only ALSA inspection reports headphone volume from -60 to 0 dB.
Upstream's generic Wave XLR profile permits 80 dB gain.
That difference requires device-specific validation before enabling the larger range.
The initial CadisWave Wave XLR gain control must retain the verified 75 dB limit.
ALSA ranges and vendor encodings must remain separate concepts.

The host has GTK 4.14.5 and libadwaita 1.5.0 runtime libraries.
The native development packages are missing.
The inspected upstream toolchain file requests Rust 1.98.1.
The default compiler currently reports Rust 1.95.0.
The current command environment cannot reach the desktop session bus.
These conditions require resolution before build and desktop acceptance checks.

Titen tools are available, but the Git-origin project is not registered.
Project recall could not proceed.
Current source and hardware observations therefore define this design.

## Approach

Adopt the inspected Rust release and preserve the useful custom behavior.
Keep the upstream core and runtime boundaries.
Replace the primary device presentation with the supplied design.
Keep mixer, scenes, effects, setup, and diagnostics accessible through application navigation.
Do not replace the working audio runtime with a new audio server.

Porting the Python application would require rebuilding features already present upstream.
A new Go implementation would add hardware and audio integration work.
The selected approach reduces that work and keeps future upstream changes reviewable.

## Repository identity and history

Rename the existing GitHub repository to `RamaAditya49/cadiswave`.
Preserve its history, issues, stars, visibility, and repository settings.
Check the destination name before the rename.
Rename the local directory to `/home/ramaaditya/Project/cadiswave`.
Check that the destination directory does not exist.

Use `origin` for Rama's renamed repository.
Use `upstream` for `https://github.com/rikkichy/openwave.git`.
Preserve the five custom commits in history.
Integrate the pinned upstream history through a reviewed merge.
Do not force-push, reset history, or discard unrelated work.
Recheck the working directory before integration.

Use the following identities consistently:

| Item | Identity |
| --- | --- |
| Application | CadisWave |
| Desktop command | `cadiswave` |
| Capture service command | `cadiswave-daemon` |
| Diagnostic command | `cadiswave-diag` |
| Engineering command | `cadiswave-probe` |
| Private maintenance command | `cadiswave-maintenance` |
| User service | `cadiswave.service` |
| Application ID | `io.github.RamaAditya49.CadisWave` |
| Configuration directory | `$XDG_CONFIG_HOME/cadiswave` |
| Data directory | `$XDG_DATA_HOME/cadiswave` |
| Runtime resources | `cadiswave_` prefix |

Use standard XDG defaults when environment variables are absent.
Keep hardware names, USB identifiers, upstream credits, and historical references unchanged.
Record the upstream revision in maintained documentation.
Retain the original copyright notice.
Add attribution for CadisWave changes without replacing original ownership.

## Module boundaries

| Module | Responsibility |
| --- | --- |
| `cadiswave-core` | Device capabilities, protocol validation, state types, routing rules, and pure calculations |
| `cadiswave-runtime` | USB ownership, audio workers, command handling, persistence, service control, and diagnostics |
| `cadiswave-desktop` | GTK widgets, gestures, translations, notifications, tray controls, and application navigation |

The desktop submits typed commands and consumes runtime snapshots.
The runtime captures device identity when it accepts a hardware command.
Selecting another device must not redirect queued commands.
The runtime reports command completion separately from command admission.
The desktop must not perform synchronous USB, process, filesystem, or audio graph operations.

Add focused presentation modules for the device page, knob, meters, settings, compact controls, and status bar.
Keep localization and theme values in dedicated modules and resources.
Reuse existing controller boundaries for mixer and scene functions.
Do not put all new behavior in the application entry point.
Keep pure value conversion and capability rules independently testable.

## Visual system

Use GTK4 and libadwaita for native Linux integration.
Use custom GTK CSS and drawing for the supplied device presentation.
Do not use an embedded browser for the primary interface.

The design uses these values:

| Element | Reference |
| --- | --- |
| Window | 1280 by 800 logical pixels |
| Main background | `#0b0c0d` |
| Panel background | Approximately `#1c1e1f` |
| Main text | `#f2f4f3` |
| Secondary text | `#8b928f` |
| Accent | `#7dff9b` |
| Error and mute | `#ff7b84` |
| Pending state | `#ffd166` |
| Panel corners | Approximately 20 logical pixels |
| Device surface corners | Approximately 30 logical pixels |
| Outer content margin | Approximately 40 logical pixels |

Use one consistent icon family.
Use tabular figures for changing numbers.
Use monospace text for diagnostic details and keyboard hints.
Use local fonts with a system fallback.
Bundle additional fonts only with their distribution license.
Do not load fonts, scripts, or images from the network at runtime.

Build the knob, LED arc, icons, and surfaces as original application resources or drawing code.
Do not copy files from the design archive into the repository.
Keep source images, reference HTML, support scripts, screenshots, and extraction directories outside the repository.
Commit only application resources and maintained documentation.

Use adaptive layouts instead of fixed screen coordinates.
At the reference size, preserve the three-column composition and spacing.
At smaller sizes, stack panels in a scrollable layout.
Validate 1280 by 800, 1024 by 768, and 800 by 600 layouts.
Validate normal and high display scaling.
Indonesian text must not overlap values or controls.

## Main device page: artboard 2a

The header contains branding, connection state, settings, About, and reconnect actions.
Use native window controls and a draggable header.
Connection states distinguish discovery, connected, reconnecting, disconnected, and unavailable observations.

The left column contains input meters, input level, gain, and observed dial mode.
The center contains the device surface, mute strip, large value, virtual knob, LED arc, and hardware indicators.
The right column contains headphone controls and capture status.
The lower bar contains the latest service event and keyboard hints.
The event bar expands into a bounded diagnostic view.

The mute strip and `M` shortcut submit the same mute command.
Dragging the knob changes the supported value for the current observed dial mode.
Arrow keys change that value in its supported step size.
`Ctrl+R` requests device discovery without restarting the global audio server.
Shortcuts must not intercept text entry in dialogs.
Controls have keyboard access, focus feedback, and accessible names.

Hardware dial mode is an observation, not an assumed writable setting.
Mode buttons indicate the physical mode when that mode is known.
Do not report a mode change unless the device confirms it.
When the mode has no verified writable value, disable knob edits and explain the limitation.
Show unknown hardware values as unavailable.
Do not invent numeric values for an unsupported mix mode.

The 48 V indicator reflects observed state.
Normal UI startup, scenes, migration, and testing must not change phantom power.

## Desktop integration: artboard 2b

The tray menu provides mute, compact controls, full application access, and quit.
The compact controls use the same runtime commands and snapshots as the main window.
Opening compact controls must not start a second USB owner.
Without a tray host, keep the application window accessible.

Use desktop notifications for connection transitions.
Send notifications for observed transitions, with duplicate suppression.
Show capture readiness only after the runtime confirms it.
Do not label capture as active because a child process merely exists.

Show dial feedback within the application when visible.
Use a supported desktop notification fallback when external OSD positioning is unavailable.
Wayland compositor policy determines external window placement.
Do not promise GNOME Quick Settings integration without a shell extension.
Keep the compact control composition consistent with the reference.

## Device settings and save states: artboards 3a and 3b

Retain the settings sheet structure and visual hierarchy.
Each control has explicit capability and operation state.
Display unsupported rows with translated explanations.
Do not submit unknown USB offsets or persistence commands.

| Reference feature | CadisWave behavior |
| --- | --- |
| Clipguard | Show an unavailable hardware control until its protocol is verified |
| Hardware low-cut | Show an unavailable hardware control until its protocol is verified |
| Software low-cut | Expose the existing PipeWire effect separately and identify its software scope |
| LED color and brightness | Show unavailable hardware controls until support is verified |
| Sample rate | Show observed rate; enable changes only after target-specific negotiation and restoration pass tests |
| Hardware monitor mix | Enable only for a verified profile capability; Wave XLR `007d` currently lacks a mapped value |
| Save to device | Show an unavailable action until a verified persistence command exists |
| Save application settings | Use the existing validated configuration store |

Keep application persistence separate from hardware persistence.
Do not call application settings saved to the device.
Successful USB writes confirm a live change, not persistence after power loss.
Display saving, saved, failed, and retry states for operations with implemented persistence.
Use real completion events for those states.
Do not simulate progress or success with a timer.

Use typed operation errors and preserve exact technical details in the expandable error view.
Translate the surrounding explanation and recovery action.
Revert applies only to known settings with an implemented operation.
An error must not clear an unresolved pending state.

## Sample-rate confirmation: artboard 3c

Show the requested rate and the expected stream interruption before an enabled rate change.
Cancellation leaves the audio graph and saved settings unchanged.
Scope negotiation to the selected device and its owned capture path.
Preserve previous state for restoration if negotiation fails.
Require a confirmed resulting rate and capture readiness before showing success.
Do not change the global PipeWire clock or restart unrelated audio services.
If the target cannot be controlled safely, retain the read-only rate display and explain the limitation.
Do not claim a fixed interruption duration before measuring it.

## Capture and synchronization

Retain capture keepalive, device reconnect, default-input restoration, and hardware synchronization behavior.
Audit each existing custom commit against the Rust implementation.
Record which behavior is already covered and which requires a port.
Verify the hardware gain-to-ALSA conversion for the connected `007d` device.

Show service running state, capture readiness, and synchronization state separately.
Replace the reference's legacy environment toggles with supported runtime settings where equivalent behavior exists.
Explain an unavailable equivalent instead of writing unused configuration.
Read service events through a bounded worker.
Stop and drain that worker when its owner closes.

Automatic disruptive recovery remains an explicit user setting.
Mute, silence, or unavailable observations alone must not trigger recovery.
Recovery uses bounded attempts and the upstream identity checks.
Ordinary startup must not cycle a card profile or restart the user's audio session.

## English and Bahasa Indonesia

Use Fluent message catalogs for `en` and `id`.
Keep message keys independent from visible text.
Support System, English, and Bahasa Indonesia language choices.
Use English when the system locale has no supported match.
Persist the choice in the application UI settings.

A language change updates open desktop views without restarting the audio runtime.
It must not recreate routes, reconnect USB, or overwrite configuration.
Do not change the process-wide locale while workers run.
Use application-owned locale bundles for presentation.

Translate labels, dialogs, tooltips, accessibility text, tray actions, notifications, and user-facing status explanations.
Keep device names, commands, identifiers, and exact technical errors unchanged.
Keep stable settings values and remote action names language-independent.
Use complete messages with named values instead of translated sentence fragments.
Use concise, natural Indonesian.
Apply the repository's technical English rules to authored English prose.

Test catalog key parity, argument parity, parsing, and English fallback.
Test persisted language selection and live language changes.
Verify each reference screen in both languages.

## Responsiveness and lifecycle

Render only changed snapshot values.
Keep recent user gestures separate from stale observations.
Coalesce continuous edits per control and device.
Always submit the final gesture value.
Keep polling and command completion responsive during continuous edits.

Use frame-clock animation for visual interpolation.
Target smooth motion at a 60 Hz display rate.
Stop animation callbacks when widgets are hidden or destroyed.
Respect disabled desktop animation settings.
Show the confirmed numeric value separately from visual interpolation.

Compute meters from verified PCM measurements.
Do not normalize unknown vendor meter integers as calibrated dBFS.
Do not draw two independent channels from a mono measurement.
Add a stereo measurement path when both channels can be verified.
Use a measured attack and release presentation policy.
Stop meter redraws after levels settle or the page becomes hidden.

Keep worker queues, event buffers, diagnostics, and child-process output bounded.
Use cancellation and bounded drain behavior for disconnect and shutdown.
Retain upstream ownership checks and failure reporting.
Repeated page, language, and compact-window changes must not leak workers.

## Migration and installation

Preserve the old checkout revision and its recoverable Git history.
Keep existing user configuration available until migration is verified.
Use validated schemas for optional configuration import.
Do not overwrite existing CadisWave settings with legacy defaults.
Do not mutate the old application's settings during import.

Detect existing application and service ownership before activation.
Do not run OpenWave and CadisWave vendor-control owners together.
Prepare and verify the new binaries before any service replacement.
Document installation, upgrade, and recovery procedures.
Do not describe repository preparation as a completed runtime migration.

## Public maintenance requirements

Provide an English README and an Indonesian README.
Document dependencies, source builds, device support, current limits, and contribution checks.
Maintain architecture, localization, and upstream-origin documentation.
Include contribution instructions and a security reporting policy.
Keep public issue reports free of unreviewed diagnostic dumps.

Keep the Rust lockfile and a verified toolchain declaration.
Use formatting, Clippy, relevant tests, and isolated desktop checks in CI.
Preserve the useful upstream installation and cleanup tests.
Update packaging, service files, desktop entries, icons, and AppStream metadata consistently.
Do not leave obsolete Python packaging as the primary installation path.
Preserve historical source through Git instead of shipping a second runtime.

End every new commit with the required CADIS attribution trailer.
Do not add model attribution or generated-content footers.
Commit no mockup files, archives, reference screenshots, or local audit output.

## Verification and acceptance

Build the complete workspace with the verified toolchain and native dependencies.
Run formatting and relevant Clippy checks.
Run core and runtime tests.
Run isolated GTK callback and installation checks.
Run the preserved upstream regression checks affected by identity changes.
Validate desktop entries and AppStream metadata.

Compare actual desktop renders with every reference artboard.
Inspect layout, typography, spacing, controls, state colors, and supported transitions.
Keep comparison screenshots outside the repository.
Exercise the main page, settings, About, compact controls, errors, and disconnected states in both languages.
Confirm that no control reports simulated hardware success.

Measure drag responsiveness, physical-dial feedback, redraw behavior, and idle resource use.
Record the test environment with each measurement.
Target normal gesture feedback within one display frame.
Target normal physical-dial feedback within two successful polling cycles.
Report actual results instead of claiming guaranteed latency.

On the connected Wave XLR, verify readback and supported controls under a single application owner.
Preserve the user's current gain, volume, mute, and phantom state after controlled tests.
Verify capture byte flow and microphone usability through the actual desktop audio session.
Verify reconnect and service ownership through the supported lifecycle.
Do not count device-free tests as physical hardware verification.
Do not change phantom power for a demonstration.

Verify the renamed GitHub repository and final remote URLs.
Verify the final local directory and clean Git state.
Inspect the tracked file list for excluded reference material.
Report implemented controls, unsupported controls, test results, and any remaining runtime verification limits.

## Evidence references

- [Inspected upstream release](https://github.com/rikkichy/openwave/tree/4172c71db3e0b929571d02c06bbcb3456726e5e2)
- [Upstream architecture](https://github.com/rikkichy/openwave/blob/4172c71db3e0b929571d02c06bbcb3456726e5e2/docs/ARCHITECTURE.md)
- [Upstream protocol](https://github.com/rikkichy/openwave/blob/4172c71db3e0b929571d02c06bbcb3456726e5e2/docs/protocol.md)
- [Upstream device profiles](https://github.com/rikkichy/openwave/blob/4172c71db3e0b929571d02c06bbcb3456726e5e2/crates/openwave-core/src/profiles.rs)
- [Fluent Rust implementation](https://github.com/projectfluent/fluent-rs)
- [GTK drawing area](https://docs.gtk.org/gtk4/class.DrawingArea.html)
- [libadwaita adaptive layouts](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/1-latest/adaptive-layouts.html)

The supplied reference was extracted and rendered outside the repository.
No reference file is part of this specification commit.
