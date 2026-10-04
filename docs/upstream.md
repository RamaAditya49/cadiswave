# Upstream origin

CadisWave derives from [OpenWave](https://github.com/rikkichy/openwave).
The native baseline is version 1.2.0, commit `4172c71db3e0b929571d02c06bbcb3456726e5e2`.
The merge preserves the original project history and five local Python changes.
The MIT license retains the original copyright.

Build with Rust 1.98.1, GTK 4.14+, libadwaita 1.5+, and libusb development files.
The baseline test suite checks core rules, runtime ownership, installation, and desktop callbacks.
Desktop callback tests require the isolated GTK test runner.
