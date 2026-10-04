# CadisWave verification

Verified locally on 2026-10-04 with Rust 1.98.1.
The host runs Zorin OS 18.1, GTK 4.14.5, libadwaita 1.5.0, and PipeWire 1.0.5.
Build dependencies came from an extracted Ubuntu development SDK.
The installed application uses the host runtime libraries.

## Software checks

| Check | Result |
| --- | --- |
| Cargo formatting | Passed |
| Clippy, all workspace targets, warnings denied | Passed |
| Workspace tests | 418 passed; 41 marked ignored |
| Isolated GTK tests | All 24 display tests passed |
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

## Physical acceptance

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

## Current limits

Clipguard, hardware low-cut, LED changes, device persistence, and sample-rate changes remain disabled.
Original Wave XLR hardware monitor mix is not mapped.
Software effects remain available through Mixer.
Other mapped models passed device-free checks but were not physically tested here.

The legacy Python source and configuration remain recoverable outside the active native source tree.
See [migration](migration.md) for activation, processed capture mapping, and rollback instructions.
