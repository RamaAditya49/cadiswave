# CadisWave

Kontrol perangkat Elgato Wave di Linux, dibuat dengan Rust, GTK4, libadwaita, dan PipeWire.

[English](README.md) · [Kontribusi](CONTRIBUTING.md) · [Arsitektur](docs/ARCHITECTURE.md) · [Migrasi](docs/migration.md)

CadisWave merupakan proyek independen berlisensi MIT yang dikembangkan dari [OpenWave](https://github.com/rikkichy/openwave), bukan produk resmi Elgato.

## Fitur

- Dashboard perangkat dengan meter input terukur, dial, mute, volume headphone, dan status koneksi.
- Bahasa Inggris dan Indonesia; pergantian bahasa tidak memulai ulang audio atau worker USB.
- Kontrol ringkas dan system tray; jendela utama tetap tersedia jika desktop tidak menyediakan tray.
- Mixer PipeWire, sumber aplikasi, beberapa mix, pemilihan output, scene, dan efek perangkat lunak.
- Command berdasarkan identitas perangkat, konfirmasi nilai aktual, antrean meter terbatas, dan penghentian worker yang terkontrol.
- Pemantauan aliran byte capture. Pemulihan yang mengganggu audio harus diaktifkan secara eksplisit.

## Dukungan perangkat

| Perangkat | USB ID | Kontrol yang telah dipetakan |
| --- | --- | --- |
| Wave XLR | `0fd9:007d` | Gain maksimal 75 dB, mute, phantom power, headphone, low impedance |
| XLR Dock | `0fd9:00a6` | Gain, mute, phantom power, headphone, low impedance |
| Wave:3 | `0fd9:0070` | Gain, mute, headphone, monitor mix |
| XLR Dock MK.2 | `0fd9:00c7` | Gain, mute, phantom power, headphone, monitor mix, low impedance |

Clipguard, hardware low-cut, perubahan LED, penyimpanan ke perangkat, dan perubahan sample rate belum tersedia.
Gunakan Mixer untuk software low-cut, gate, compressor, EQ, delay, dan mono.
Mode dial mengikuti pembacaan hardware. Mode yang tidak dikenal tidak dapat digunakan untuk mengubah nilai.

## Build dan pemasangan

Gunakan Rust 1.98.1, GTK 4.14+, libadwaita 1.5+, libusb 1.0, dan pkg-config.
Audio menggunakan PipeWire, pipewire-pulse, WirePlumber, ALSA utilities, dan plugin SWH LADSPA.

Untuk Ubuntu 24.04:

```bash
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev libusb-1.0-0-dev \
  pipewire pipewire-pulse wireplumber alsa-utils pulseaudio-utils swh-plugins
rustup toolchain install 1.98.1 --component rustfmt --component clippy
cargo build --release --locked --workspace --bins
make install PREFIX="$HOME/.local"
```

Jalankan `cadiswave`. Buka Pengaturan untuk memilih bahasa dan Mixer untuk sumber audio, efek, scene, serta pemilihan perangkat.
Tekan `M` untuk mute, tombol panah pada slider dial untuk perubahan presisi, dan `Ctrl+R` untuk reconnect.
Untuk pemasangan sistem, jalankan `install.sh` sebagai pengguna biasa; installer akan menyiapkan payload sebelum meminta akses administrator.

Aktifkan **Ikuti mute hardware** pada editor sumber capture olahan agar mute mengikuti perangkat Wave yang dipilih.

## Migrasi

Pertahankan instalasi lama sampai build baru lulus pemeriksaan.
Hentikan `openwave.service` sebelum CadisWave mengambil kontrol vendor.
Impor pengaturan bersifat opsional dan tidak menimpa file tujuan yang sudah ada.
Lihat [panduan migrasi dan pemulihan](docs/migration.md).

Sinkronisasi USB dimiliki aplikasi desktop, termasuk saat berjalan melalui tray.
Daemon capture mempertahankan stream input; daemon tersebut bukan pengganti worker USB aplikasi desktop.
File mockup, arsip desain, dan gambar referensi tidak disertakan dalam repository.

## Kredit

CadisWave dikembangkan dan dikelola oleh **Rama Aditya (CADIS)**.

OpenWave dan pemetaan protokol: rikkichy dan kontributor.
CadisWave dan artwork aplikasi baru: Rama Aditya (CADIS) dan kontributor.
[Lisensi MIT](LICENSE) tetap memuat copyright asli.
