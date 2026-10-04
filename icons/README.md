# CadisWave artwork

Rama selected the mint Signal C icon on 2026-10-04.
The canonical selected image is `cadiswave.png`.
The installed application icon is `cadiswave.svg`.
Its SVG contains the exact PNG data and does not require another installed image.

The installed viewport excludes empty outer padding.
The C, waveform, colors, and material shading remain unchanged.
Update that SVG with this command from the repository root:

```bash
python3 packaging/sync-artwork.py --viewport 189 205 876 876
```

Use no viewport argument to include the complete PNG after replacing the artwork.
Keep the selected PNG and its SVG together in each artwork change.

White and black tray icons use a simple C and waveform for small displays.
Red indicates hardware mute.
Keep the existing icon names because saved preferences and tray clients use them.

GTK registers the supplied artwork directory before creating application windows.
Notifications use the supplied application file icon.
Desktop entries match the Wayland application identifier with `StartupWMClass`.
The installer refreshes the desktop icon cache when the cache tool is available.

Rama Aditya (CADIS) and contributors maintain this artwork.
The repository MIT license applies.
