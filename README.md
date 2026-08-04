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
- Index otomatis dibangun ulang tiap hari jam 03.00 selama aplikasi berjalan.
  Kalau laptop sleep melewati jam 3, rebuild jalan begitu bangun.
- Clipboard watcher polling 800 ms, dedupe, maksimal 50 entri, hanya teks.

## Instalasi

Unduh `Lukfor_<versi>_x64-setup.exe` dari halaman
[Releases](../../releases), lalu jalankan.

- Tidak butuh hak admin — terpasang ke `%LOCALAPPDATA%\Lukfor`.
- WebView2 diinstal otomatis bila belum ada (Windows 11 sudah bawaan).
- Installer belum ditandatangani, jadi SmartScreen akan memberi peringatan:
  klik **More info → Run anyway**.

Uninstall lewat **Apps & Features**, atau jalankan
`%LOCALAPPDATA%\Lukfor\uninstall.exe`.

### Menjalankan otomatis saat login

Installer tidak memasang autostart. Untuk mengaktifkannya, jalankan sekali di
PowerShell:

```powershell
$exe = "$env:LOCALAPPDATA\Lukfor\lukfor.exe"
$s = (New-Object -ComObject WScript.Shell).CreateShortcut(
    "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\Lukfor.lnk")
$s.TargetPath = $exe; $s.WorkingDirectory = (Split-Path $exe); $s.Save()
```

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
