<p align="center">
  <img src="https://raw.githubusercontent.com/RamaAditya49/cadiswave/main/icons/cadiswave.png" alt="CadisWave icon" width="96" height="96">
</p>
<h1 align="center">CadisWave — Software Wave XLR untuk Linux</h1>
<p align="center"><strong>Kontrol Elgato Wave XLR dan mixer audio PipeWire yang open source.</strong></p>
<p align="center">Atur gain mikrofon, mute, phantom power, dan headphone lewat aplikasi desktop Linux native.</p>
<p align="center">
  <a href="https://github.com/RamaAditya49/cadiswave/actions/workflows/tests.yml"><img src="https://github.com/RamaAditya49/cadiswave/actions/workflows/tests.yml/badge.svg?branch=main" alt="Pengujian"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-7dff9b?labelColor=141a17" alt="Lisensi MIT"></a>
  <img src="https://img.shields.io/badge/platform-Linux-7dff9b?labelColor=141a17" alt="Linux">
</p>
<p align="center"><strong>Rust · GTK4 · libadwaita · PipeWire</strong></p>
<p align="center">
  <a href="README.md">English</a> ·
  <a href="#build-dan-jalankan">Pasang dari source</a> ·
  <a href="docs/wave-xlr-linux.id.md">Setup Wave XLR di Linux</a> ·
  <a href="#pertanyaan-umum">FAQ</a> ·
  <a href="CONTRIBUTING.md">Kontribusi</a> ·
  <a href="docs/verification.md">Hasil pengujian</a>
</p>

**CadisWave adalah software Wave XLR untuk Linux yang gratis dan open source**, dikembangkan oleh [Rama Aditya](https://github.com/RamaAditya49) (CADIS).
Aplikasi ini menyediakan kontrol mikrofon Elgato, routing audio aplikasi, mix terpisah, dan efek software melalui PipeWire.
CadisWave bisa dipakai sebagai alternatif Linux native untuk alur kontrol dan mixing Wave Link.
Baca [batas dukungan](#dukungan-perangkat) sebelum beralih.

| Ringkasan proyek | Detail |
| --- | --- |
| Platform | Desktop Linux dengan PipeWire; Rust, GTK4, dan libadwaita native |
| Perangkat yang diuji fisik | Elgato Wave XLR asli, USB `0fd9:007d` |
| Bahasa antarmuka | English dan Bahasa Indonesia |
| Pemasangan | [Build dari source](#build-dan-jalankan); lihat [setup Linux](docs/wave-xlr-linux.id.md) |
| Lisensi dan pengembang | [MIT](LICENSE); Rama Aditya (CADIS) |
| Bukti pengujian | [Pemeriksaan software dan hasil fisik](docs/verification.md), dicatat pada 2026-10-04 |

![Dashboard perangkat CadisWave dengan meter input, dial gain, dan kontrol headphone](docs/images/device-id.png)

<p align="center"><sub>UI GTK asli dengan data uji terkontrol. Nilai meter berasal dari fixture pengujian.</sub></p>

## Untuk kontrol audio sehari-hari

| Fitur | Yang bisa kamu lakukan |
| --- | --- |
| Dashboard perangkat | Baca level input, atur gain, aktifkan mute, dan kontrol headphone. |
| Kontrol ringkas | Akses kontrol utama dalam jendela kecil. Gunakan tray jika desktop menyediakan tray host. |
| Ikon status | Hijau saat aktif, merah saat mute, oranye saat error. Integrasi dock tersedia untuk GNOME 46. |
| Ukuran jendela adaptif | Gunakan ukuran terakhir yang muat di layar saat ini. Kontrol utama menyesuaikan jendela sempit. |
| Mixer PipeWire | Kelola sumber aplikasi, mix terpisah, output, scene, dan efek software. |
| Inggris dan Indonesia | Ganti bahasa tanpa memulai ulang audio atau worker USB. |
| Status perangkat terkonfirmasi | Lihat nilai hasil pembacaan hardware, terpisah dari perubahan yang masih diproses. |
| Pemantauan capture | Periksa aktivitas byte capture dan koneksi. Pemulihan yang mengganggu audio harus diaktifkan secara eksplisit. |
| Pengaturan mikrofon | Pilih mikrofon, atur low-cut, terapkan preset suara, dan simpan preset bernama. |
| Tes mikrofon | Rekam 1–10 detik di memori. Periksa level puncak dan clipping, lalu putar melalui output pilihan. |

## Lihat lebih dekat

<table>
  <tr>
    <th>Kontrol ringkas</th>
    <th>Pengaturan aplikasi</th>
  </tr>
  <tr>
    <td align="center"><img src="docs/images/compact-id.png" alt="Kontrol ringkas CadisWave dalam bahasa Indonesia" height="400"></td>
    <td align="center"><img src="docs/images/settings-footer-id.png" alt="Pengaturan CadisWave dalam bahasa Indonesia dengan jarak tombol yang diperbaiki" height="400"></td>
  </tr>
</table>

Screenshot menampilkan aplikasi yang dirender dengan fixture GTK terkontrol, bukan mockup desain.
Lihat [asal screenshot](docs/images/README.md) untuk detail capture.

## Dukungan perangkat

| Perangkat | USB ID | Kontrol yang dipetakan | Pengujian fisik CadisWave |
| --- | --- | --- | --- |
| Wave XLR | `0fd9:007d` | Gain maksimal 75 dB, mute, phantom power, headphone, low impedance, monitor mix | Kontrol lama sudah diuji; monitor baru belum diuji pada perangkat fisik; lihat [hasil](docs/verification.md) |
| XLR Dock | `0fd9:00a6` | Gain, mute, phantom power, headphone, low impedance | Belum diuji di sini |
| Wave:3 | `0fd9:0070` | Gain, mute, headphone, monitor mix, Clipguard (API 5.3/5.4) | Belum diuji di sini |
| XLR Dock MK.2 | `0fd9:00c7` | Gain, mute, phantom power, headphone, monitor mix, low impedance, Clipguard, hardware low-cut | Belum diuji di sini |

Kontrol yang dipetakan memiliki tes protokol. Hasilnya bukan bukti pengujian fisik untuk semua model atau firmware.
Baca [dukungan hardware](docs/hardware-support.md) sebelum memakai perangkat lain.

Pengaturan perangkat menampilkan kontrol sesuai model perangkat. Kontrol yang belum didukung memiliki penjelasan.
Perubahan LED, penyimpanan eksplisit ke perangkat, dan perubahan sample rate belum tersedia.
Pembacaan sample rate membedakan format stream yang teramati dari properti konfigurasi.
Gunakan pengaturan perangkat untuk low-cut software dan preset suara. Gunakan Mixer untuk gate, compressor, EQ, delay, dan mono.
Tes mikrofon merekam 1–10 detik di memori, menghitung level puncak dan clipping, lalu memutar hasil melalui output pilihan.
Preset bawaan mencakup Meeting, Podcast, dan Streaming. Preset bernama dapat disimpan dan dihapus.
Mode dial yang tidak dikenal tidak dapat digunakan untuk mengubah nilai.

## Build dan jalankan

Gunakan **Rust 1.98.1**, GTK **4.14+**, libadwaita **1.5+**, libusb 1.0, dan pkg-config.
Audio menggunakan PipeWire, pipewire-pulse, WirePlumber, ALSA utilities, dan plugin SWH LADSPA.

Untuk Ubuntu 24.04, pasang dependensi:

```bash
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev libusb-1.0-0-dev \
  pipewire pipewire-pulse wireplumber alsa-utils pulseaudio-utils swh-plugins
rustup toolchain install 1.98.1 --component rustfmt --component clippy
```

Build dan pasang sebagai pengguna biasa:

```bash
git clone https://github.com/RamaAditya49/cadiswave.git
cd cadiswave
cargo build --release --locked --workspace --bins
make install PREFIX="$HOME/.local"
"$HOME/.local/bin/cadiswave"
```

Buka **Pengaturan** untuk memilih bahasa. Buka **Mixer** untuk sumber audio, efek, scene, dan pemilihan perangkat.

| Shortcut | Tindakan |
| --- | --- |
| `M` | Aktifkan atau nonaktifkan mute |
| Tombol panah pada slider dial | Atur nilai yang dipilih |
| `Ctrl+R` | Hubungkan ulang |

Untuk pemasangan sistem, jalankan `install.sh` sebagai pengguna biasa.
Installer menyiapkan dan memeriksa payload sebelum meminta akses administrator.
Lihat [pengaturan hardware](docs/hardware-support.md), [pemasangan Bazzite](docs/install-bazzite.md), dan [pemecahan masalah](docs/troubleshooting.md).

Aktifkan **Ikuti mute hardware** pada editor sumber capture olahan agar mute mengikuti perangkat Wave yang dipilih.

## Pindah dari OpenWave

Pertahankan instalasi lama sampai build baru lulus pemeriksaan.
Hentikan `openwave.service` sebelum CadisWave mengambil kontrol vendor.
CadisWave menolak pemilik vendor kedua. Impor opsional mempertahankan pengaturan yang sudah ada.
Ikuti [panduan migrasi dan pemulihan](docs/migration.md).

Aplikasi desktop memiliki sinkronisasi USB, termasuk saat tersembunyi melalui tray.
Daemon capture mempertahankan stream input; daemon tersebut bukan pengganti worker USB aplikasi desktop.

## Pertanyaan umum

### Software apa yang bisa mengontrol Elgato Wave XLR di Linux?

CadisWave mengontrol Wave XLR asli di Linux melalui USB dan sinkronisasi ALSA.
Kontrolnya mencakup gain, mute, phantom power 48 V, volume headphone, dan low impedance.
Lihat [hasil pengujian fisik](docs/verification.md#physical-acceptance) dan [panduan setup](docs/wave-xlr-linux.id.md).

### Apakah Elgato Wave Link mendukung Linux?

[Panduan setup Wave Link 3 dari Elgato](https://help.elgato.com/hc/en-us/articles/46941178829585-Wave-Link-3-0-Software-Initial-Setup) mencantumkan Windows, Mac, dan Windows on Arm.
Linux tidak tercantum.
CadisWave adalah aplikasi Linux independen untuk kontrol perangkat dan mixing PipeWire.
Tidak semua fitur Wave Link tersedia di CadisWave.

### Bisa mencampur audio game, chat, browser, dan mikrofon?

Bisa. Mixer PipeWire mengelola sumber aplikasi, sumber capture, send, mix terpisah, dan output.
Pilih input mix yang diterbitkan di OBS atau aplikasi panggilan.
Baca [alur routing](docs/wave-xlr-linux.id.md#mix-audio-aplikasi-dan-mikrofon) sebelum mengubah rute audio yang sedang dipakai.

### Distro Linux apa yang sudah diuji?

Pengujian fisik menggunakan Zorin OS 18.1, GTK 4.14.5, libadwaita 1.5.0, dan PipeWire 1.0.5.
CI berjalan di Ubuntu 24.04.
[Panduan Bazzite dan Fedora Atomic](docs/install-bazzite.md) menjelaskan jalur pemasangan lain; belum ada pengujian fisik yang tercatat pada image tersebut.
Lihat [verifikasi](docs/verification.md) untuk dependensi build dan batas pengujian.

### Apakah CadisWave menggantikan semua fitur Wave Link?

CadisWave menyediakan kontrol perangkat, routing, scene, dan efek yang diproses di komputer.
Clipguard dan low-cut hardware tersedia sesuai model perangkat.
Perubahan LED, penyimpanan eksplisit ke perangkat, dan perubahan sample rate belum tersedia.
Lihat [dukungan perangkat](#dukungan-perangkat) untuk batas tiap model.

### Di mana bisa mengunduh atau memasang CadisWave?

Gunakan [langkah pemasangan dari source](#build-dan-jalankan) untuk aplikasi native saat ini.
Rilis `v0.1.x` yang masih tersedia berasal dari aplikasi OpenWave sebelumnya.
Rilis tersebut bukan build CadisWave native saat ini.

### Apakah CadisWave tersedia dalam bahasa Indonesia?

Ya. Pilih English atau Bahasa Indonesia di Pengaturan.
Pergantian bahasa tidak memulai ulang worker audio atau USB.
[README bahasa Inggris](README.md) menyediakan informasi yang setara.

## Kontribusi

Mulai dari [panduan kontributor](CONTRIBUTING.md).
Baca [arsitektur](docs/ARCHITECTURE.md), [referensi protokol](docs/protocol.md), dan [panduan lokalisasi](docs/localization.md).
Laporkan bug dan usulan fitur lewat [GitHub issues](https://github.com/RamaAditya49/cadiswave/issues).
Sertakan model perangkat, USB ID, dan langkah reproduksi.

## Pengembang dan lisensi

**Dikembangkan dan dikelola oleh [Rama Aditya](https://github.com/RamaAditya49) (CADIS).**

CadisWave adalah proyek independen yang dikembangkan dari [OpenWave](https://github.com/rikkichy/openwave) oleh rikkichy dan kontributor.
Implementasi CadisWave dan artwork aplikasi baru: Rama Aditya (CADIS) dan kontributor.
[Lisensi MIT](LICENSE) tetap memuat copyright asli.
CadisWave bukan produk resmi Elgato.

Arsip desain dan file mockup tetap berada di luar repository.
Screenshot di atas merupakan dokumentasi aplikasi.
