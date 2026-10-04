# Hardware support

CadisWave enables exact VID:PID profiles, not a family-wide match on the word “Wave”. All listed USB IDs use Elgato vendor ID `0fd9`. The native authority is [`cadiswave_core::profiles::PROFILES`](../crates/cadiswave-core/src/profiles.rs), selected by `profile_for_usb`; unsupported IDs have no production protocol profile.

| Product / variant | PID | Software scope |
|---|---|---|
| Wave XLR | `007d` | Enabled XLR profile: 34-byte config, 10-byte meter, 51-byte device info |
| XLR Dock / Wave XLR MK.2, `00a6` variant | `00a6` | Enabled XLR-layout profile; product naming does not cover other Dock/MK.2 PIDs |
| Wave:3 | `0070` | Enabled microphone profile: 16-byte config, 8-byte meter, 64-byte device info |
| XLR Dock MK.2 | `00c7` | Enabled three-block vendor protocol; read-only `0103` / `0203` bank detection |
| Any other PID, including `00b6` | Other | Not enabled by these profiles |

“Enabled” means the implementation contains that exact profile, not that every firmware or multi-unit configuration has passed physical acceptance testing. The `00c7` implementation follows [OpenXLR's recorded hardware findings](https://github.com/emaspa/openxlr/blob/03bdae51b47fd01b599230957537474ec66b7cc9/docs/hardware-support.md) and its author's [Dock verification report](https://github.com/emaspa/openxlr/issues/1#issuecomment-5549540393). Those are upstream hardware results, not physical certification of this native backend. The original Wave XLR (`007d`) passed [physical checks](verification.md#physical-acceptance).
Other profiles passed protocol checks with device-free GTK fixtures.
No physical `00c7` run was performed.

## Controls

| Control | `007d` / `00a6` profile | `0070` profile | `00c7` profile |
|---|---|---|---|
| Microphone gain | Q8.8 dB; `007d`: up to 75 dB; `00a6`: up to 80 dB | Q8.8 dB, up to 40 dB | Integer dB, 0–80 dB |
| Mute | Yes | Yes | Yes |
| Headphone volume | Signed Q8.8 dB | Signed Q8.8 dB | −60 to 0 dB, quarter-dB steps |
| 48 V phantom power | Yes, config byte 6 | No | Yes, settings byte 1 bit 1 |
| Low impedance mode | Yes | No | Yes |
| Clipguard | Unmapped | API `5.3` / `5.4` only, config byte 5; physical acceptance pending | Yes, settings byte 2 inverted bit 2; upstream verified |
| Hardware low cut | Unmapped | Unavailable | Yes, settings byte 1 bit 4; upstream verified |
| LED color and brightness | Unmapped | Conflicting evidence; unavailable | Unmapped |
| Explicit save to device | Unmapped | Unmapped | No separate command; upstream observed retained gain |
| Microphone/PC monitor mix | `007d` only: LE word 12, 0 = PC, `0x6400` = mic; `00a6` unavailable | Yes, 0–100% | Yes, 0–200 raw (0 = mic, 200 = PC) |
| Knob mode, firmware/API/serial, meters | Profile-defined | Profile-defined | No physical knob; USB descriptor serial; firmware/API unavailable; PCM meters |

Controls absent from a profile are hidden, not emulated. Profile capabilities also govern ALSA synchronization; ALSA control names/ranges are discovered instead of assuming fixed numeric control IDs. [`cadiswave_core::protocol`](../crates/cadiswave-core/src/protocol.rs) validates profile fields and exact identity matches; [`cadiswave_runtime::device`](../crates/cadiswave-runtime/src/device.rs) owns USB access and ALSA synchronization.

The Dock's known banks are tested using reads of all three documented blocks. A stall or short block allows trying the other bank; disconnects and timeouts do not. A transient USB I/O read failure is retried once on the same bank, based on upstream streaming observations. If neither bank returns every complete block, connection fails without enabling writes. The chosen bank stays fixed until disconnect. Only explicitly changed blocks are written, preserving unrelated DSP flags and reserved bytes. Dock controls use the vendor protocol directly; CadisWave does not apply the legacy ALSA mirror scales to this UAC2 device.

Clipguard and hardware low cut use exact profile mappings.
Other onboard DSP and EQ controls remain unavailable.
Existing CadisWave source effects use host PipeWire processing.
Firmware/API offsets and vendor meter blocks for `00c7` are not assumed from another model.

The [pinned OpenXLR report](https://github.com/emaspa/openxlr/blob/219dc11d3a0d6390044d504ee75d9637706e17fa/docs/hardware-support.md#L129-L215) records Dock processing tests.
It identifies original Wave XLR processing, monitor mix, hardware-save, and LED controls as unmapped in OpenXLR.
The `00a6` profile does not inherit the Dock MK.2 processing fields.
The `00c7` low-cut control is Boolean; no selectable frequency is mapped.
The Dock retains gain after an upstream power-cycle test, but no separate save command is available.
CadisWave has not physically verified Dock processing or persistence.

Original Wave XLR monitor support uses separate [WaveController source evidence](https://github.com/oparada1988/WaveController/blob/23498e526a0fd3ea9991edaf07170b5abfb76878/wavecontroller/engine/elgato_wave.py#L152-L185).
Its author [reports Linux physical knob changes at bytes 12–13](https://github.com/oparada1988/WaveController/commit/44644946189b21fc454118a206b2d64e4202245f).
The upstream report provides no retained raw capture or firmware scope.
CadisWave has not physically verified this monitor field.
The `00a6` profile retains no monitor mapping.
WaveController's original Clipguard, low-cut, and RGB fields remain candidates.
No retained physical effect result supports enabling those fields here.
Its architecture document conflicts with its executable transfer and offset definitions.

Wave:3 Clipguard follows [LibreWave's recovered API 5.3/5.4 schema](https://github.com/zhgmx/LibreWave/blob/2d81fde497ef57e893f8b1510b2777dbe3a75381/docs/protocol-evidence.md#L29-L51).
Physical effect verification remains pending.
The controller admits Clipguard only for observed API `5.3` or `5.4`.
The device owner reads metadata again before the config write.
Unknown APIs and incomplete metadata prevent the entire batch from writing config.
Other API versions remain unavailable for Clipguard writes.
Other firmware variants remain unverified.
Wave:3 RGB and brightness claims conflict with that schema and [published physical observations](https://github.com/LukasParke/wave3-research/blob/35e85646f30b2a61d68fadbd360428286f907882/native-linux/tests/probe-wave3-config.log#L408-L515).
Those fields remain unchanged by CadisWave.

**48 V is a real hardware write.** Verify the selected unit, cable and microphone manufacturer's instructions before enabling it. Some microphones and connected equipment must not receive phantom power. Reduce monitoring levels before changing power or connecting equipment. A scene deliberately cannot toggle phantom power. Do not use raw probe writes to bypass these precautions.

## Multiple units and hotplug

- Discovery distinguishes connected units by profile and USB bus/address. The selector displays serial where available, otherwise the runtime USB location. Bus/address can change after replugging and is not a durable serial substitute.
- `DeviceManager` gives each connection incarnation an independent serialized worker for connection, polling, read-modify-write controls and disconnect. An operation captures its `UnitId`; changing the selection cancels unsent debounced device edits but does not transfer queued work to another unit. Retirement drains that unit's queue before disconnect.
- ALSA pairing uses exact USB VID:PID and bus/device identity, with exact serial evidence where available. Ambiguity must not become “first Elgato card wins”. If identity cannot be established safely, hardware/ALSA synchronization is unavailable rather than aimed at an arbitrary card.
- Known unrelated USB products are filtered before requiring their ALSA bus metadata; damaged unrelated hotplug records do not select or block a valid exact target. Ambiguous or incomplete possible targets still fail closed.
- Temporarily empty/partial ALSA controls are rediscovered on the existing ALSA polling interval without reopening the USB unit. Identity is checked around discovery and before writes; complete controls stop rediscovery, and unknown controls never use guessed numeric IDs.
- The capture manager keeps one pin per Wave input. Adding or removing one input must not tear down the remaining inputs' pins.
- Saved scene hardware entries resolve by exact serial. A legacy model-only entry is eligible only when that model has exactly one connected candidate. A missing or ambiguous device is reported; it is not substituted.

Capture source rows bind to their selected PipeWire node names. Two units of the same model still need distinct source bindings; a sidebar selection is not permission to silently replace another source's hardware identity.

## Reporting an unknown device

Include the USB VID:PID, reported product name, firmware if known, kernel and PipeWire versions, and the symptom. Start with [privacy-reduced diagnostics](troubleshooting.md#diagnostics-and-privacy). Do not add an unverified PID to udev/profile lists just to make the application connect, and do not probe unknown offsets as routine troubleshooting.

Engineers can consult the [protocol reference](protocol.md#engineer-only-probe) and native [`cadiswave_runtime::probe`](../crates/cadiswave-runtime/src/probe.rs) for transport details and the limitations of the single-device probe. It selects the first supported unit in bus/address order, not a user-specified unit. Close every vendor-control client before any explicit USB diagnostic read, and connect only one supported unit when investigating a specific target.
