# Elgato Wave control protocol

This is an engineering reference for the profiles enabled in CadisWave, derived from reverse engineering rather than a vendor specification. It describes implemented offsets, not a physical-validation certificate. The exact profile table is [`cadiswave_core::profiles::PROFILES`](../crates/cadiswave-core/src/profiles.rs); [`cadiswave_core::protocol`](../crates/cadiswave-core/src/protocol.rs) validates and encodes supported fields, and [`cadiswave_runtime::device::VendorDevice`](../crates/cadiswave-runtime/src/device.rs) owns native USB transfers. See [hardware support](hardware-support.md) for the exact PID scope and cautions.

## Transport and ownership

The legacy `007d`, `00a6` and `0070` configuration protocols use USB Class control transfers on endpoint 0:

| Field | Read | Write |
|---|---|---|
| `bmRequestType` | `0xA1` (class/interface/IN) | `0x21` (class/interface/OUT) |
| `bRequest` | `0x85` | `0x05` |
| `wValue` | Block selector | Block selector |
| `wIndex` | `0x3303` | `0x3303` |

Wave Link's `0x3300` addresses interface 0, owned by Linux's `snd-usb-audio`. The legacy profiles use `0x3303`, retaining the protocol's `0x33` prefix while targeting interface 3. CadisWave does not detach the audio driver for these transfers. The `00c7` Dock uses the separate vendor protocol below; neither dialect is a guarantee that arbitrary transfers cannot disrupt hardware.

Only one process should own vendor transfers to a unit. A competing GUI, diagnostic collector or probe can produce `-EIO`/read failures. Quit CadisWave **including its tray process** before probing or using diagnostics with `--device`. Native vendor-control clients acquire `Lease::vendor_control`; the lease does not make an unrelated third-party client safe to run concurrently. Within the runtime, `DeviceManager` gives each captured `UnitId` a serialized queue for polling and writes. Selection changes cannot retarget queued work, and retirement drains the queue before disconnect. This ownership must not be bypassed by another USB client.

Writes are whole-block read-modify-write operations: read the profile's config, patch a supported field, write the block. Preserve all other bytes. No offset is safe merely because a similarly named product uses it.

## Blocks and encodings

All multibyte fields below are little-endian.

| Block | `wValue` | `007d` / `00a6` length | `0070` length |
|---|---|---|---|
| Config | `0x0000` | 34 bytes | 16 bytes |
| Meter | `0x0001` | 10 bytes | 8 bytes |
| Device info | `0x000A` | 51 bytes | 64 bytes |

The meter begins with two unsigned 32-bit raw levels; their PCM normalization is unverified. Native UI meters use captured PCM instead of treating these vendor integers as linear gain. Device-info API version is at bytes 0–1. XLR-profile firmware is at 6–8 and serial at 27–46; Wave:3 firmware is at 21–23 and serial at 36–47.

### XLR profiles: `0fd9:007d` and `0fd9:00a6`

| Offset | Type | Field |
|---|---|---|
| 0 | uint16 | Gain: raw / 256 dB; maximum `0x4b00` (75 dB) for `007d`; `0x5000` (80 dB) for `00a6` |
| 4 | byte | Mute: `1` muted, `0` live |
| 6 | byte | 48 V phantom: `1` on, `0` off |
| 9 | int16 | Headphone level: raw / 256 dB; zero is unity |
| 12 | uint16 | `007d` monitor mix only: Q8.8 percent, 0 = PC, `0x6400` = microphone |
| 14 | byte | Knob mode: `1` gain, `2` headphones; `3` monitor mix on `007d` only |
| 33 | byte | Low impedance mode: `1` on, `0` off |

The `00a6` profile uses this layout. It does not apply to the `00c7` Dock described below, `00b6` or every device sold as a Dock/MK.2.
The monitor field and mode 3 do not apply to `00a6`.

The original monitor field follows [WaveController's pinned exact profile](https://github.com/oparada1988/WaveController/blob/23498e526a0fd3ea9991edaf07170b5abfb76878/wavecontroller/engine/elgato_wave.py#L152-L185).
Its author [reports Linux physical knob observations at bytes 12–13](https://github.com/oparada1988/WaveController/commit/44644946189b21fc454118a206b2d64e4202245f).
The [direction correction](https://github.com/oparada1988/WaveController/commit/a885daec07f326a9086f7d28e4051d2dfc856a31) identifies 0 as PC-only and `0x6400` as microphone-only.
CadisWave preserves the raw Q8.8 value and patches only bytes 12–13.
It does not copy WaveController's simultaneous mute or LED changes.
The upstream report has no retained raw capture or firmware version.
CadisWave has no physical acceptance result for this monitor field.
These limits do not authorize other WaveController fields or other PIDs.

### Wave:3: `0fd9:0070`

| Offset | Type | Field |
|---|---|---|
| 0 | uint16 | Gain: raw / 256 dB; maximum `0x2800` (40 dB) |
| 4 | byte | Mute: `1` muted, `0` live |
| 5 | byte | Clipguard: `1` enabled, `0` disabled |
| 7 | int16 | Headphone level: raw / 256 dB |
| 10 | uint16 | Microphone/PC mix: raw / 256 percent; maximum `0x6400` (100%) |
| 12 | byte | Dial mode: `1` gain, `2` headphones, `3` monitor mix |

No phantom or low-impedance offset is defined for Wave:3.
Clipguard uses the field in [LibreWave's recovered API 5.3/5.4 schema](https://github.com/zhgmx/LibreWave/blob/2d81fde497ef57e893f8b1510b2777dbe3a75381/crates/librewave-protocol/src/wave3.rs#L237-L262).
[OpenXLR](https://github.com/emaspa/openxlr/blob/219dc11d3a0d6390044d504ee75d9637706e17fa/docs/hardware-support.md#L239-L295) uses the same Clipguard field.
CadisWave has no physical Wave:3 Clipguard acceptance result.
Clipguard admission requires the observed API to equal `5.3` or `5.4`.
Other API versions remain unavailable for Clipguard writes.
The controller checks the observed API before dispatch.
The device owner reads the API again before any batch containing a Wave:3 Clipguard write.
An unsupported API or incomplete metadata read prevents every config write in that batch.
The recovered schema does not establish a broader firmware compatibility range.
All unlisted bytes remain unknown or reserved.

Wave:3 LED color and brightness mappings conflict with the recovered schema.
The [published test log](https://github.com/LukasParke/wave3-research/blob/35e85646f30b2a61d68fadbd360428286f907882/native-linux/tests/probe-wave3-config.log#L408-L515) does not confirm color or brightness effects.
CadisWave preserves those bytes and offers no writes for them.
Hardware low cut remains unavailable on this profile.

### XLR Dock MK.2: `0fd9:00c7`

Protocol evidence comes from OpenXLR's [hardware report](https://github.com/emaspa/openxlr/issues/1#issuecomment-5549540393), [documented bank variants](https://github.com/emaspa/openxlr/blob/03bdae51b47fd01b599230957537474ec66b7cc9/docs/hardware-support.md) and [MK.2 protocol implementation](https://github.com/emaspa/openxlr/blob/03bdae51b47fd01b599230957537474ec66b7cc9/src/OpenXLR.Core/Devices/WaveXlrMk2Device.cs). CadisWave independently implements those wire facts; no upstream implementation code is included.

Reads use `bmRequestType=0xC1`, writes use `0x41`, and both use `bRequest=0x01`. The `wValue` selects a block; `wIndex` is the bank. Only vendor interface 3 is claimed, without driver detach, alternate-setting changes or device reset.

| Block | Selector | Exact length | Implemented fields |
|---|---|---|---|
| Input settings | `0x0004` | 38 bytes | Byte 0: integer gain, 0–80 dB. Byte 1: mute bit 0, phantom bit 1, low cut bit 4. Byte 2: Clipguard disabled bit 2 |
| Headphones | `0x0005` | 2 bytes | Byte 0: attenuation in quarter-dB steps, 0–240 (0 to −60 dB). Byte 1: low-impedance bit 1 |
| Hardware monitor mix | `0x0001` | 6 bytes | Byte 0: 0–200, microphone-only to PC-only |

Connection reads all three blocks at `0x0103`, trying `0x0203` only after a stall or incomplete block. Each bank must answer every exact length; otherwise the handle closes and no writes are enabled. Other errors propagate rather than authorizing another bank. A USB I/O read error gets one retry on the same bank, based on upstream observations during streaming. Writes are never automatically retried. The chosen bank is fixed for the connection and detected afresh after reconnect. Complete-length reads are a necessary admission check, not independent proof of an unknown firmware layout.

Production controls read all three blocks, preserve every unmodified byte/bit, and write only blocks changed by the requested settings. No commit command is sent. A batch spanning blocks is serialized but not hardware-atomic: if a later transfer fails, earlier writes may have succeeded and the next poll observes the actual state.

Hardware low cut changes only byte 1 mask `0x10`.
Clipguard clears byte 2 mask `0x04` to enable and sets it to disable.
The [pinned OpenXLR implementation](https://github.com/emaspa/openxlr/blob/219dc11d3a0d6390044d504ee75d9637706e17fa/src/OpenXLR.Core/Devices/WaveXlrMk2Device.cs#L237-L287) defines both fields.
Its [hardware report](https://github.com/emaspa/openxlr/blob/219dc11d3a0d6390044d504ee75d9637706e17fa/docs/hardware-support.md#L207-L215) records Dock effect verification.
The low-cut field is Boolean; no selectable cutoff frequency is mapped.
CadisWave has no physical acceptance result for these fields.

The Dock applies writes directly and [retained gain after an upstream replug test](https://github.com/emaspa/openxlr/blob/219dc11d3a0d6390044d504ee75d9637706e17fa/src/OpenXLR.Core/Devices/XlrDockMk2Device.cs#L4-L17).
No separate save command is mapped.
Block `0x0003` stalls, so CadisWave does not send a commit packet.
Retention of every DSP field on every firmware remains unverified.

The serial comes from the USB string descriptor. Firmware/API versions and vendor meter layouts are not mapped; USB `bcdDevice` is not presented as a firmware version. UI metering still uses PCM. There is no physical knob. Unexposed onboard DSP flags remain unchanged by ordinary controls, and scenes never request phantom power.

CadisWave's native implementation has passed device-free protocol, transport and GTK checks; it has not been physically exercised on a `00c7` unit here. Upstream records hardware verification at `0x0103`, with `0x0203` compatibility owner-reported.

## Engineer-only probe

Use the native [`cadiswave-probe`](../crates/cadiswave-runtime/src/probe.rs) executable with dependencies and USB permissions already configured. From a source checkout, first build the sibling executables with `cargo build --locked --workspace --bins`, then use `target/debug/cadiswave-probe` in place of `cadiswave-probe` below. The probe has no per-unit selection flag: it connects to the first supported unit in bus/address order. **Use only one connected supported unit when investigating a specific device**, and verify the printed model, VID:PID and bus/address before proceeding. Do not use it as a multi-device control interface.

Read-oriented commands:

```sh
cadiswave-probe dump
cadiswave-probe watch --interval 0.1
```

`dump` reports expected versus returned lengths for config/meter/device-info blocks. `watch` prints changed byte offsets while you move one physical control at a time; Ctrl+C stops it. Dumps can contain serials and are not privacy-redacted. Custom `dump --wvalue 0xN --len N` requests are protocol research, not a general device-health check.

For `00c7`, `dump` reads the three blocks above; unknown selectors are rejected and requested reads are capped to the documented lengths. `watch` and explicitly confirmed `poke` operate on the 38-byte input-settings block only. They do not treat the headphone or monitor blocks as legacy meters/device information.

**`poke` writes hardware. Even `poke --noop` sends a full config write.** Its unchanged payload can still trigger firmware side effects; it is not a read-only verification command. The interface is `poke --offset N --byte VALUE`, or `poke --noop` without offset/byte. Both forms prompt for confirmation unless `--yes` is supplied. The config is read after consent, then written and read back for byte-for-byte verification. Do not write unknown offsets, copy byte numbers across profiles, or demonstrate writes by toggling phantom power. Disconnect sensitive equipment and establish the exact target/layout before any controlled write experiment.

A successful unchanged write does not establish that unknown fields are safe or that another PID is compatible. New hardware support requires independently reviewed identity, block and field evidence before enabling writes.

## Capture identity

Native device workers use the USB descriptor serial to match capture nodes.
Vendor metadata supplies firmware and API versions.
A missing descriptor serial leaves capture identity unknown.

## Write confirmation

The device worker reads the hardware configuration after each successful write.
It compares requested fields with the normalized configuration.
A transfer acknowledgment alone does not establish confirmation.
A mismatch returns an unavailable result without replaying the write.
