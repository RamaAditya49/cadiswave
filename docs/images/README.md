# Application screenshots

These files show the actual CadisWave GTK interface.
They were captured on 2026-10-04 from revision `a563899`.
The isolated gallery uses controlled device and audio observations.
Meter values and the `Gallery` identity are test fixture data.
They are not recordings from a live daily session.

The gallery test is `app::tests::device_gallery_renders_both_languages_and_small_windows`.
It resides in `crates/cadiswave-desktop/src/app.rs`.
The test renders English and Indonesian through the production widgets and styles.

| File | Captured view | Capture region |
| --- | --- | --- |
| `device-en.png` | English device dashboard | 1280×800 at 0,0 |
| `device-id.png` | Indonesian device dashboard | 1280×800 at 0,0 |
| `compact-en.png` | English compact window | 400×684 at 0,0 |
| `compact-id.png` | Indonesian compact window | 400×684 at 0,0 |
| `settings-footer-en.png` | English settings footer | 480×714 at 400,43 |
| `settings-footer-id.png` | Indonesian settings footer | 480×714 at 400,43 |

Capture regions omit the private display background and windows behind the foreground dialog.
PNG files retain the original application pixels.
No generative image processing, text replacement, or interface reconstruction was applied.
The files contain no private device serials or host audio configuration.

CadisWave artwork and interface are maintained by Rama Aditya (CADIS) and contributors.
See the repository MIT license and original OpenWave notices.
Design archives and reference mockups remain outside Git.
