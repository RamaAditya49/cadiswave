<p align="center"><img src="icons/cadiswave.svg" alt="CadisWave" width="112"></p>
<h1 align="center">CadisWave</h1>

Native Linux controls for Elgato Wave devices. Built with Rust, GTK4, libadwaita, and PipeWire.

[Bahasa Indonesia](README.id.md) · [Build and contribute](CONTRIBUTING.md) · [Architecture](docs/ARCHITECTURE.md) · [Migration](docs/migration.md)

CadisWave is an independent MIT project derived from [OpenWave](https://github.com/rikkichy/openwave). CadisWave is not an Elgato product.

## Features

- Device dashboard with measured input channels, a physical dial view, mute, headphone volume, and connection status.
- English and Bahasa Indonesia. Change the language without restarting audio or USB workers.
- Compact controls and a system tray. The full window remains available without a tray host.
- PipeWire mixing, application sources, independent mixes, output selection, scenes, and software effects.
- Serial-bound device commands, confirmed readback, bounded meter queues, and owned worker cleanup.
- Capture byte monitoring and observation-only health checks. Disruptive recovery requires an explicit setting.

## Hardware support

| Device | USB ID | Mapped controls |
| --- | --- | --- |
| Wave XLR | `0fd9:007d` | Gain up to 75 dB, mute, phantom power, headphones, low impedance |
| XLR Dock | `0fd9:00a6` | Gain, mute, phantom power, headphones, low impedance |
| Wave:3 | `0fd9:0070` | Gain, mute, headphones, monitor mix |
| XLR Dock MK.2 | `0fd9:00c7` | Gain, mute, phantom power, headphones, monitor mix, low impedance |

Only mapped protocol controls can submit commands. Physical validation results are recorded separately from device-free tests.

Clipguard, hardware low-cut, LED changes, hardware persistence, and sample-rate changes remain unavailable.
Use Mixer for software low-cut, gate, compression, EQ, delay, and mono effects.
The hardware knob mode is an observation. Unknown modes disable dial edits.

## Build

Use Rust 1.98.1, GTK 4.14+, libadwaita 1.5+, libusb 1.0, and pkg-config.
Install PipeWire, pipewire-pulse, WirePlumber, ALSA utilities, and SWH LADSPA plugins for runtime audio features.

On Ubuntu 24.04:

```bash
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev libusb-1.0-0-dev \
  pipewire pipewire-pulse wireplumber alsa-utils pulseaudio-utils swh-plugins
rustup toolchain install 1.98.1 --component rustfmt --component clippy
cargo build --release --locked --workspace --bins
make install PREFIX="$HOME/.local"
```

Run `cadiswave`. Open Settings to select a language. Open Mixer for sources, effects, scenes, and device selection.
Press `M` to toggle mute. Use the dial slider's arrow keys for precise edits. Press `Ctrl+R` to reconnect.

Run `install.sh` as your login user for a system installation.
The installer stages and checks files before requesting administrator access.
See [hardware setup](docs/hardware-support.md) and [troubleshooting](docs/troubleshooting.md).

Use **Follow hardware mute** in a processed capture source editor to synchronize its mute with the selected Wave device.

## Migration and ownership

Keep the old installation until the new build passes its checks.
Stop the legacy `openwave.service` before granting CadisWave vendor control.
CadisWave rejects a second vendor owner and preserves existing settings during optional import.
See [migration and recovery](docs/migration.md).

The desktop runtime owns USB synchronization, including hidden tray operation.
The capture daemon maintains input streams. It does not replace the desktop USB owner.
No design archive, reference image, or mockup runtime is included in this repository.

## Credits

OpenWave and its protocol work: rikkichy and contributors.
CadisWave implementation and original application artwork: CADIS and contributors.
The [MIT license](LICENSE) retains the original copyright.
