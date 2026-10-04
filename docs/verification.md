# CadisWave verification

Verified locally on 2026-10-04 with Rust 1.98.1.
The public repository is [RamaAditya49/cadiswave](https://github.com/RamaAditya49/cadiswave).
The original repository identity and history remain intact.
Device settings work uses branch `feat/device-settings` from `0300a4f`.
Origin points to CadisWave; upstream points to rikkichy/OpenWave.
The host runs Zorin OS 18.1, GTK 4.14.5, libadwaita 1.5.0, and PipeWire 1.0.5.
Build dependencies came from an extracted Ubuntu development SDK.
The installed application uses the host runtime libraries.

## Base revision checks

| Check | Result |
| --- | --- |
| Cargo formatting | Passed |
| Clippy, all workspace targets, warnings denied | Passed |
| Workspace tests | 420 passed; 43 marked ignored |
| Isolated GTK tests | All 26 display tests passed |
| Release binaries and runtime examples | Built with the locked dependency set |
| Installed application smoke | Passed with private audio, GTK, D-Bus, and device isolation |
| Desktop entry and AppStream | Validated |

The remaining 17 ignored cases need disposable privileged installations or RPM tools.
They are not local passes.
CI runs the separate private RPM ownership check.

Smoke checks cover actual PCM levels, mute, scenes, output selection, graph repair, server replacement, and worker shutdown.
They also cover tray-host loss, host return, and installed application actions.
Device-free checks do not establish physical hardware support.

The final review found three important issues.
Four failing GTK regressions reproduced them before correction.
Compact readings now follow the confirmed dial mode.
Device controls have translated accessible labels.
Calibration measurements and recovery messages use Fluent parameters.
Exact backend errors remain unchanged.
The review dialog changes language without replacing its response generation.
All four regressions and the complete software checks passed after correction.
No minor review findings remain deferred.
The final native Wayland installation retained the default input, mute, and 75 dB gain.
Live accessibility inspection confirmed the dial, headphone, and low-impedance control names.

The settings footer has 16 pixels above its actions and 12 pixels between actions and status text.
Geometry checks passed in English and Indonesian.
Idle status text is hidden; pending save feedback appears immediately.
Rama Aditya appears in About, both READMEs, AppStream, package metadata, and the license.
The original copyright remains intact.
An initial CI smoke failed on a retired GTK accessibility object during a diagnostic tree walk.
The smoke driver now skips that specific retired-object error and preserves other errors.
Regression checks and the complete installed smoke passed locally after correction.

## Physical acceptance

These physical checks predate the new device settings controls.
They do not establish physical acceptance for the new monitor mapping.

The connected original Wave XLR identifies as `0fd9:007d`.
Only one desktop runtime owns vendor control.
The capture daemon owns capture streams separately.

| Check | Observed result |
| --- | --- |
| Gain, mute, headphones, low impedance | Independent USB readback confirmed each reversible change |
| USB confirmation time | 7, 8, 8, and 9 ms for those four controls |
| GTK gain edit | 75 to 74.5 dB; ALSA confirmation in 259 ms; 75 dB restored |
| Processed input mute | GTK action reached capture in 437 ms for unmute and 481 ms for mute |
| Processed microphone audio | 204800 bytes and 102393 nonzero samples during a two-second collection after first data |
| Reconnect | Device, mono capture readiness, default input, and mute recovered |
| Language | English to Indonesian saved during active capture without replacing the USB owner |
| Quit and native Wayland restart | Original default input, mute, and capture readiness retained |

GTK gain confirmation includes the 200 ms edit debounce.
Processed mute measurements include runtime scheduling and the external confirmation probe.
USB transfer acknowledgment alone does not establish hardware confirmation.
The worker reads the configuration after each write and rejects mismatched fields.
No write replay occurs after a mismatch.

The tests restored gain, mute, headphone level, and low impedance.
Phantom power remained unchanged.
The existing processed default input remained selected.
PipeWire, pipewire-pulse, WirePlumber, and the existing processing service retained their invocation identities during reconnect.
No microphone recording, private device serial, or host audio configuration is included here.

## Resources and visual checks

A visible native Wayland window was measured for ten seconds with ongoing capture and device monitoring.
Desktop CPU averaged 11.70 percent of one CPU core.
Its owned process group averaged 20.93 percent of one CPU core.
Desktop resident memory was 201.7 MiB.
The process group reported 112.8 MiB of charged memory.
Resident memory includes shared pages; these memory measurements must not be added.
This is one local observation, not a performance guarantee.

Native layouts were inspected at 1280×800, 1024×768, and 800×600 logical pixels.
Compact controls, both settings languages, controlled confirmation states, and a scale factor of two were inspected.
Reference material and screenshots remain outside Git.
Physical-dial timing and display-frame latency remain unmeasured.

## Device settings checks

The device settings change adds direct software low-cut, built-in voice presets, and named presets.
Microphone tests record one to ten seconds of raw capture in bounded memory.
Playback requires an explicit output identity. Recordings are discarded after cancellation.
Snapshots and public actions expose metadata, not PCM.

The final private installed test passed three complete microphone cycles and the full routing smoke.
Each cycle recorded 48,000 stereo frames at 48 kHz.
Its peak was -12.0412 dBFS, with no clipped samples.
Playback used `fixture_output`, then discard cleared the session.
The test confirmed that no microphone child or node remained.
Two further private runs passed 20 recording, playback, and discard cycles each.
Each recording retained the same frame count, peak, and selected playback output.
The input signal came from a private synthetic tone.
No host microphone, host audio route, or USB device was accessed.

All 485 workspace tests passed. The workspace reported 59 ignored cases.
All 42 private GTK cases passed, including the new settings and geometry tests.
The other 17 ignored cases remain outside these local passes.
Formatting, workspace lint, locked release binaries, and locked runtime examples passed.
English and Indonesian layouts were inspected at 390, 480, and 640 pixels.
A shorter low-cut hint keeps the Indonesian selected value readable.
A final independent review found no remaining important issues.

Review corrections cover output replacement, close before recording publication, and shared recording ownership.
They also cover selected-device removal, Wave:3 API admission, and deferred GTK notifications.
Each important correction has a regression test.

Ephemeral audio nodes can change between the multiple PipeWire discovery queries.
This previously expired an unchanged microphone session during recording or playback.
The controller now retains exact known identities during a transient global discovery failure.
The worker still checks live cookie, serial, media class, and channels before and after each audio operation.
Known replacements and worker validation failures still expire the session.

One initial workspace run failed a process cleanup assertion.
The failure did not recur during 30 test repetitions.
A deterministic test showed that the test helper treated the kernel's dead `X` state as live.
The helper now accepts documented terminal states and reads termination once.
The original failing process transition was not captured.
Production process cleanup remains unchanged.

One privilege test encountered Busy after its owner released an installation lease.
A deterministic fork test showed that a child retains the shared lock descriptor until close or exec.
The fixture now retries only Busy within its existing deadline after the owner releases the lease.
Other errors still fail immediately. Production installation lease behavior remains unchanged.

A setup fixture also used a five-second event deadline around durable file operations.
Delaying one successful fsync by six seconds reproduced its timeout.
The fixture now uses a 30-second total deadline for each setup attempt.
The delayed case and normal cases pass. Production setup behavior remains unchanged.

One smoke attempt passed PCM measurement, then GNU timeout reported SIGKILL during recorder cleanup.
The original recorder shutdown delay was not captured.
The final full smoke passed with unchanged recorder assertions and without concurrent workspace checks.

Final local evidence remains outside Git:

- Workspace log: `/tmp/cadiswave-settings-final-tests.log`.
- Lint log: `/tmp/cadiswave-settings-final-clippy.log`.
- GTK log: `/tmp/cadiswave-settings-final-gtk.log`.
- GTK gallery: `/tmp/cadiswave-settings-gtk.vuPIP4pO/evidence/`.
- Private installed smoke: `/tmp/cadiswave-settings-smoke.Vq8nn5Wb/evidence/`.
- Microphone stress run: `/tmp/cadiswave-smoke.4z2aNCYY/evidence/`.

The initial window now fits saved dimensions within 90 percent of the current logical monitor dimensions.
The application keeps manual resizing and maximized state, with a 1024 by 720 minimum.
The narrow layout places the gain value beside a compact knob and keeps primary controls visible.
The initial 640 by 480 and 800 by 600 layouts passed before Rama requested a larger minimum.
The revised 1024 by 720 minimum passed geometry, preference, and GTK viewport checks.
Smaller saved sizes and resize requests now use the larger minimum.

## Current limits

LED changes, explicit device persistence, and sample-rate changes remain unavailable.
Capture rates identify observed formats separately from configured properties.
Original Wave XLR monitor mix uses pinned Linux source evidence; local physical acceptance remains pending.
Clipguard supports Dock MK.2 and Wave:3 API 5.3/5.4. Hardware low-cut supports Dock MK.2.
Original Wave XLR Clipguard and hardware low-cut remain unavailable.
Other mapped models passed device-free checks but were not physically tested here.
Read [hardware support](hardware-support.md) for exact profiles and evidence limits.

Feature implementation used release payloads staged and tested in a private namespace.

## Device settings host update

Rama authorized the host update after the feature implementation on 2026-10-04.
The user-prefix installation now contains the tested `eb77ee9` binaries.
The desktop and capture daemon run those binaries; process executable hashes match the release build.
The installer check passed against the new installation receipt.

The update retained a private backup of the previous payload and configuration outside Git.
The capture service stopped during file replacement, then restarted.
PipeWire, pipewire-pulse, and WirePlumber retained their invocation identities.
The default input, default output, input mute, input volume, and application configuration remained unchanged.
Writable ALSA controls remained unchanged.
The read-only playback channel map changed from inactive values to stereo when the desktop opened its output stream.

The live settings dialog exposes sensitive software low-cut, built-in presets, Record, and monitor mix controls.
AT-SPI inspection checked the current desktop PID, excluding stale accessibility entries.
The new microphone actions are available on the actual session bus.
The public runtime snapshot is readable. No host microphone recording was started.
The unmapped original Wave XLR controls remain unavailable.

The legacy Python source and configuration remain recoverable outside the active native source tree.
See [migration](migration.md) for activation, processed capture mapping, and rollback instructions.

## Selected application icon

Rama selected the mint Signal C artwork on 2026-10-04.
Both READMEs display the selected PNG.
The installed SVG includes the exact selected PNG data.
Its viewport removes empty outer padding.
Small tray icons use the C and waveform motif in white, black, or mute red.

The original installed GTK theme did not find `cadiswave` and returned `image-missing`.
Production now registers its supplied artwork directory before creating windows.
The regression failed before correction and passed afterward.
The gallery no longer adds an independent icon-path workaround.
All 26 GTK cases passed against the staged application assets.
The installed About gallery displays the selected icon and Rama Aditya.

A private notification service received the actual GTK connection notification.
Its image path points to the supplied `cadiswave.svg`.
Desktop entries identify the application with `StartupWMClass`.
The installer refreshes the desktop icon cache when its tool is available.
The live desktop theme now resolves the installed CadisWave artwork.

Formatting, lint, all 420 workspace tests, release builds, metadata, and installed smoke passed.
The 43 ignored workspace cases include the 26 separately executed GTK cases.
The remaining 17 cases are not local passes.

The tested release and artwork were installed in the user prefix.
The native Wayland application was restarted and its About dialog opened.
The capture daemon retained its process and invocation identities.
Default input, mute, ALSA controls, source configuration, and source snapshots remained unchanged.
The previous application payload remains backed up outside Git.
