# Lukfor

Quick launcher ala Spotlight untuk Windows. Tekan `Alt+Space`, ketik, Enter.
Dibangun dengan Tauri 2 + React + TypeScript.

![License](https://img.shields.io/badge/license-MIT-blue)
![Platform](https://img.shields.io/badge/platform-Windows%20x64-lightgrey)

## Fitur

| Input | Aksi |
| --- | --- |
| (teks biasa) | Fuzzy search aplikasi, file, dan folder |
| `2+2*3`, `sqrt(144)` | Kalkulator inline — Enter menyalin hasil |
| `g: sesuatu` | Buka Google search di browser default |
| `cb` / `cb kata` | Clipboard history (50 entri terakhir) — Enter menyalin kembali |
| `reindex` / `refresh` / `update` | Bangun ulang index sekarang (sama dengan `Ctrl+R`) |
| `Ctrl+D` pada hasil | Pin / lepas favorit (atau klik bintang di baris) |
| `↑` `↓` atau `Tab` / `Shift+Tab` | Pindah antar hasil; kursor tetap di kolom ketik |
| `shutdown` / `restart` / `sleep` (atau `matikan`, `mulai ulang`, `tidur`) | Matikan, mulai ulang, atau tidurkan PC. Enter pertama hanya menyiapkan (baris jadi oranye), Enter kedua yang menjalankan; mengetik, pindah baris, Esc, atau diam 6 detik membatalkan |

- **Favorit**: panel yang baru dibuka langsung menampilkan app, file, dan folder
  yang kamu pin, siap dibuka dengan panah + Enter. Saat mengetik, favorit yang
  cocok naik ke atas hasil. Tersimpan di `%LOCALAPPDATA%\Lukfor\favorites.json`;
  favorit yang sudah dihapus atau di-uninstall tetap tampil sebagai "Not found"
  supaya bisa dilepas.
- **Tema ikut Windows**: terang atau gelap mengikuti setelan Windows.

- **Semua jenis aplikasi terindeks**, lengkap dengan logo aslinya:
  - Shortcut Start Menu (`.lnk` / `.url`)
  - Aplikasi Microsoft Store / UWP (WhatsApp, Photos, Calculator, dll) — dienumerasi
    lewat `Get-StartApps` dan diluncurkan via `shell:AppsFolder\<AUMID>`
  - `.exe` dan shortcut portabel di folder user (Desktop, Downloads, dst)
- **Hotkey global**: `Alt+Space`, otomatis jatuh ke `Ctrl+Alt+Space` lalu
  `Ctrl+Shift+Space` bila sudah dipakai aplikasi lain (mis. PowerToys Run).
  Hotkey yang aktif ditampilkan di footer.
- **Esc / klik di luar / kehilangan fokus** → window langsung sembunyi.
- Window borderless, transparan, always-on-top, tidak muncul di taskbar.
- Index dibangun di background thread saat start: aplikasi dulu (instan), lalu
  folder user dengan batas kedalaman 8 dan maksimum 250.000 entri — pencarian
  tetap di bawah 250 ms walau file banyak.
- Index dibangun ulang otomatis di dua momen: tiap hari jam 03.00 selama
  aplikasi berjalan (kalau laptop sleep melewati jam 3, rebuild jalan begitu
  bangun), dan saat panel dibuka kalau index sudah lebih tua dari 6 jam.
  Refresh saat buka panel jalan di background — kamu tinggal mengetik seperti
  biasa, hasil lama tetap kepakai sampai yang baru siap.
- **Baru pasang aplikasi atau bikin folder dan mau langsung kepakai?** Tekan
  `Ctrl+R` (atau ketik `reindex`) untuk membangun ulang saat itu juga. Footer
  menghitung entri sampai selesai, lalu melaporkan hasil akhirnya.
- Clipboard watcher polling 800 ms, dedupe, maksimal 50 entri, hanya teks.

## Instalasi

Yang dibutuhkan: **Windows 10/11 64-bit**. Tidak perlu hak administrator, dan
tidak perlu memasang Node, Rust, atau apa pun — cukup satu file installer.

### Langkah 1 — Unduh installer

Buka halaman **[Releases](../../releases/latest)**, lalu di bagian **Assets**
klik file bernama:

```
Lukfor_0.1.0_x64-setup.exe
```

Ukurannya sekitar 2 MB.

### Langkah 2 — Jalankan installer

Klik dua kali file yang barusan diunduh.

> **Windows akan menampilkan layar biru bertuliskan "Windows protected your PC".**
> Ini normal dan bukan berarti file-nya berbahaya — peringatan itu muncul karena
> installer belum ditandatangani dengan sertifikat berbayar (harganya ratusan
> dolar per tahun), jadi Windows belum mengenalinya.
>
> Cara melewatinya:
>
> 1. Klik **More info** (tulisan kecil di bawah judul)
> 2. Klik tombol **Run anyway** yang baru muncul
>
> Kalau kamu ragu, kamu selalu bisa membangun installer-nya sendiri dari source
> code di repo ini — lihat bagian [Build rilis](#build-rilis).

Setelah itu installer berjalan sendiri tanpa pertanyaan apa pun dan langsung
selesai. Lukfor terpasang di `%LOCALAPPDATA%\Lukfor`.

Kalau WebView2 belum ada di komputermu, installer akan mengunduhnya otomatis
(butuh koneksi internet). Windows 11 sudah membawanya sejak awal, jadi biasanya
langkah ini terlewat begitu saja.

### Langkah 3 — Buka Lukfor

Installer membuat pintasan di **desktop** dan di **Start Menu**. Pakai salah
satunya untuk menjalankan Lukfor pertama kali.

Setelah berjalan, cara membukanya cukup tekan **`Alt` + `Space`**. Lukfor
sengaja tidak muncul di taskbar — ia menunggu diam-diam di latar belakang
sampai hotkey ditekan, jadi jangan bingung kalau setelah diklik seolah tidak
terjadi apa-apa.

Panel pencarian akan muncul di tengah layar. Ketik apa saja untuk mencari
aplikasi dan file, lalu tekan `Enter` untuk membuka. Tekan `Esc` atau klik di
luar panel untuk menutupnya.

> Saat pertama dijalankan, Lukfor perlu beberapa detik untuk memindai isi
> komputer. Jumlah item yang sudah terindeks terlihat di pojok kanan bawah panel.

### Langkah 4 (opsional) — Jalankan otomatis saat login

Installer sengaja tidak memasang autostart. Kalau kamu mau Lukfor selalu siap
setiap kali komputer menyala, buka **PowerShell** lalu tempel perintah ini:

```powershell
$exe = "$env:LOCALAPPDATA\Lukfor\lukfor.exe"
$s = (New-Object -ComObject WScript.Shell).CreateShortcut(
    "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\Lukfor.lnk")
$s.TargetPath = $exe; $s.WorkingDirectory = (Split-Path $exe); $s.Save()
```

Untuk membatalkannya, hapus file
`%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\Lukfor.lnk`.

### Uninstall

Lewat **Settings → Apps → Installed apps → Lukfor → Uninstall**, atau jalankan
`%LOCALAPPDATA%\Lukfor\uninstall.exe`.

### Kalau `Alt+Space` tidak berfungsi

1. **Pastikan Lukfor memang berjalan.** Cek dengan `Get-Process lukfor` di
   PowerShell. Kalau kosong, jalankan `%LOCALAPPDATA%\Lukfor\lukfor.exe`.
2. **Pastikan hanya ada satu instance.** Kalau perintah di atas menampilkan
   lebih dari satu baris, instance kedua kalah berebut `Alt+Space` dan diam-diam
   memakai kombinasi cadangan. Tutup semua lalu buka satu saja:
   ```powershell
   Get-Process lukfor | Stop-Process -Force
   Start-Process "$env:LOCALAPPDATA\Lukfor\lukfor.exe"
   ```
3. **Cek hotkey yang benar-benar aktif.** Aplikasi lain (misalnya PowerToys Run)
   mungkin sudah memakai `Alt+Space`. Kombinasi yang sedang dipakai Lukfor
   tertulis di pojok kiri bawah panel, dan juga di
   `%LOCALAPPDATA%\Lukfor\lukfor.log`.

## Development

Prasyarat: [Node.js](https://nodejs.org), [Rust](https://rustup.rs), dan
Microsoft C++ Build Tools (lihat
[prasyarat Tauri](https://tauri.app/start/prerequisites/)).

```powershell
npm install
npm run tauri dev
```

## Build rilis

```powershell
npm run tauri build
```

Menghasilkan `src-tauri/target/release/lukfor.exe` dan installer NSIS di
`src-tauri/target/release/bundle/nsis/`.

> **Penting:** jangan pakai `cargo build --release` langsung. Perintah itu
> menghasilkan binary yang memuat `devUrl` (`http://localhost:1420`) alih-alih
> menyertakan frontend hasil build, sehingga aplikasi hanya menampilkan
> "localhost refused to connect". Selalu lewat Tauri CLI seperti di atas.
> Untuk uji cepat tanpa installer: `npm run tauri build -- --no-bundle`.

## Catatan hotkey

`Alt+Space` secara default milik menu sistem window Windows, tetapi hotkey global
yang diregistrasi lewat `RegisterHotKey` mengambil alih kombinasi tersebut selama
Lukfor berjalan (perilaku yang sama dengan PowerToys Run). Jika registrasi gagal,
Lukfor otomatis memakai fallback dan menuliskannya di footer UI.

Kalau hotkey terasa aneh, pastikan hanya ada **satu** instance Lukfor yang jalan
(`Get-Process lukfor`) — instance kedua akan kalah rebutan `Alt+Space` dan diam-diam
turun ke fallback.

## Troubleshooting

Log startup ditulis ke `%LOCALAPPDATA%\Lukfor\lukfor.log` (registrasi hotkey,
jumlah entri terindeks, durasi indexing) karena build rilis berjalan tanpa console.

## Batasan

- Windows x64 saja.
- Clipboard history hanya teks (bukan gambar) dan tidak persisten antar restart.

## Lisensi

[MIT](LICENSE).
