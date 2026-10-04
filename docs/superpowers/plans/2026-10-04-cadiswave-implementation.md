# CadisWave Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans for native execution. Use superpowers:subagent-driven-development for explicitly selected agent execution. Track each step with its checkbox.

**Goal:** Prepare the public CadisWave repository and a responsive, bilingual Rust desktop application based on the approved device design.

**Architecture:** Preserve the upstream core, runtime, and desktop boundaries. Extend typed runtime observations and commands before connecting new GTK controls. Keep reference material outside Git.

**Tech Stack:** Rust, GTK4, libadwaita, libusb, ALSA, PipeWire, WirePlumber, Fluent, Cargo, and GitHub Actions.

**Spec:** [Approved design](../specs/2026-10-04-cadiswave-design.md)

Status: implementation, physical activation, repository rename, and main publication are complete.

## Global constraints

- Use CadisWave as the application name.
- Use `cadiswave` as the repository, directory, and desktop command name.
- Support English and Bahasa Indonesia throughout the desktop interface.
- Keep the design archive, reference images, and reference runtime outside the repository.
- Preserve the MIT license and upstream credits.
- The initial CadisWave Wave XLR gain control must retain the verified 75 dB limit.
- Do not change phantom power for a demonstration.
- Do not change the global PipeWire clock or restart unrelated audio services.
- Preserve the five custom commits in history.
- Do not force-push, reset history, or discard unrelated work.
- End every new commit with the required CADIS attribution trailer.
- Use `io.github.RamaAditya49.CadisWave` as the application ID.
- Use `cadiswave.service`, XDG `cadiswave` directories, and the `cadiswave_` runtime prefix.
- Target GTK 4.14+ and libadwaita 1.5+.
- Verify the pinned upstream Rust 1.98.1 toolchain before using its declared build contract.
- Keep the lockfile consistent with verified dependencies and the actual toolchain.
- Use at most 20 words per English instruction and 25 words per English description.

## Review focus

1. A device disconnect during a gesture must cancel unsent edits and reject stale completion events. Task 4 tests this condition.
2. Invalid legacy settings must remain intact and must not overwrite valid CadisWave settings. Task 2 tests this condition.
3. Unknown knob modes and nonfinite control values must not authorize gain writes. Tasks 3 and 4 test these conditions.
4. A language change during an active audio session must preserve routes, USB ownership, and pending commands. Tasks 5 and 9 test this condition.
5. Partial PCM frames, anti-phase stereo, and stale meter generations must not produce false channel readings. Task 6 tests these conditions.

## Execution and evidence locations

The current design branch is `cadiswave/prepare`.
Create an isolated implementation worktree through the using-git-worktrees skill.
Use a branch named `cadiswave/implementation` from the reviewed plan commit.
Keep the original checkout recoverable until integration passes its checks.
The final local directory is `/home/ramaaditya/Project/cadiswave`.

The inspected upstream revision is `4172c71db3e0b929571d02c06bbcb3456726e5e2`, tagged `v1.2.0`.
Fetch that revision from rikkichy instead of using an unpinned moving branch.
The current review checkout is outside the repository and is not a release input.

Create a private evidence directory with `mktemp -d -t cadiswave-evidence-XXXXXX`.
Store screenshots, performance measurements, staged installations, and reference renders there.
Keep the original design archive in Downloads.
Never copy its HTML, scripts, uploaded images, or cutout into the source tree.

Use read-only observations to locate the actual desktop session and existing installation.
Do not start another vendor-control owner during discovery.
Report unavailable session access instead of using a private test bus as physical acceptance evidence.

## File and dependency map

Existing upstream paths become the following CadisWave paths after Task 2:

| File or directory | Responsibility |
| --- | --- |
| `crates/cadiswave-core/src/model.rs` | Shared state, preferences, commands, and snapshots |
| `crates/cadiswave-core/src/profiles.rs` | Exact USB profiles and validated limits |
| `crates/cadiswave-core/src/protocol.rs` | Config decoding and supported writes |
| `crates/cadiswave-core/src/locale.rs` | Stable language preference and locale resolution |
| `crates/cadiswave-core/src/pcm.rs` | Channel-aware PCM decoding |
| `crates/cadiswave-runtime/src/controller.rs` | Commands, snapshots, and completion events |
| `crates/cadiswave-runtime/src/controller/state/mutations.rs` | Validated mutations and preference persistence |
| `crates/cadiswave-runtime/src/device.rs` | Serialized USB access and ALSA synchronization |
| `crates/cadiswave-runtime/src/meter.rs` | Identity-scoped meter workers |
| `crates/cadiswave-runtime/src/events.rs` | Bounded service-event reader |
| `crates/cadiswave-runtime/src/store.rs` | Atomic application-state persistence |
| `crates/cadiswave-runtime/src/paths.rs` | XDG paths, installation paths, and ownership leases |
| `crates/cadiswave-runtime/src/service.rs` | Capture service ownership and management |
| `crates/cadiswave-desktop/src/i18n.rs` | Locale bundles and translated presentation |
| `crates/cadiswave-desktop/src/ui/device/mod.rs` | Main device page composition |
| `crates/cadiswave-desktop/src/ui/device/projection.rs` | Snapshot-to-view conversion |
| `crates/cadiswave-desktop/src/ui/device/controls.rs` | Shared typed control actions |
| `crates/cadiswave-desktop/src/ui/device/knob.rs` | Accessible knob drawing and gestures |
| `crates/cadiswave-desktop/src/ui/device/meters.rs` | Channel meters and settling animation |
| `crates/cadiswave-desktop/src/ui/device/status.rs` | Connection, capture, and service status |
| `crates/cadiswave-desktop/src/ui/device/settings.rs` | Capabilities, settings, and operation feedback |
| `crates/cadiswave-desktop/src/ui/device/compact.rs` | Compact controls for the existing application owner |
| `crates/cadiswave-desktop/src/app.rs` | Lifecycle and navigation wiring |
| `crates/cadiswave-desktop/src/ui/matrix.rs` | Preserved mixer and effects access |
| `data/style.css` | Shared visual tokens and component styles |
| `data/locales/en.ftl`, `data/locales/id.ftl` | Complete interface messages |
| `icons/cadiswave*.svg` | Original application and tray artwork |
| `packaging/smoke-install.sh` | Isolated installed-application checks |
| `.github/workflows/tests.yml` | Required build and regression checks |

Tasks 1 and 2 establish the buildable base.
Tasks 3 and 4 establish hardware and presentation contracts.
Tasks 5 and 6 establish localization and measured input state.
Tasks 7 through 9 compose the interface and desktop integrations.
Task 10 verifies and integrates the complete result.

## Commit convention

Use a specific staged file list for every commit.
Run `git diff --cached --check` before committing.
Use this exact trailer after a blank line:

```text
Co-Authored-By: CADIS <agent@cadis.digital>
```

Each task ends with one verified commit or a small set of independently verified commits.
Do not amend historical commits to change their attribution.

---

### Task 1: Establish the pinned Rust baseline

**Files:**
- Integrate: upstream `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `crates/`, `data/`, `packaging/`, and native build inputs.
- Preserve: `docs/superpowers/` and all existing Git history.
- Create: `docs/upstream.md` and `docs/migration.md`.
- Test: existing upstream core, runtime, installation, and desktop tests.

**Interfaces:**
- Consume the existing branch at the reviewed plan commit.
- Produce a buildable upstream Rust workspace with preserved custom behavior requirements.
- Keep the existing `AppCommand`, `RuntimeHandle`, `RuntimeEvent`, and `AppSnapshot` contracts.

- [x] Create the isolated worktree with the using-git-worktrees skill.
- [x] Resolve lowercase Git origin through Titen before the first compile.
- [x] Compile bounded Titen context once when project resolution succeeds.
- [x] Continue safely without recall when the project remains unregistered.
- [x] Fetch the approved upstream release and verify its revision.

```bash
git fetch origin refs/tags/v1.2.0
git rev-parse FETCH_HEAD
git merge-base HEAD FETCH_HEAD
```

Expected revision: `4172c71db3e0b929571d02c06bbcb3456726e5e2`.

- [x] Inspect each custom commit before resolving the upstream merge.

```bash
git log --reverse --format='%h %s' 23f5a2b..9e96e5e
git diff 23f5a2b..9e96e5e -- wavexlr/device.py wavexlr/audio.py wavexlr/daemon.py
git merge --no-commit FETCH_HEAD
```

- [x] Resolve source conflicts using the Rust release as the active implementation.
- [x] Keep the approved specification and plan in the merge result.
- [x] Record coverage for each custom behavior in `docs/migration.md`.

Required rows: mute confirmation, physical gain sync, default input restoration, capture ordering, and capture byte-flow monitoring.
Each row identifies the Rust implementation, its regression test, or the owning task below.

- [x] Preserve the Python baseline in Git and retire obsolete active Python packaging after mapping its behavior.
- [x] Install required development packages without starting or restarting audio services.

```bash
sudo apt-get install build-essential pkg-config libgtk-4-dev libadwaita-1-dev libusb-1.0-0-dev
rustup toolchain install 1.98.1 --profile minimal --component rustfmt --component clippy
pkg-config --modversion gtk4 libadwaita-1 libusb-1.0
cargo test --locked --workspace
cargo build --locked --workspace --bins
```

If the declared toolchain is unavailable, verify the upstream minimum and lockfile compatibility before selecting an available official toolchain.
Update every exact toolchain check together after successful validation.
Do not describe a substituted toolchain as the upstream pin.

- [x] Record the working baseline, build prerequisites, and upstream revision in maintained documentation.
- [x] Commit the tested integration with the CADIS trailer.

### Task 2: Establish CadisWave identities and compatible state paths

**Files:**
- Rename: the three crate directories and executable source filenames from `openwave` to `cadiswave`.
- Modify: workspace manifests, imports, `paths.rs`, `service.rs`, `store.rs`, setup, installation, diagnostics, and release inputs.
- Rename: desktop files, AppStream metadata, original icons, audio integration filenames, and package files.
- Create: `crates/cadiswave-runtime/src/migration.rs`.
- Test: `crates/cadiswave-runtime/tests/paths_leases.rs`, `installation.rs`, `host_integration.rs`, and `migration.rs`.

**Interfaces:**
- Preserve `RuntimePaths::discover() -> Result<RuntimePaths>` and existing ownership checks.
- Produce `migration::inspect_legacy(source: &Path, destination: &Path) -> Result<LegacyImport>`.
- Produce `LegacyImport::apply(&self) -> Result<()>` for validated, missing destination files only.
- `LegacyImport` retains source paths, destination paths, and validated source bytes.

- [x] Add identity and migration regressions before changing runtime paths.

```rust
#[test]
fn application_paths_use_cadiswave_and_keep_legacy_files() {
    assert_eq!(
        cadiswave_runtime::paths::config_dir().unwrap().file_name().unwrap(),
        "cadiswave"
    );
    assert_eq!(
        cadiswave_runtime::paths::data_dir().unwrap().file_name().unwrap(),
        "cadiswave"
    );
}
```

Run this test in a private XDG environment.
Add a temporary-directory import test with malformed source JSON and an existing valid destination.
Assert unchanged source bytes, unchanged destination bytes, and a reported validation error.
Add a linked-path test that rejects import through symlink ancestry.

```rust
#[test]
fn malformed_legacy_state_remains_unchanged() {
    let legacy = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let source = legacy.path().join("sources.json");
    let original = b"[unterminated";
    std::fs::write(&source, original).unwrap();
    assert!(inspect_legacy(legacy.path(), destination.path()).is_err());
    assert_eq!(std::fs::read(source).unwrap(), original);
    assert!(!destination.path().join("sources.json").exists());
}

#[test]
fn existing_destination_is_never_replaced() {
    let legacy = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    std::fs::write(legacy.path().join("sources.json"), b"{}").unwrap();
    let target = destination.path().join("sources.json");
    let original = b"{\"keep\":true}";
    std::fs::write(&target, original).unwrap();
    inspect_legacy(legacy.path(), destination.path()).unwrap().apply().unwrap();
    assert_eq!(std::fs::read(target).unwrap(), original);
}
```

- [x] Run the new regressions and confirm failure against the old identity.
- [x] Rename active product identities using the file map.
- [x] Preserve hardware names, historical documentation, and original license ownership.
- [x] Add validated import inspection and exclusive destination creation.
- [x] Recheck source identity and destination absence when applying a previously inspected import.
- [x] Keep installer and uninstaller ownership checks consistent with the new identity.
- [x] Preserve cross-product USB exclusion for the same connected unit.

CadisWave must recognize an active legacy owner before opening vendor controls.
Changing the application prefix must not bypass that check.

```bash
cargo test --locked -p cadiswave-runtime --test paths_leases
cargo test --locked -p cadiswave-runtime --test installation
cargo test --locked -p cadiswave-runtime --test host_integration
cargo test --locked -p cadiswave-runtime --test migration
cargo build --locked --workspace --bins
```

- [x] Search active product files for old command, service, resource, and repository identities.
- [x] Keep each intentional legacy match documented.
- [x] Commit the tested identity change.

### Task 3: Validate Wave XLR limits and dial capabilities

**Files:**
- Modify: `crates/cadiswave-core/src/profiles.rs`, `protocol.rs`, and `lib.rs`.
- Create: `crates/cadiswave-core/src/capabilities.rs`.
- Modify: `crates/cadiswave-runtime/src/device.rs` only where conversion or gain mirroring requires correction.
- Test: `crates/cadiswave-core/tests/protocol.rs`, `hardware_support.rs`, and runtime `device_mirror.rs`.

**Interfaces:**
- Preserve `ConfigBuffer::decode(ProfileId, &[u8]) -> Result<ConfigBuffer>`.
- Preserve `ConfigBuffer::apply(DeviceSetting) -> Result<()>`.
- Produce `capabilities::for_profile(ProfileId) -> ControlCapabilities`.
- `ControlCapabilities` exposes hardware booleans for Clipguard, low-cut, LED, persistence, and monitor mix.
- Unsupported Wave XLR fields remain false.

- [x] Add exact original-device gain and unknown-mode regression cases.

```rust
#[test]
fn original_wave_xlr_gain_limit_is_75_db() {
    let mut config = ConfigBuffer::decode(ProfileId::WaveXlr, &[0; 34]).unwrap();
    config.apply(DeviceSetting::GainRaw(u16::MAX)).unwrap();
    assert_eq!(config.state().gain_raw, 0x4b00);
}

#[test]
fn unmapped_original_wave_xlr_mode_does_not_become_gain() {
    for mode in [3, 255] {
        let mut bytes = [0; 34];
        bytes[14] = mode;
        let config = ConfigBuffer::decode(ProfileId::WaveXlr, &bytes).unwrap();
        assert_eq!(config.state().knob_mode, KnobMode::None);
    }
}
```

- [x] Confirm failure against the inherited 80 dB limit and unknown-to-gain fallback.
- [x] Set `WaveXlr` gain maximum to `0x4b00` without changing other profiles through struct inheritance.
- [x] Decode only verified mode values; retain Wave:3's verified monitor mapping.
- [x] Update shared profile tests to assert each profile's own expected limit.
- [x] Verify ALSA gain mirroring with 0, half-dB steps, 75 dB, and out-of-range values.
- [x] Preserve untouched config bytes in every tested patch.

```bash
cargo test --locked -p cadiswave-core --test protocol
cargo test --locked -p cadiswave-core --test hardware_support
cargo test --locked -p cadiswave-runtime --test device_mirror
cargo test --locked -p cadiswave-runtime --test device_workers
```

- [x] Commit the validated original-device controls.

### Task 4: Define shared device projection and control actions

**Files:**
- Create: desktop `ui/device/projection.rs`, `controls.rs`, and `mod.rs`.
- Modify: desktop `ui/mod.rs` and `ui/test_support.rs`.
- Preserve: runtime command validation and per-unit debounce boundaries.
- Test: projection unit tests and isolated GTK callback tests.

**Interfaces:**
- Consume `AppSnapshot`, `UnitId`, `Observation<DeviceState>`, and `RuntimeHandle`.
- Produce `DeviceProjection::from_snapshot(&AppSnapshot) -> DeviceProjection`.
- Produce `DeviceControls::new(RuntimeHandle) -> DeviceControls`.
- Produce `DeviceControls::edit(&DeviceProjection, f64, EditTiming) -> Result<CommandId, ControlError>`.
- Produce `DeviceControls::toggle_mute(&DeviceProjection) -> Result<CommandId, ControlError>`.
- `ControlError` distinguishes unavailable state, unsupported mode, invalid value, and runtime rejection.

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceProjection {
    pub unit: Option<UnitId>,
    pub state: Option<DeviceState>,
    pub pending: Vec<DeviceSetting>,
    pub writable: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ControlError {
    Unavailable,
    UnsupportedMode,
    InvalidValue,
    Runtime(String),
}
```

- [x] Test default, missing, unknown, and selected-known snapshots before adding widgets.

```rust
#[test]
fn missing_observation_never_enables_controls() {
    let projection = DeviceProjection::from_snapshot(&AppSnapshot::default());
    assert_eq!(projection.unit, None);
    assert_eq!(projection.state, None);
    assert!(!projection.writable);
}
```

- [x] Extend the existing controlled GTK backend for gain and low-impedance fixture commands.
- [x] Add a gesture test that selects device A, starts an edit, then switches to device B.
- [x] Assert that queued work keeps device A's identity and unsent work is cancelled.
- [x] Add disconnect, nonfinite input, frozen lifecycle, and failed-completion cases.
- [x] Implement projection from the selected unit's known observation and pending intents.
- [x] Submit gain, headphones, or monitor commands only for their supported observed mode.

```rust
handle.submit(AppCommand::SetDeviceSetting {
    unit,
    setting,
    timing,
})
```

Use `EditTiming::Debounced` during a gesture and `EditTiming::Immediate` for its final value.
Do not force a final write after a cancelled gesture or retired unit.

- [x] Run projection tests and existing device worker and mutation regressions.
- [x] Run GTK callback tests through the isolated installed test runner.
- [x] Commit the shared control contracts.

### Task 5: Add complete English and Indonesian localization

**Files:**
- Create: core `locale.rs`, desktop `i18n.rs`, and `data/locales/en.ftl`, `id.ftl`.
- Modify: core `model.rs`, runtime preference mutations, desktop dialogs, sidebar, matrix, tray, actions, and application text.
- Modify: manifests and installation resource lists for verified Fluent dependencies and catalogs.
- Test: core `tests/locale.rs`, desktop localization tests, and runtime preference persistence tests.

**Interfaces:**
- Produce `LanguageChoice::{System, English, Indonesian}` with stable serialized values `system`, `en`, and `id`.
- Produce `LanguageChoice::resolve(self, system_locale: &str) -> &'static str`.
- Add `Preferences.language: LanguageChoice` with `System` as the legacy default.
- Add `PreferencesEdit.language: Option<LanguageChoice>`.
- Produce `I18n::new(LanguageChoice, &str) -> Result<I18n>`.
- Produce `I18n::text(&self, key: &str, args: Option<&FluentArgs<'_>>) -> String`.
- Produce `I18n::set_choice(&mut self, LanguageChoice, &str) -> Result<()>`.

- [x] Add locale-resolution and legacy-preference regressions.

```rust
#[test]
fn system_language_has_a_deterministic_fallback() {
    assert_eq!(LanguageChoice::System.resolve("id_ID.UTF-8"), "id");
    assert_eq!(LanguageChoice::System.resolve("en_US.UTF-8"), "en");
    assert_eq!(LanguageChoice::System.resolve("de_DE.UTF-8"), "en");
    assert_eq!(LanguageChoice::English.resolve("id_ID.UTF-8"), "en");
}
```

- [x] Test both Fluent catalogs for identical keys and named arguments.
- [x] Test malformed preferences without overwriting the original state file.
- [x] Confirm the new tests fail before adding locale support.
- [x] Add application-owned locale bundles without calling process-wide `setlocale`.
- [x] Inventory every authored interface string and add its catalog message.

```ftl
# English
app-name = CadisWave
device-connected = Connected
device-unavailable = Device unavailable
tap-to-mute = Tap to mute
hardware-control-unavailable = This hardware control is not available yet.

# Bahasa Indonesia
app-name = CadisWave
device-connected = Terhubung
device-unavailable = Perangkat tidak tersedia
tap-to-mute = Ketuk untuk membisukan
hardware-control-unavailable = Kontrol hardware ini belum tersedia.
```

Keep those messages in separate locale files.
Add full messages for source, mix, scene, effects, setup, installation, removal, errors, tray, status, and accessibility flows.
Keep technical errors unchanged beneath translated explanations.

- [x] Render existing open views again after a completed language-preference command.
- [x] Add a controlled-backend test that switches languages without issuing USB or routing commands.
- [x] Test preference serialization, English fallback, key parity, and translated tray updates.
- [x] Commit the complete localization boundary.

### Task 6: Provide measured stereo input and bounded service events

**Files:**
- Create: core `pcm.rs` and runtime `events.rs`.
- Modify: runtime `meter.rs`, controller event handling, and shared snapshot meter state.
- Modify: runtime `lib.rs` and core `lib.rs`.
- Test: core `tests/pcm.rs`, runtime `meter_lifecycle.rs`, and `tests/service_events.rs`.

**Interfaces:**
- Produce `ChannelPeaks::{Mono(f64), Stereo { left: f64, right: f64 }}` in core `pcm.rs`.
- Produce `PcmPeakDecoder::new(channels: u32) -> Result<PcmPeakDecoder>`.
- Produce `PcmPeakDecoder::push(&mut self, bytes: &[u8]) -> Option<ChannelPeaks>`.
- Add channel peaks to `MeterEvent` while retaining a compatible scalar peak for existing mixer rows.
- Add `AppSnapshot.channel_meters: Arc<IndexMap<String, ChannelPeaks>>`.
- Produce `EventTail::start() -> Result<EventTail>` and `EventTail::stop(&mut self) -> Result<()>`.
- Produce `EventTail::latest(&self) -> Vec<String>` with at most 200 entries.

- [x] Add channel-separation and partial-frame regressions.

```rust
#[test]
fn stereo_levels_do_not_cancel_each_other() {
    let mut decoder = PcmPeakDecoder::new(2).unwrap();
    let bytes = [0x00, 0x40, 0x00, 0xc0];
    assert_eq!(decoder.push(&bytes), Some(ChannelPeaks::Stereo {
        left: 0.5,
        right: 0.5,
    }));
}

#[test]
fn partial_frames_wait_for_the_remaining_channel() {
    let mut decoder = PcmPeakDecoder::new(2).unwrap();
    assert_eq!(decoder.push(&[0x00, 0x40]), None);
    assert_eq!(decoder.push(&[0x00, 0x20]), Some(ChannelPeaks::Stereo {
        left: 0.5,
        right: 0.25,
    }));
}
```

- [x] Confirm failure before replacing the mono-downmix decoder for device-page readings.
- [x] Preserve actual negotiated channel count and decode signed 16-bit PCM per channel.
- [x] Preserve the existing scalar mixer policy separately from the device page's channel measurements.
- [x] Preserve identity and worker-generation rejection before snapshot updates.
- [x] Preserve byte-flow readiness for zero bytes and incomplete PCM frames.
- [x] Keep a mono observation explicitly mono instead of inventing a right channel.
- [x] Match device-page meter observations to the selected unit through validated capture bindings.
- [x] Show unavailable readings when the selected capture binding is missing or ambiguous.
- [x] Use a bounded, coalesced channel mailbox instead of an unbounded high-rate channel queue.
- [x] Read service events with a cancellable child worker and bounded line storage.

Use `journalctl --user -u cadiswave.service -f -o cat --no-pager` only in the event worker.
Cap each line at 4096 bytes and keep the latest 200 lines.
Report missing journal access without marking the service failed.
On shutdown, cancel, terminate, and reap the owned child through the existing process owner.

- [x] Test invalid channels, EOF, oversized lines, stale generations, and repeated reader startup and shutdown.

```bash
cargo test --locked -p cadiswave-core --test pcm
cargo test --locked -p cadiswave-runtime --test meter_lifecycle
cargo test --locked -p cadiswave-runtime --test service_events
cargo test --locked -p cadiswave-runtime --test audio_pins
```

- [x] Commit measured input and bounded event handling.

### Task 7: Build the main hardware presentation

**Files:**
- Create: desktop `ui/device/knob.rs`, `meters.rs`, and `status.rs`.
- Modify: desktop `ui/device/mod.rs`, `app.rs`, and `data/style.css`.
- Modify: core preference geometry defaults and limits in `model.rs`.
- Create: original application and tray SVG assets under `icons/`.
- Test: device-page GTK callbacks, keyboard handling, rendering guards, and layout smoke checks.

**Interfaces:**
- Consume `DeviceProjection`, `DeviceControls`, `I18n`, and `ChannelPeaks`.
- Produce `DevicePage::new(DeviceControls, Rc<RefCell<I18n>>) -> DevicePage`.
- Produce `DevicePage::widget(&self) -> &gtk::Widget`.
- Produce `DevicePage::render(&self, &AppSnapshot)`.
- Produce `DevicePage::retranslate(&self)`.
- Produce `Knob::set_projection(&self, &DeviceProjection)` and `Meters::set_peaks(&self, Option<ChannelPeaks>)`.
- Produce `knob::interpolate(current: f64, target: f64, elapsed: Duration, animations: bool) -> f64`.

- [x] Add GTK callback tests for mute, drag completion, keyboard arrows, and text-entry shortcut exclusion.
- [x] Test that snapshot rendering emits no hardware commands.
- [x] Confirm the new page tests fail before adding the widgets.
- [x] Build the three-column composition with GTK containers and responsive breakpoints.
- [x] Set default geometry to 1280 by 800 and permit the required 800 by 600 layout.

```css
window.cadiswave {
  background: #0b0c0d;
  color: #f2f4f3;
}
.cadiswave-panel {
  background: #1c1e1f;
  border: 1px solid alpha(#ffffff, 0.07);
  border-radius: 20px;
  padding: 18px;
}
.cadiswave-device {
  border-radius: 30px;
  background: linear-gradient(170deg, #202224, #0f1011);
}
.cadiswave-accent { color: #7dff9b; }
.cadiswave-muted { color: #ff7b84; }
.cadiswave-pending { color: #ffd166; }
```

- [x] Draw the knob and 25-segment LED arc with `GtkDrawingArea` and Cairo.
- [x] Use a 270-degree sweep from -135 to 135 degrees.
- [x] Keep gesture input, confirmed numeric values, and visual interpolation separate.
- [x] Use the GTK frame clock only while visible values are moving.
- [x] Stop callbacks when hidden or destroyed and respect disabled animations.

Use this bounded interpolation rule for finite, normalized knob positions:

```rust
pub fn interpolate(current: f64, target: f64, elapsed: Duration, animations: bool) -> f64 {
    if !animations || (target - current).abs() < 0.0001 {
        return target;
    }
    let factor = 1.0 - (-elapsed.as_secs_f64() / 0.07).exp();
    current + (target - current) * factor
}

#[test]
fn disabled_animations_show_the_confirmed_position_immediately() {
    assert_eq!(interpolate(0.0, 0.5, Duration::from_millis(16), false), 0.5);
}
```

Reject nonfinite input before calling this function.
Remove the frame-clock callback when the returned position reaches its target.
- [x] Add native window controls, connection state, settings, About, and reconnect actions.
- [x] Add observed dial mode, 48 V indicator, headphone controls, and expanded event view.
- [x] Keep mixer, scenes, and effects accessible through native navigation.
- [x] Render artboard 2a in both languages and compare it with the external reference.
- [x] Inspect 1280 by 800, 1024 by 768, 800 by 600, and high-scale layouts.
- [x] Measure gesture redraw and confirm settled meters stop redrawing.
- [x] Commit the verified main presentation.

### Task 8: Build capability-aware settings and confirmed operation feedback

**Files:**
- Create: desktop `ui/device/settings.rs`.
- Modify: desktop `app.rs`, existing effect dialogs, localization catalogs, and shared style resources.
- Create: core `src/rate_change.rs` only if a target-scoped rate backend passes its admission checks.
- Test: settings GTK callbacks, persistence outcomes, and supported rate-change policy.

**Interfaces:**
- Consume `ControlCapabilities`, `DeviceControls`, `RuntimeEvent::CommandFinished`, and the existing configuration store.
- Produce `DeviceSettings::new(DeviceControls, Rc<RefCell<I18n>>) -> DeviceSettings`.
- Produce `DeviceSettings::render(&self, &AppSnapshot)` and `DeviceSettings::retranslate(&self)`.
- Produce `PersistenceState::{Idle, Pending(CommandId), Saved, Failed(String)}`.

- [x] Add a GTK test that clicks every unavailable hardware row and records zero submitted commands.
- [x] Add a failed-persistence test that retains unsaved state and displays the actual error.
- [x] Add a cancellation test that records zero rate or graph commands.
- [x] Confirm failure before creating the settings sheet.
- [x] Build the settings composition from artboard 3a using the verified capability table.
- [x] Keep Clipguard, hardware low-cut, LED writes, and hardware persistence unavailable for `007d`.
- [x] Expose software low-cut through the existing source effect controls with explicit software labeling.
- [x] Bind application save feedback to real persistence outcomes.

```rust
match outcome {
    CommandOutcome::Applied { .. } => PersistenceState::Saved,
    other => PersistenceState::Failed(format!("{other:?}")),
}
```

Use that transition only after a corresponding implemented application-state persistence command finishes.
Do not apply it to vendor live-write completion or command admission.

- [x] Inspect whether the selected capture supports scoped rate negotiation and restoration.
- [x] Keep sample rate read-only unless that backend has confirmed operation and rollback tests.

The admission checks require exact target identity, supported rate, captured previous state, confirmed resulting rate, and confirmed capture readiness.
If any check is unavailable, the rate button remains disabled with its translated explanation.
Render artboard 3c through a controlled backend without changing the real audio server.

- [x] Compare settings, pending, failed, and confirmation renders with artboards 3a, 3b, and 3c.
- [x] Test both languages, keyboard navigation, scrolling, and unknown observations.
- [x] Commit the verified settings and operation feedback.

### Task 9: Add compact controls and desktop lifecycle integration

**Files:**
- Create: desktop `ui/device/compact.rs`.
- Modify: desktop `tray.rs`, `app.rs`, `actions.rs`, and notification presentation.
- Modify: locale catalogs and relevant capture-status projections.
- Test: compact GTK callbacks, controlled runtime ownership, tray-host fallback, and notification suppression.

**Interfaces:**
- Consume the existing `RuntimeHandle`, `DeviceControls`, and shared locale bundle.
- Produce `CompactControls::new(DeviceControls, Rc<RefCell<I18n>>) -> CompactControls`.
- Produce `CompactControls::render(&self, &AppSnapshot)` and `CompactControls::retranslate(&self)`.
- Produce `ConnectionNotifier::observe(&mut self, &DeviceProjection) -> Option<gio::Notification>`.
- Produce `ConnectionNotifier::default()` with no prior observed connection.

- [x] Add a test that opens compact controls and verifies unchanged runtime and USB owner counts.
- [x] Add a test that switches language while a device command is pending.
- [x] Assert no graph rebuild, USB reconnect, or command cancellation from that language change.
- [x] Test repeated connected snapshots for zero duplicate notifications.

```rust
#[test]
fn repeated_connected_snapshots_do_not_repeat_notifications() {
    let unit = UnitId {
        profile: ProfileId::WaveXlr,
        bus: 1,
        address: 2,
        incarnation: 1,
    };
    let projection = DeviceProjection {
        unit: Some(unit),
        state: Some(ConfigBuffer::decode(ProfileId::WaveXlr, &[0; 34]).unwrap().state()),
        pending: Vec::new(),
        writable: true,
    };
    let mut notifier = ConnectionNotifier::default();
    assert!(notifier.observe(&projection).is_some());
    assert!(notifier.observe(&projection).is_none());
}
```
- [x] Confirm failure before wiring compact controls.
- [x] Add the artboard 2b compact composition using shared control actions.
- [x] Expose mute, compact controls, full window, and quit through the existing tray integration.
- [x] Preserve an accessible full window when the tray host is absent.
- [x] Use desktop notifications for confirmed connection transitions.

Build notifications from catalog messages and submit them through the existing application owner:

```rust
if let Some(notification) = notifier.observe(&projection) {
    application.send_notification(Some("cadiswave-device-connection"), &notification);
}
```
- [x] Use in-window dial feedback and supported notification fallback for external OSD.
- [x] Do not create a GNOME shell extension or assume tray coordinates are valid on Wayland.
- [x] Distinguish service state, capture byte readiness, and synchronization errors.
- [x] Connect supported capture settings to typed runtime preferences.
- [x] Label legacy synchronization options unavailable when no verified equivalent exists.
- [x] Drain compact-window resources and event workers during application shutdown.
- [x] Compare both-language compact renders with artboard 2b.
- [x] Commit the verified desktop integration.

### Task 10: Verify, document, and integrate the public repository

**Files:**
- Create or update: `README.md`, `README.id.md`, `CONTRIBUTING.md`, `SECURITY.md`, and maintained architecture, localization, migration, and upstream documents.
- Modify: CI, native packaging, desktop metadata, installer resource lists, and affected release inputs.
- Preserve: original license notices and the approved design and plan.
- Test: complete workspace, isolated installed application, packaging metadata, and physical device acceptance.

**Interfaces:**
- Consume all completed tasks and the tested implementation branch.
- Produce `RamaAditya49/cadiswave` with the verified branch history.
- Produce `/home/ramaaditya/Project/cadiswave` with consistent `origin` and `upstream` remotes.
- Produce a concise verification report that separates software tests from physical acceptance.

- [x] Add both READMEs with dependencies, source builds, supported controls, current limits, and credits.
- [x] Document translation checks and the required CADIS trailer for new project commits.
- [x] Document recovery, upgrade, legacy import, and diagnostic privacy behavior.
- [x] Update CI with the verified compiler, dependencies, locales, and renamed resource paths.
- [x] Run the required source checks once after the final source change.

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo build --release --locked --workspace --bins
cargo build --release --locked -p cadiswave-runtime --examples
cargo test --locked -p cadiswave-desktop --lib --no-run --message-format=json
```

- [x] Stage an installation under the private evidence directory.
- [x] Run the adapted `packaging/smoke-install.sh` against that installation and its GTK test binary.
- [x] Validate the installed desktop entry and AppStream metadata.

```bash
desktop-file-validate "$CADISWAVE_STAGE/usr/share/applications/cadiswave.desktop"
appstreamcli validate --no-net "$CADISWAVE_STAGE/usr/share/metainfo/io.github.RamaAditya49.CadisWave.metainfo.xml"
```

Set `CADISWAVE_STAGE` to the private staging directory for those commands.
Keep screenshots and test logs outside Git.

- [x] Inspect every artboard and relevant error state in both languages.
- [x] Measure software control confirmation, idle resources, and lifecycle behavior. Record physical-dial and display-frame timing as unmeasured.
- [x] Record actual measurements without describing a target as a measured result.
- [x] Locate the real desktop audio session and inspect existing OpenWave ownership.
- [x] Build and stage all binaries before any controlled service replacement.
- [x] Verify the connected `007d` under one vendor-control owner.
- [x] Test supported gain, mute, headphones, and low-impedance readback while preserving the original state.
- [x] Verify capture bytes and microphone usability through the actual audio session.
- [x] Verify reconnect and default-input restoration without restarting unrelated services.
- [x] Preserve existing user settings and recovery access throughout activation.

If physical session access remains unavailable, report that limit and retain the current working installation.
Do not claim completed runtime migration from isolated test results.

- [x] Check GitHub and local destination availability immediately before the authorized rename.
- [x] Rename the existing GitHub repository without creating a replacement repository.

```bash
gh api --method PATCH repos/RamaAditya49/openwave -f name=cadiswave
gh api repos/RamaAditya49/cadiswave --jq '{full_name, visibility, html_url}'
```

- [x] Rename the original checkout directory after verifying the destination does not exist.

```bash
mv /home/ramaaditya/Project/openwave /home/ramaaditya/Project/cadiswave
```

- [x] Update worktree administration paths with `git worktree repair` when required.
- [x] Set the renamed repository as `origin` and rikkichy as `upstream`.

```bash
git remote rename origin upstream
git remote rename rama origin
git remote set-url origin git@github.com:RamaAditya49/cadiswave.git
git remote -v
```

- [x] Recheck external branch state before pushing the verified implementation branch.
- [x] Integrate into `main` without a force-push after all required checks pass.
- [x] Use a normal merge when shared `main` advances. Shared `main` was unchanged; integration used a fast-forward.
- [x] Verify the pushed revision and the final local branch.
- [x] Scan tracked files for reference material, archives, private output, and obsolete active product identities.

```bash
git diff --check
git status --short
git ls-files
git log -1 --format='%h %s%n%b'
```

- [x] Report the repository URL, local path, revision, test results, unsupported controls, and physical verification state.

## Coverage map

| Approved specification area | Owning tasks |
| --- | --- |
| Rust base and custom behavior preservation | 1, 3, 6 |
| Repository identity, history, and local directory | 2, 10 |
| Core, runtime, desktop separation | 1, 4, 6, 7 |
| Visual system and adaptive layouts | 7, 8, 9 |
| Artboard 2a and keyboard controls | 3, 4, 6, 7 |
| Artboard 2b and tray integration | 4, 5, 9 |
| Artboards 3a and 3b | 3, 5, 8 |
| Artboard 3c and rate-change admission | 8 |
| Capture, readiness, and synchronization | 1, 3, 6, 9, 10 |
| English and Bahasa Indonesia | 5, 7, 8, 9, 10 |
| Smooth gestures and lifecycle ownership | 4, 6, 7, 9 |
| State migration and installation ownership | 2, 10 |
| Public maintenance and clean source tree | 2, 5, 10 |
| Device-free checks and physical acceptance | 3, 4, 6, 7, 8, 9, 10 |

## Execution choice

Recommend native execution in this session.
The tasks depend closely on shared runtime identities, snapshots, and translated views.
Native execution keeps these contracts consistent without repeated implementation handoffs.
A fresh final reviewer checks the completed branch after the required tests.

Agent execution remains an option if Rama explicitly selects it.
That method uses a fresh implementer and reviewer for each task.
Do not begin product implementation until the reviewed plan and execution method are accepted.
