# NEXORA Studio — Comprehensive Architecture & Codebase Map

> **Tujuan Dokumen Ini:**  
> Dokumen ini adalah peta arsitektur menyeluruh untuk **NEXORA Studio**. Setiap Agen AI (maupun developer) yang melanjutkan pengembangan proyek ini **WAJIB** membaca dokumen ini terlebih dahulu agar memahami struktur modul, aturan UI/UX, mekanisme *real-time capture*, serta konfigurasi *build/toolchain* khusus pada workspace ini tanpa kebingungan.

---

## 1. Identitas & Visi Aplikasi

- **Nama Aplikasi:** NEXORA Studio
- **Bahasa & Framework UI:** Rust (`edition = "2021"`) + `eframe` / `egui` `0.31`
- **Target Platform:** Windows 10 / 11 (64-bit Native Desktop Application)
- **Fokus Utama:** Aplikasi *screen & window recording* lokal berkualitas studio (`.mp4`) dengan antarmuka *compact*, modern, responsif, dan terinspirasi oleh alur kerja **OBS Studio**.
- **Mode Default:** **Studio Mode** (*Dual Preview*: kanvas **Preview** di kiri untuk pengeditan *scene/source* dan kanvas **Program** di kanan untuk output final rekaman).
- **Tema UI:** Mendukung penuh **Dark Mode** dan **Light Mode** secara menyeluruh (disimpan secara persisten).
- **Batasan Ketat (Out-of-Scope):**
  - **JANGAN** mengimplementasikan fitur *Live Streaming*, RTMP, integrasi YouTube/Twitch, chat streaming, fitur cloud, atau sistem login/akun. Fokus 100% pada keandalan *local recording* dan manajemen studio lokal.

---

## 2. Konfigurasi Toolchain & Cara Build (KRUSIAL UNTUK AGEN AI)

Lingkungan Windows pada mesin ini memiliki konfigurasi khusus yang **tidak boleh diubah sembarangan**:

1. **Lokasi Binary Rust (`cargo` & `rustc`):**
   - Berada di `C:\Users\JUNIOR-JR\.cargo\bin`.
   - Saat menjalankan perintah melalui terminal PowerShell otomatis, selalu awali dengan:
     ```powershell
     $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
     ```
2. **Target Toolchain (`rust-toolchain.toml`):**
   - Dipasang ke `stable-x86_64-pc-windows-gnu` (menggunakan `rust-lld` dan `llvm-dlltool` di `~/.cargo/bin`).
   - **Alasan:** Mesin ini tidak menginstal *Visual Studio C++ Build Tools* (`link.exe` MSVC) dan memiliki `C:\laragon\bin\git\usr\bin\link.exe` di `PATH` sistem yang akan menyebabkan error jika memakai target `msvc`.
3. **Lokasi Direktori Build (`.cargo/config.toml`):**
   - `target-dir = "D:/Rust/.nexora-target"`
   - **Alasan:** Path workspace `D:\Rust\Nexora Studio` mengandung karakter spasi. GCC wrapper pada target `windows-gnu` meneruskan `@response` file ke linker yang gagal jika `target-dir` mengandung spasi. Dengan menempatkan `target-dir` di `D:/Rust/.nexora-target`, proses kompilasi dan linking berjalan 100% bersih.
4. **Perintah Kompilasi, Test, dan Menjalankan Aplikasi:**
   ```powershell
   # 1. Hentikan instance aplikasi yang sedang berjalan agar file .exe tidak terkunci
   Get-Process nexora-studio -ErrorAction SilentlyContinue | Stop-Process -Force

   # 2. Jalankan unit test & kompilasi
   $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
   cargo test
   cargo build

   # 3. Jalankan binary hasil build
   & "D:\Rust\.nexora-target\debug\nexora-studio.exe"
   ```

---

## 3. Diagram Arsitektur Sistem

```
+-----------------------------------------------------------------------------------+
|                                  src/main.rs                                      |
|               (Entry Point eframe::run_native + Unit Test Suite)                  |
+-----------------------------------------+-----------------------------------------+
                                          |
                                          v
+-----------------------------------------------------------------------------------+
|                              src/app.rs (NexoraApp)                               |
|   Koordinator State Utama, Sinkronisasi Capture Request, & 60 FPS Update Loop     |
+------+-------------+--------------+-------------+--------------+------------------+
       |             |              |             |              |
       v             v              v             v              v
+------------+ +------------+ +-----------+ +-----------+ +-------------------------+
|   studio   | |  capture   | |   audio   | | recording | |         ui/*            |
|  (Scenes,  | | (Win32 GDI | |  (Mixer,  | | & encoder | | - icons.rs (Tabler)     |
|  Sources,  | |  Monitor & | |  Desktop  | | (Session, | | - theme.rs (Dark/Light) |
|  Preview/  | |  Window    | |  & Mic    | |  Timer,   | | - top_bar.rs            |
|  Program,  | |  Worker    | |  Meters,  | |  Output   | | - workspace.rs (16:9)   |
|  Transform)| |  Thread)   | |  Gain/dB) | |  Verify)  | | - dock.rs (OBS Panels)  |
+------------+ +------------+ +-----------+ +-----------+ | - dialogs.rs (Modals)   |
                                                          | - status_bar.rs         |
                                                          +-------------------------+
```

---

## 4. Peta Lengkap Direktori & File

### A. Konfigurasi Root & Aset Offline
| Path File / Folder | Fungsi & Penjelasan |
| :--- | :--- |
| [`Cargo.toml`](file:///d:/Rust/Nexora%20Studio/Cargo.toml) | Daftar dependensi (`eframe = "0.31"`, `egui = "0.31"`, `serde`, `serde_json`, `chrono`, `anyhow`, `tracing`, `directories`, `uuid`). |
| [`rust-toolchain.toml`](file:///d:/Rust/Nexora%20Studio/rust-toolchain.toml) | Mengunci toolchain ke `stable-x86_64-pc-windows-gnu`. |
| [`.cargo/config.toml`](file:///d:/Rust/Nexora%20Studio/.cargo/config.toml) | Mengatur `target-dir = "D:/Rust/.nexora-target"`. |
| [`assets/icons/tabler-icons.ttf`](file:///d:/Rust/Nexora%20Studio/assets/icons/tabler-icons.ttf) | Font vektor resmi **Tabler Icons** (`v3.31.0`, 5.936 ikon, `2.38 MB`) yang di-*embed* secara offline ke dalam binary via `include_bytes!`. |
| [`assets/icons/tabler-icons-codepoints.json`](file:///d:/Rust/Nexora%20Studio/assets/icons/tabler-icons-codepoints.json) | Kamus JSON offline yang memetakan seluruh 5.936 nama ikon Tabler (misal `"device-desktop": "ea89"`) ke *Unicode codepoint hex*. |
| [`assets/icons/tabler-sprite.svg`](file:///d:/Rust/Nexora%20Studio/assets/icons/tabler-sprite.svg) | Sprite SVG lengkap seluruh ikon Tabler untuk kebutuhan aset vektor offline di masa mendatang. |
| [`assets/icons/tabler-icons.min.css`](file:///d:/Rust/Nexora%20Studio/assets/icons/tabler-icons.min.css) | Referensi stylesheet CSS resmi Tabler Icons. |

---

### B. Modul Core & State (`src/`)
| Path File | Simbol Utama | Tanggung Jawab & Detail Implementasi |
| :--- | :--- | :--- |
| [`src/main.rs`](file:///d:/Rust/Nexora%20Studio/src/main.rs) | `main()`, `mod tests` | Menginisialisasi `tracing`, memuat ukuran/posisi/status *maximized* jendela terakhir dari [`AppSettings`](file:///d:/Rust/Nexora%20Studio/src/settings.rs), menjalankan `eframe::run_native`, serta memuat 5 *unit test* inti. |
| [`src/app.rs`](file:///d:/Rust/Nexora%20Studio/src/app.rs) | [`NexoraApp`](file:///d:/Rust/Nexora%20Studio/src/app.rs#L26-L49), [`QuickScreenTemplate`](file:///d:/Rust/Nexora%20Studio/src/app.rs#L18-L24) | Menyimpan seluruh state aplikasi, memulihkan sesi project & layout terakhir (`default_project.nexora.json`) secara otomatis saat startup, merekonsiliasi monitor/window/kamera tersimpan dengan perangkat sistem aktif ([`reconcile_sources_with_catalog`](file:///d:/Rust/Nexora%20Studio/src/app.rs#L133-L244)), menyediakan manajemen preset *Screen & Layout* ([`save_layout_preset`](file:///d:/Rust/Nexora%20Studio/src/app.rs#L357-L392), [`load_layout_preset`](file:///d:/Rust/Nexora%20Studio/src/app.rs#L395-L416)), serta menyimpan otomatis seluruh pengaturan & layout saat terjadi perubahan maupun saat keluar aplikasi (`on_exit`). |
| [`src/settings.rs`](file:///d:/Rust/Nexora%20Studio/src/settings.rs) | [`AppSettings`](file:///d:/Rust/Nexora%20Studio/src/settings.rs#L156-L173), [`ScreenLayoutPreset`](file:///d:/Rust/Nexora%20Studio/src/settings.rs#L141-L153), [`ThemeMode`](file:///d:/Rust/Nexora%20Studio/src/settings.rs#L8-L12), [`VideoPreset`](file:///d:/Rust/Nexora%20Studio/src/settings.rs#L36-L43), [`BitratePreset`](file:///d:/Rust/Nexora%20Studio/src/settings.rs#L81-L87) | Menyimpan & memuat preferensi pengguna secara otomatis ke `%APPDATA%\Nexora\NexoraStudio\config\settings.json` (tema, resolusi, FPS, bitrate, folder output, visibilitas panel, ukuran/posisi jendela, tinggi dock bawah, serta daftar *Screen & Layout Presets*). |
| [`src/studio/mod.rs`](file:///d:/Rust/Nexora%20Studio/src/studio/mod.rs) | [`StudioState`](file:///d:/Rust/Nexora%20Studio/src/studio/mod.rs#L15-L27), [`ViewportRole`](file:///d:/Rust/Nexora%20Studio/src/studio/mod.rs#L8-L12) | Mengelola daftar `Scene`, memisahkan `preview_scene_id` dan `program_scene_snapshot`, serta menyediakan fungsi transisi `transition_preview_to_program()` (`CUT / Apply`) dan mode `auto_sync_program` (`Link`). |
| [`src/studio/scene.rs`](file:///d:/Rust/Nexora%20Studio/src/studio/scene.rs) | [`Scene`](file:///d:/Rust/Nexora%20Studio/src/studio/scene.rs#L6-L12) | Menyimpan daftar layer [`SourceItem`](file:///d:/Rust/Nexora%20Studio/src/studio/source.rs#L181-L190), operasi tambah/hapus/duplikat/urutkan layer (`move_up`, `move_down`, `move_to_top`, `move_to_bottom`), dan [`top_display_label`](file:///d:/Rust/Nexora%20Studio/src/studio/scene.rs#L31-L65) untuk membuat label header otomatis di atas kanvas. |
| [`src/studio/source.rs`](file:///d:/Rust/Nexora%20Studio/src/studio/source.rs) | [`SourceItem`](file:///d:/Rust/Nexora%20Studio/src/studio/source.rs#L181-L190), [`SourceKind`](file:///d:/Rust/Nexora%20Studio/src/studio/source.rs#L5-L27), [`SourceTransform`](file:///d:/Rust/Nexora%20Studio/src/studio/source.rs#L79-L89), [`SourceColorTag`](file:///d:/Rust/Nexora%20Studio/src/studio/source.rs#L138-L147) | Mendefinisikan 4 jenis source (`DisplayCapture`, `WindowCapture`, `WebcamDevice`, `ImageSource`), ikon Tabler masing-masing (`tabler_icon()`), transformasi posisi/skala ternormalisasi (`0.0..1.0`) beserta preset PiP, dan label warna baris. |
| [`src/capture/mod.rs`](file:///d:/Rust/Nexora%20Studio/src/capture/mod.rs) | [`CaptureCatalog`](file:///d:/Rust/Nexora%20Studio/src/capture/mod.rs#L34-L39), [`CameraDeviceInfo`](file:///d:/Rust/Nexora%20Studio/src/capture/mod.rs#L27-L31), [`LiveCaptureEngine`](file:///d:/Rust/Nexora%20Studio/src/capture/mod.rs#L182-L194), [`MfCameraSession`](file:///d:/Rust/Nexora%20Studio/src/capture/mod.rs#L1000-L1010) | **Real-Time Win32 Screen, Window & Native Camera Capture:** Memindai monitor asli (`EnumDisplayMonitors` + `GetMonitorInfoW`), jendela aplikasi (`EnumWindows` + `GetWindowTextW`), deteksi otomatis seluruh kamera internal laptop dan webcam USB yang terpasang (`enumerate_camera_devices`), serta menangkap gambar live secara real-time via *worker thread* (`nexora-capture-worker`). Untuk kamera, menggunakan Windows Media Foundation (`IMFSourceReader`) dengan konversi hardware native `NV12` & `YUY2` ke `RGBA` dan sistem caching frame langsung ke tekstur GPU `egui::TextureHandle`. |
| [`src/audio/mod.rs`](file:///d:/Rust/Nexora%20Studio/src/audio/mod.rs) | [`AudioMixerState`](file:///d:/Rust/Nexora%20Studio/src/audio/mod.rs#L53-L60), [`AudioChannel`](file:///d:/Rust/Nexora%20Studio/src/audio/mod.rs#L10-L21) | **Native Windows WASAPI Audio Mixer:** Mendeteksi perangkat output audio (`waveOutGetDevCapsW`) & mikrofon asli (`MFEnumDeviceSources`), membaca level suara secara real-time melalui hardware peak meter Windows (`IAudioMeterInformation::GetPeakValue`), serta menyediakan kontrol volume (`0.0..1.0`), kalkulasi `%` dan desibel (`volume_db()`), serta `Mute` / `Disabled`. |
| [`src/recording/mod.rs`](file:///d:/Rust/Nexora%20Studio/src/recording/mod.rs) | [`RecordingController`](file:///d:/Rust/Nexora%20Studio/src/recording/mod.rs#L25-L35), [`RecordingStatus`](file:///d:/Rust/Nexora%20Studio/src/recording/mod.rs#L6-L11) | Mengatur siklus hidup rekaman (`Idle`, `Recording`, `Paused`), perhitungan waktu aktif (`elapsed_active()`) yang akurat saat di-*pause*, pembuatan nama file otomatis dengan *timestamp*, dan validasi file hasil rekaman. |
| [`src/encoder/mod.rs`](file:///d:/Rust/Nexora%20Studio/src/encoder/mod.rs) | [`EncoderConfig`](file:///d:/Rust/Nexora%20Studio/src/encoder/mod.rs#L22-L29) | Konfigurasi parameter video/audio encoder (`H.264/AVC` + `AAC`, resolusi, FPS, bitrate). |
| [`src/project/mod.rs`](file:///d:/Rust/Nexora%20Studio/src/project/mod.rs) | [`NexoraProject`](file:///d:/Rust/Nexora%20Studio/src/project/mod.rs#L8-L16), [`ProjectManager`](file:///d:/Rust/Nexora%20Studio/src/project/mod.rs#L49-L53) | Menyimpan & memuat seluruh konfigurasi scene, source, urutan layer, dan audio ke file JSON terstruktur (`default_studio.nexora.json`) serta memvalidasi ketersediaan perangkat saat project dibuka. |
| [`src/media_library/mod.rs`](file:///d:/Rust/Nexora%20Studio/src/media_library/mod.rs) | [`MediaLibrary`](file:///d:/Rust/Nexora%20Studio/src/media_library/mod.rs#L29-L32), [`RecordingEntry`](file:///d:/Rust/Nexora%20Studio/src/media_library/mod.rs#L6-L12) | Memindai direktori output untuk menampilkan daftar rekaman video beserta tanggal, ukuran file, aksi buka folder, dan putar video. |
| [`src/platform/mod.rs`](file:///d:/Rust/Nexora%20Studio/src/platform/mod.rs) | `open_folder_in_explorer`, `open_file_with_system_player` | Abstraksi perintah OS Windows (`explorer.exe` dan `cmd /C start`) secara non-blocking. |

---

### C. Modul Antarmuka Pengguna (`src/ui/`)
| Path File | Fungsi Utama | Detail Tampilan & Interaksi |
| :--- | :--- | :--- |
| [`src/ui/icons.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/icons.rs) | [`install_tabler_icons`](file:///d:/Rust/Nexora%20Studio/src/ui/icons.rs#L10-L26) | Mendaftarkan font `tabler-icons.ttf` ke `egui` dan menyediakan konstanta ikon (`DEVICE_DESKTOP`, `APP_WINDOW`, `CAMERA`, `PHOTO`, `EYE`, `EYE_OFF`, `LOCK`, `LOCK_OPEN`, `PLUS`, `MINUS`, `SETTINGS`, dll.). |
| [`src/ui/theme.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/theme.rs) | [`ThemePalette`](file:///d:/Rust/Nexora%20Studio/src/ui/theme.rs#L5-L26), [`apply_studio_theme`](file:///d:/Rust/Nexora%20Studio/src/ui/theme.rs#L74-L115) | Mengatur sistem warna permukaan bertingkat (*app background*, *panel*, *elevated widget*, *canvas well*, *accent colors*, *status badges*) untuk Dark Mode dan Light Mode. |
| [`src/ui/top_bar.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/top_bar.rs) | [`render_top_bar`](file:///d:/Rust/Nexora%20Studio/src/ui/top_bar.rs#L8-L236) | Baris navigasi atas (`34px`): identitas `NEXORA Studio`, menu `Project`, menu `Panels`, tombol `Studio Mode`, indikator `REC / STANDBY` + timer, toggle `Light/Dark Mode`, tombol `Library`, dan `Settings`. |
| [`src/ui/workspace.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/workspace.rs) | [`render_preview_workspace`](file:///d:/Rust/Nexora%20Studio/src/ui/workspace.rs#L9-L122) | Merender toolbar transformasi cepat (`Zoom`, `Safe Area`, `PiP Presets`), dua layar **Preview** & **Program** dengan rasio terkunci `16:9`, *inter-viewport transition bar* (`CUT`, `Apply`, `Link`), interaksi *drag-and-drop* posisi layer pada kanvas **Preview**, merender tekstur tangkapan layar asli (`live_tex`), serta **OBS-Style Screen Context Menu** (klik kanan langsung pada kanvas output untuk `Open Preview/Program Projector` dan `Save Preview/Program Screenshot`). |
| [`src/ui/dock.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/dock.rs) | [`render_bottom_studio_dock`](file:///d:/Rust/Nexora%20Studio/src/ui/dock.rs#L10-L107) | Merender 4 panel bawah bergaya OBS Studio: **`Scenes`**, **`Source`**, **`Audio Mixer`**, dan **`Controls`**. Mendukung *context menu* klik kanan lengkap pada **`Source`**, *double-click* untuk membuka **Properties**, dan *shortcut* keyboard (`Del` untuk hapus, `F2` untuk rename). |
| [`src/ui/dialogs.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/dialogs.rs) | [`render_dialogs`](file:///d:/Rust/Nexora%20Studio/src/ui/dialogs.rs#L124-L579), [`render_library_drawer_if_open`](file:///d:/Rust/Nexora%20Studio/src/ui/dialogs.rs#L9-L122) | Merender jendela pop-up **Properties for '<Source>'** (pemilihan monitor/window/kamera, opsi kursor, preset transformasi, tombol *Refresh Devices*), **Settings**, **Rename Scene**, **Rename Source**, dan *side drawer* **Recording Library**. |
| [`src/ui/status_bar.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/status_bar.rs) | [`render_status_bar`](file:///d:/Rust/Nexora%20Studio/src/ui/status_bar.rs#L6-L78) | Baris status bawah (`24px`) yang menampilkan status engine capture, resolusi/FPS/bitrate output, status kanal audio, dan notifikasi lokasi penyimpanan rekaman. |

---

## 5. Aturan Penting UI/UX (Wajib Dipatuhi Agen Selanjutnya)

1. **Label Scene & Source Otomatis di Bagian Atas Saja:**
   - Label deskriptif scene (contoh: `Scene 1 - Desktop + Cam PiP`) hanya ditampilkan secara otomatis pada *header bar* di atas kanvas **Preview** / **Program** (lihat [`Scene::top_display_label`](file:///d:/Rust/Nexora%20Studio/src/studio/scene.rs#L31-L65)).
   - Label informasi layer source (contoh: `🖥 Primary Display — Primary Monitor (Display 1) • 1920×1080`) hanya ditampilkan sebagai *header strip* tipis (`20px`) di bagian paling atas kotak layer, **bukan** di tengah-tengah kanvas agar tidak menghalangi tampilan layar yang direkam.
2. **Penamaan & Alur Kerja Panel `Source` (Gaya OBS Studio):**
   - Nama panel kedua di dock bawah wajib singkat dan jelas: **`Source`**.
   - Penambahan dan pengeditan source dilakukan melalui:
     - **Klik Kanan (*Context Menu*)** pada item di daftar `Source` atau pada ruang kosong di dalam daftar `Source` (`Add Source`, `Set Color`, `Order`, `Transform`, `Copy / Duplicate`, `Remove`, `Rename...`, `Properties`).
     - **Klik Ganda (*Double-Click*)** pada item `Source` untuk langsung membuka jendela **Properties**.
     - **Bottom Toolbar** di bawah daftar `Source`: tombol `[ + ]`, `[ - ]`, `[ ⚙ Properties ]`, `[ ↑ ]`, `[ ↓ ]`.
3. **Tata Letak Kolom Dock Bawah Wajib Vertikal (`Layout::top_down`):**
   - Di dalam `ui.horizontal(...)` pada [`render_bottom_studio_dock`](file:///d:/Rust/Nexora%20Studio/src/ui/dock.rs#L54-L104), setiap panel (`Scenes`, `Source`, `Audio Mixer`, `Controls`) **wajib** dibungkus dengan `ui.allocate_ui_with_layout(Vec2::new(col_w, total_h), Layout::top_down(Align::Min), ...)` dan `ui.set_max_size(...)`.
   - Jangan pernah menggunakan `ui.allocate_ui(Vec2, ...)` langsung di dalam `ui.horizontal(...)` karena `egui` akan mewariskan layout horizontal ke dalam kartu dan menyebabkan isi panel berjejer ke samping (*horizontal overflow*).
4. **Penggunaan Ikon Offline Tabler Icons:**
   - Selalu gunakan konstanta ikon dari [`src/ui/icons.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/icons.rs).
   - Jika membutuhkan ikon baru, cari namanya di [`assets/icons/tabler-icons-codepoints.json`](file:///d:/Rust/Nexora%20Studio/assets/icons/tabler-icons-codepoints.json) dan tambahkan konstanta `\u{xxxx}`-nya ke [`src/ui/icons.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/icons.rs).

---

## 6. Roadmap Pengembangan Tahap Berikutnya

1. **Stage 3 — Real-Time Video/Audio Encoding & MP4 Muxing (`src/encoder/` & `src/recording/`):**
   - Menghubungkan *frame buffer* RGBA dari [`LiveCaptureEngine`](file:///d:/Rust/Nexora%20Studio/src/capture/mod.rs) dan stream audio WASAPI (Desktop Loopback + Microphone) ke pipeline encoder `H.264 + AAC` (melalui FFmpeg CLI/library atau Windows Media Foundation `IMFSinkWriter`) untuk menghasilkan file `.mp4` berdurasi penuh saat tombol **Start Recording** ditekan.
2. **Stage 4 — Live Webcam & Image Overlay Rendering (`src/capture/` & `src/ui/workspace.rs`):**
   - Menambahkan tangkapan *frame* kamera/webcam nyata (Media Foundation / DirectShow) untuk `SourceKind::WebcamDevice` dan pemuatan tekstur gambar PNG/JPG nyata untuk `SourceKind::ImageSource` di kanvas **Preview** & **Program**.

