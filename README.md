<p align="center">
  <img src="https://raw.githubusercontent.com/RamaAditya49/cadiswave/main/icons/cadiswave.png" alt="CadisWave icon" width="96" height="96">
</p>
<h1 align="center">CadisWave</h1>
<p align="center"><strong>Control your Wave on Linux.</strong></p>
<p align="center">Microphone controls, audio routing, and a native desktop for daily Wave XLR use.</p>
<p align="center">
  <a href="https://github.com/RamaAditya49/cadiswave/actions/workflows/tests.yml"><img src="https://github.com/RamaAditya49/cadiswave/actions/workflows/tests.yml/badge.svg?branch=main" alt="Tests"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-7dff9b?labelColor=141a17" alt="MIT license"></a>
  <img src="https://img.shields.io/badge/platform-Linux-7dff9b?labelColor=141a17" alt="Linux">
</p>
<p align="center"><strong>Rust · GTK4 · libadwaita · PipeWire</strong></p>
<p align="center">
  <a href="README.id.md">Bahasa Indonesia</a> ·
  <a href="#build-and-run">Build and run</a> ·
  <a href="CONTRIBUTING.md">Contribute</a> ·
  <a href="docs/verification.md">Test results</a>
</p>

![CadisWave device dashboard with input meters, gain dial, and headphone controls](docs/images/device-en.png)

<p align="center"><sub>Actual GTK interface, captured with controlled test data. Meter values show a test fixture.</sub></p>

## Made for your daily audio controls

| Feature | What you can do |
| --- | --- |
| Device dashboard | Read input levels, adjust gain, toggle mute, and control headphones. |
| Compact controls | Keep key controls nearby. Use the tray when your desktop provides a tray host. |
| PipeWire mixer | Manage application sources, independent mixes, outputs, scenes, and software effects. |
| English and Indonesian | Change the interface language without restarting audio or USB workers. |
| Confirmed device state | Read confirmed hardware values. Keep pending edits separate from actual device state. |
| Capture monitoring | Check capture byte activity and connection status. Enable disruptive recovery explicitly. |

## A closer look

<table>
  <tr>
    <th>Compact controls</th>
    <th>Application settings</th>
  </tr>
  <tr>
    <td align="center"><img src="docs/images/compact-en.png" alt="CadisWave compact controls" height="400"></td>
    <td align="center"><img src="docs/images/settings-footer-en.png" alt="CadisWave settings with language selection and spaced save buttons" height="400"></td>
  </tr>
</table>

These screenshots show the application with controlled GTK fixtures. They are not design mockups.
See [screenshot provenance](docs/images/README.md) for capture details.

## Device support

| Device | USB ID | Mapped controls | CadisWave physical checks |
| --- | --- | --- | --- |
| Wave XLR | `0fd9:007d` | Gain up to 75 dB, mute, phantom power, headphones, low impedance | Tested; see [results](docs/verification.md) |
| XLR Dock | `0fd9:00a6` | Gain, mute, phantom power, headphones, low impedance | Not tested here |
| Wave:3 | `0fd9:0070` | Gain, mute, headphones, monitor mix | Not tested here |
| XLR Dock MK.2 | `0fd9:00c7` | Gain, mute, phantom power, headphones, monitor mix, low impedance | Not tested here |

Mapped controls have protocol tests. This does not establish physical acceptance for every model or firmware.
Read [hardware support](docs/hardware-support.md) before using another device.

Clipguard, hardware low-cut, LED changes, hardware persistence, and sample-rate changes remain unavailable.
Original Wave XLR hardware monitor mix is not mapped.
Use Mixer for software low-cut, gate, compression, EQ, delay, and mono effects.
Unknown hardware dial modes disable dial edits.

## Build and run

Use **Rust 1.98.1**, GTK **4.14+**, libadwaita **1.5+**, libusb 1.0, and pkg-config.
Runtime audio uses PipeWire, pipewire-pulse, WirePlumber, ALSA utilities, and SWH LADSPA plugins.

On Ubuntu 24.04, install the dependencies:

```bash
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev libusb-1.0-0-dev \
  pipewire pipewire-pulse wireplumber alsa-utils pulseaudio-utils swh-plugins
rustup toolchain install 1.98.1 --component rustfmt --component clippy
```

Build and install as your login user:

```bash
git clone https://github.com/RamaAditya49/cadiswave.git
cd cadiswave
cargo build --release --locked --workspace --bins
make install PREFIX="$HOME/.local"
"$HOME/.local/bin/cadiswave"
```

Open **Settings** to select a language. Open **Mixer** for sources, effects, scenes, and device selection.

| Shortcut | Action |
| --- | --- |
| `M` | Toggle mute |
| Arrow keys on the dial slider | Adjust the selected value |
| `Ctrl+R` | Reconnect |

Run `install.sh` as your login user for a system installation.
The installer stages and checks files before requesting administrator access.
See [hardware setup](docs/hardware-support.md), [Bazzite installation](docs/install-bazzite.md), and [troubleshooting](docs/troubleshooting.md).

Enable **Follow hardware mute** in a processed capture source editor to synchronize mute with the selected Wave device.

## Move from OpenWave

Keep the old installation until the new build passes its checks.
Stop the legacy `openwave.service` before granting CadisWave vendor control.
CadisWave rejects a second vendor owner. Optional import preserves existing settings.
Follow the [migration and recovery guide](docs/migration.md).

The desktop runtime owns USB synchronization, including hidden tray operation.
The capture daemon maintains input streams. It does not replace the desktop USB owner.

## Contribute

Start with the [contributor guide](CONTRIBUTING.md).
Read the [architecture](docs/ARCHITECTURE.md), [protocol reference](docs/protocol.md), and [localization guide](docs/localization.md).
Use [GitHub issues](https://github.com/RamaAditya49/cadiswave/issues) for bugs and feature requests.
Include your device model, USB ID, and reproduction steps.

## People and license

**Developed and maintained by [Rama Aditya](https://github.com/RamaAditya49) (CADIS).**

CadisWave is an independent project derived from [OpenWave](https://github.com/rikkichy/openwave) by rikkichy and contributors.
CadisWave implementation and original application artwork: Rama Aditya (CADIS) and contributors.
The [MIT license](LICENSE) retains the original copyright.
CadisWave is not an Elgato product.

Design archives and mockup files remain outside this repository.
The screenshots above are application documentation.
