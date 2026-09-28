# OpenWave

OpenWave is an unofficial Linux control app and background service for the
Elgato Wave XLR USB audio interface.

It is not affiliated with, endorsed by, or supported by Elgato/Corsair. Wave XLR
does not have official Linux software, so this project implements the Linux
control path with standard USB, ALSA, PipeWire, WirePlumber, GTK4, and libadwaita
components.

## Features

- GTK4/libadwaita desktop app for Wave XLR controls
- Microphone gain and mute controls
- Headphone output volume control for the Wave XLR headphone jack
- Low impedance mode toggle
- Firmware/API/serial display
- Hardware mute sync to ALSA and PipeWire
- Hardware mic gain sync to ALSA while the dial is in mic gain mode
- Background capture keepalive to avoid the Wave XLR Linux capture race
- Default input restoration after device reconnect
- System tray support with quick mute
- First-run setup for USB permissions and the user systemd service

## Current Limitations

- The physical dial is not a programmable system volume knob.
- Headphone volume only affects the Wave XLR 3.5 mm headphone output.
- The dial mode is chosen on the device itself. Press the dial to cycle between
  mic gain, headphone volume, and mic/PC mix.
- Clipguard, low-cut, LED color, sample-rate switching, and save-to-hardware are
  not implemented yet.

## Requirements

- Linux with PipeWire and WirePlumber
- Python 3.10+
- pip, if installing from source with `python3 -m pip`
- GTK4 and libadwaita
- PyGObject
- libusb 1.0
- ALSA utilities
- `pkexec`/polkit for first-run USB permission setup

On Arch/CachyOS:

```bash
sudo pacman -S python python-pip python-gobject gtk4 libadwaita libusb pipewire wireplumber alsa-utils polkit
```

On Debian/Ubuntu-like systems, install the equivalent packages:

```bash
sudo apt install python3 python3-pip python3-gi gir1.2-gtk-4.0 gir1.2-adw-1 libusb-1.0-0 pipewire wireplumber alsa-utils policykit-1
```

## Install From Source

```bash
git clone https://github.com/RamaAditya49/openwave.git
cd openwave
python3 -m pip install --user .
openwave
```

On first launch, OpenWave prompts to install:

- a udev rule for Wave XLR USB permissions
- `openwave.service`, a user systemd service for capture/default-mic/hardware sync

## Run Without Installing

```bash
git clone https://github.com/RamaAditya49/openwave.git
cd openwave
python3 -m wavexlr
```

Start hidden in the tray:

```bash
python3 -m wavexlr -- --hide
```

## Desktop Entry

If installed with `pip`, copy the desktop entry manually:

```bash
mkdir -p ~/.local/share/applications
cp wavexlr.desktop ~/.local/share/applications/openwave.desktop
update-desktop-database ~/.local/share/applications 2>/dev/null || true
```

## Background Service

The user service runs:

```bash
python3 -c "from wavexlr.daemon import main; main()"
```

It keeps Wave XLR capture active, restores the Wave XLR as default input, and
mirrors confirmed hardware mute changes into PipeWire so browsers and apps see
the mute state. By default the first hardware mute read is treated as a
baseline, not immediately forced into PipeWire, and mute changes need two
matching reads before they are applied. This keeps the physical mute button
usable while reducing stale or noisy mute reads.

Full hardware control sync for gain and headphone volume remains opt-in:

```bash
systemctl --user edit openwave.service
# add:
# [Service]
# Environment=OPENWAVE_ENABLE_HARDWARE_SYNC=1
```

Useful mute-sync environment overrides:

```bash
# default: 1
Environment=OPENWAVE_ENABLE_MUTE_SYNC=1
# default: 0 for mute-only sync, 1 for full hardware sync
Environment=OPENWAVE_SYNC_INITIAL_MUTE=0
# default: 2
Environment=OPENWAVE_MUTE_CONFIRMATIONS=2
# default: 1.0 seconds
Environment=OPENWAVE_SYNC_INTERVAL=0.5
# default: 20 seconds
Environment=OPENWAVE_PROCESSED_SOURCE_GRACE=20
```

Useful commands:

```bash
systemctl --user status openwave.service
journalctl --user -u openwave.service -f
systemctl --user restart openwave.service
```

## How It Works

Wave XLR exposes audio through standard USB audio interfaces. Device settings are
read and written through USB class control transfers. OpenWave uses libusb for
those device controls and ALSA/PipeWire for host audio integration.

The capture keepalive exists because on Linux the Wave XLR can produce silence
when playback starts before capture. Keeping capture active first avoids that
race in practice.

## Repository

Public repository:

https://github.com/RamaAditya49/openwave

## License

MIT
