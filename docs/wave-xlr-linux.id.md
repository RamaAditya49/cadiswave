# Elgato Wave XLR di Linux: setup, kontrol, dan mixing audio

[CadisWave](https://github.com/RamaAditya49/cadiswave) adalah software Wave XLR untuk Linux yang gratis dan open source.
Aplikasi ini menggabungkan kontrol mikrofon dengan mixer audio PipeWire native.
Rama Aditya (CADIS) mengembangkan dan mengelola proyek ini dengan lisensi MIT.

Panduan ini menjelaskan pemasangan, pemeriksaan perangkat, serta routing mikrofon dan audio aplikasi.
Lihat [README](../README.id.md) untuk ringkasan atau [panduan bahasa Inggris](wave-xlr-linux.md) untuk versi English.

## Apakah Wave XLR bisa dipakai di Linux?

CadisWave memiliki hasil pengujian fisik untuk Elgato Wave XLR asli dengan USB ID `0fd9:007d`.
Perubahan gain, mute, volume headphone, dan low impedance dikonfirmasi lewat pembacaan USB terpisah lalu dikembalikan ke keadaan awal.
Phantom power tidak diubah selama pengujian fisik.
Model lain memiliki pemetaan protokol, tetapi belum memiliki hasil pengujian fisik dalam proyek ini.
Lihat [dukungan hardware](hardware-support.md) dan [hasil pengujian](verification.md#physical-acceptance).

Capture audio Linux dan kontrol vendor USB merupakan dua jalur terpisah.
Input mikrofon yang berfungsi belum membuktikan bahwa aplikasi memiliki izin untuk mengubah pengaturan hardware.

## Pasang CadisWave

Ikuti [Build dan jalankan](../README.id.md#build-dan-jalankan) untuk dependensi, versi Rust, dan perintah pemasangan.
Aplikasi native saat ini menggunakan Rust 1.98.1, GTK 4.14+, libadwaita 1.5+, dan libusb 1.0.
Routing audio menggunakan PipeWire, pipewire-pulse, WirePlumber, ALSA utilities, dan plugin SWH LADSPA.

| Sistem | Bukti atau jalur pemasangan |
| --- | --- |
| Zorin OS 18.1 | [Pengujian fisik tercatat](verification.md) menggunakan library runtime host dan SDK pengembangan Ubuntu |
| Ubuntu 24.04 | [Workflow CI](../.github/workflows/tests.yml) dan [perintah dependensi](../README.id.md#build-dan-jalankan) |
| Bazzite / Fedora Atomic | [Panduan pemasangan host](install-bazzite.md); belum ada pengujian fisik pada image tersebut |
| Distro Linux lain | Periksa versi library dan nama paket; kompatibilitas perlu diverifikasi pada sistem masing-masing |

Rilis GitHub `v0.1.x` yang masih tersedia berisi aplikasi OpenWave sebelumnya.
Gunakan instruksi source saat ini untuk CadisWave native.
Jalur Flatpak masih eksperimental; keberadaan manifest bukan bukti paket yang sudah diterbitkan dan divalidasi.

## Periksa identitas perangkat dan izin USB

1. Sambungkan Wave XLR ke komputer.
2. Baca identitas USB yang terhubung:

   ```bash
   lsusb -d 0fd9:
   ```

3. Cocokkan product ID dengan [dukungan hardware](hardware-support.md).
4. Tutup aplikasi lain yang memegang kontrol vendor Wave.
5. Siapkan izin USB melalui administrator untuk instalasi di direktori pengguna.
6. Jalankan CadisWave sebagai pengguna biasa.
7. Pastikan perangkat yang dipilih di dashboard sudah benar.

[Panduan integrasi host](install-bazzite.md#first-run-host-integration) menjelaskan aturan USB dan setup layanan pengguna.
Panduan izin USB tersebut juga berlaku untuk instalasi native di direktori pengguna pada distro yang dapat diubah.
Instalasi sistem yang tepercaya bisa menyediakan jalur setup aplikasi dengan hak administrator.
Helper dari checkout atau direktori pengguna tidak dapat memakai jalur tersebut.

Jika sebelumnya memakai OpenWave, ikuti [panduan migrasi](migration.md) sebelum CadisWave mengambil kontrol vendor.
Daemon capture mempertahankan stream input secara terpisah dari aplikasi desktop yang memegang USB.

## Atur gain, mute, dan headphone

| Kontrol Wave XLR asli | Dukungan CadisWave |
| --- | --- |
| Gain mikrofon | Atur gain hingga 75 dB |
| Mute | Baca dan ubah status mute hardware |
| Volume headphone | Baca dan ubah level headphone |
| Low impedance | Baca dan ubah pengaturan hardware |
| Phantom power 48 V | Kontrol hardware tersedia; pengujian fisik tidak mengubah pengaturan ini |

Dashboard memisahkan nilai hardware yang terkonfirmasi dari perubahan yang masih diproses.
Gunakan **M** untuk mengubah mute dan tombol panah pada slider dial untuk mengatur nilai yang dipilih.
Gunakan **Ctrl+R** untuk menghubungkan ulang.

Periksa petunjuk produsen mikrofon sebelum mengaktifkan phantom power.
Scene tidak dapat mengaktifkan atau menonaktifkan phantom power.

Kontrol Clipguard, hardware low-cut, perubahan LED, penyimpanan ke perangkat, dan perubahan sample rate belum tersedia.
Mix monitor hardware Wave XLR asli belum dipetakan.
Efek software berjalan melalui PipeWire dan tidak mengubah DSP bawaan perangkat.

## Mix audio aplikasi dan mikrofon

CadisWave mengelola sumber game, chat, browser, dan mikrofon melalui PipeWire.
Setiap mix memiliki send dan kontrol master sendiri.
Input mix yang diterbitkan dapat digunakan di OBS, aplikasi rekaman, atau aplikasi panggilan.

1. Buka **Mixer**.
2. Tambahkan sumber aplikasi yang ingin kamu arahkan.
3. Tambahkan sumber capture untuk mikrofon.
4. Pilih node capture mikrofon yang tepat di editor sumber.
5. Atur send menuju mix yang diinginkan.
6. Periksa mute pada sumber, send, dan mix.
7. Pilih output hardware untuk monitoring.
8. Pilih input mix yang diterbitkan di OBS atau aplikasi panggilan.

Nama node input mix menggunakan `cadiswave_capture_<mix_id>`.
Menghapus mix akan menghapus inputnya.
Pilih input pengganti pada tiap aplikasi yang menggunakannya setelah penghapusan.

Aktifkan **Ikuti mute hardware** di editor sumber capture olahan untuk menyelaraskan mute dengan perangkat Wave yang dipilih.
Efek software mencakup low-cut, gate, compressor, EQ, delay, dan mono.
Plugin SWH LADSPA harus tersedia untuk rute yang menggunakan plugin tersebut.

Jangan masukkan audio balik aplikasi panggilan ke mix mikrofon aplikasi itu sendiri agar tidak terjadi feedback.
Monitoring langsung dari hardware dan monitoring software bisa membuat audio terdengar ganda.
Gunakan [pemecahan masalah routing](troubleshooting.md#application-or-mix-is-silent) untuk memeriksa jalur lengkap.

## Apakah CadisWave alternatif Wave Link untuk Linux?

Ya, untuk kontrol perangkat dan alur mixing PipeWire yang didukung di atas.
CadisWave adalah aplikasi independen yang dikembangkan dari OpenWave dan tetap mempertahankan pemberitahuan MIT asli.
CadisWave bukan produk resmi Elgato dan tidak menyediakan semua fitur Wave Link.

[Panduan setup Wave Link 3 dari Elgato](https://help.elgato.com/hc/en-us/articles/46941178829585-Wave-Link-3-0-Software-Initial-Setup) mencantumkan Windows, Mac, dan Windows on Arm.
Linux tidak tercantum.
Informasi sistem operasi ini diperiksa pada 2026-10-04.

## Minta bantuan

Buat [issue CadisWave](https://github.com/RamaAditya49/cadiswave/issues) dengan gejala dan langkah reproduksi.
Sertakan model perangkat, USB ID, distro Linux, dan versi library audio yang relevan.
Gunakan [diagnostik dengan detail privat yang dibatasi](troubleshooting.md#diagnostics-and-privacy) jika diperlukan.
Periksa file diagnostik sebelum membagikannya.

Lihat [FAQ README](../README.id.md#pertanyaan-umum) untuk jawaban singkat.
Lihat [verifikasi](verification.md) untuk hasil pengukuran dan batas pengujian fisik.
