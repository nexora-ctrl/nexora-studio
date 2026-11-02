# NEXORA Studio — Comprehensive Architecture & Codebase Map

> **Tujuan Dokumen Ini:**  
> Dokumen ini adalah peta arsitektur menyeluruh untuk **NEXORA Studio**. Setiap Agen AI (maupun developer) yang melanjutkan pengembangan proyek ini **WAJIB** membaca dokumen ini terlebih dahulu agar memahami struktur modul, aturan konsistensi UI/UX, mekanisme *real-time capture & audio*, sistem filter transparansi (Chroma Key, Color Key, Luma Key), urutan *layer* (*Z-ordering*), serta konfigurasi *build/toolchain* khusus pada workspace ini tanpa kebingungan.

---

## 1. Identitas & Visi Aplikasi

- **Nama Aplikasi:** NEXORA Studio
- **Bahasa & Framework UI:** Rust (`edition = "2021"`) + `eframe` / `egui` `0.31`
- **Target Platform:** Windows 10 / 11 (64-bit Native Desktop Application)
- **Fokus Utama:** Aplikasi *screen & window recording* serta *live studio presentation* lokal berkualitas studio (`.mp4`) dengan antarmuka *compact*, modern, responsif, dan terinspirasi oleh alur kerja **OBS Studio**.
- **Mode Default:** **Studio Mode** (*Dual Viewport*: kanvas **Preview** di kiri untuk pengeditan *scene/source* dan kanvas **Program** di kanan untuk output final rekaman & proyektor).
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
|         (Entry Point eframe::run_native + Tracing Filter + Unit Test Suite)       |
+-----------------------------------------+-----------------------------------------+
                                          |
                                          v
+-----------------------------------------------------------------------------------+
|                              src/app.rs (NexoraApp)                               |
|   Koordinator State Utama, Sinkronisasi Capture/Audio, & 60 FPS Update Loop       |
+------+-------------+--------------+-------------+--------------+------------------+
       |             |              |             |              |
       v             v              v             v              v
+------------+ +------------+ +-----------+ +-----------+ +-------------------------+
|   studio   | |  capture   | |   audio   | | platform  | |         ui/*            |
|  (Scenes,  | | (Win32 GDI,| |  (WASAPI  | | (Win32    | | - icons.rs (Tabler)     |
|  Sources,  | |  MF Camera,| |  Desktop  | |  Dialogs, | | - theme.rs (Dark/Light) |
|  Cut/Fade, | |  MF Video, | |  & Mic    | |  MF+WASAPI| | - top_bar.rs            |
|  Key Filter| |  Real-Time | |  Meters,  | |  Audio    | | - workspace.rs (16:9)   |
|  Chroma/   | |  Chroma/   | |  Gain/dB) | |  Engine)  | | - dock.rs (OBS Panels)  |
|  Color/    | |  Color/    | |           | |           | | - dialogs.rs (Modals &  |
|  Luma Key) | |  Luma Key) | |           | |           | |   Projector, Filters &  |
+------------+ +------------+ +-----------+ +-----------+ |   Slideshow Manager)    |
                                                          | - status_bar.rs         |
                                                          +-------------------------+
```

---

## 4. Peta Lengkap Direktori & File

### A. Konfigurasi Root & Aset Offline
| Path File / Folder | Fungsi & Penjelasan |
| :--- | :--- |
| [`Cargo.toml`](file:///d:/Rust/Nexora%20Studio/Cargo.toml) | Daftar dependensi (`eframe = "0.31"`, `egui = "0.31"`, `image` dengan fitur `png`, `jpeg`, `bmp`, `webp`, `gif`, `ico`, `serde`, `serde_json`, `chrono`, `anyhow`, `tracing`, `directories`, `uuid`). |
| [`rust-toolchain.toml`](file:///d:/Rust/Nexora%20Studio/rust-toolchain.toml) | Mengunci toolchain ke `stable-x86_64-pc-windows-gnu`. |
| [`.cargo/config.toml`](file:///d:/Rust/Nexora%20Studio/.cargo/config.toml) | Mengatur `target-dir = "D:/Rust/.nexora-target"`. |
| [`assets/icons/tabler-icons.ttf`](file:///d:/Rust/Nexora%20Studio/assets/icons/tabler-icons.ttf) | Font vektor resmi **Tabler Icons** (`v3.31.0`, 5.936 ikon, `2.38 MB`) yang di-*embed* secara offline ke dalam binary via `include_bytes!`. |
| [`assets/icons/tabler-icons-codepoints.json`](file:///d:/Rust/Nexora%20Studio/assets/icons/tabler-icons-codepoints.json) | Kamus JSON offline yang memetakan seluruh 5.936 nama ikon Tabler ke *Unicode codepoint hex*. |
| [`assets/icons/tabler-sprite.svg`](file:///d:/Rust/Nexora%20Studio/assets/icons/tabler-sprite.svg) | Sprite SVG lengkap seluruh ikon Tabler untuk kebutuhan aset vektor offline. |

---

### B. Modul Core & State (`src/`)
| Path File | Simbol Utama | Tanggung Jawab & Detail Implementasi |
| :--- | :--- | :--- |
| [`src/main.rs`](file:///d:/Rust/Nexora%20Studio/src/main.rs) | `main()`, `mod tests` | Menginisialisasi `tracing` (menyaring warning `wgpu_hal::vulkan::conv` terkait present mode NVIDIA driver `1000361000`), memuat preferensi jendela terakhir dari [`AppSettings`](file:///d:/Rust/Nexora%20Studio/src/settings.rs), menjalankan `eframe::run_native`, serta memuat 6 *unit test* inti. **Renderer dikunci** ke `Renderer::Wgpu` dengan `backends = VULKAN | DX12` dan `power_preference = HighPerformance`; backend `GL`/fitur `glow` sengaja tidak dipakai karena driver OpenGL Intel (`igxelpicd64.dll`) melaporkan Access Violation `0xc0000005` dan mematikan proses saat Projector membuka window kedua. |
| [`src/app.rs`](file:///d:/Rust/Nexora%20Studio/src/app.rs) | [`NexoraApp`](file:///d:/Rust/Nexora%20Studio/src/app.rs) | State aplikasi utama, sinkronisasi request capture (`DisplayCapture`, `WindowCapture`, `WebcamDevice`, `MediaSource`, `ImageSource`, `ImageSlideShow`), sinkronisasi audio media **hanya saat Program (`LIVE`)**, serta pengatur visibilitas modal dialog (`show_source_properties_dialog`, `show_source_filters_dialog`, `show_slideshow_manager_dialog`). |
| [`src/settings.rs`](file:///d:/Rust/Nexora%20Studio/src/settings.rs) | [`AppSettings`](file:///d:/Rust/Nexora%20Studio/src/settings.rs) | Menyimpan preferensi konfigurasi ke `%APPDATA%\Nexora\NexoraStudio\config\settings.json`. |
| [`src/studio/mod.rs`](file:///d:/Rust/Nexora%20Studio/src/studio/mod.rs) | [`StudioState`](file:///d:/Rust/Nexora%20Studio/src/studio/mod.rs), [`TransitionEffect`](file:///d:/Rust/Nexora%20Studio/src/studio/mod.rs) | Manajemen `Scene`, pemisahan Preview/Program, efek transisi **`Cut`** & **`Fade`**, dan sinkronisasi audio transport media. |
| [`src/studio/scene.rs`](file:///d:/Rust/Nexora%20Studio/src/studio/scene.rs) | [`Scene`](file:///d:/Rust/Nexora%20Studio/src/studio/scene.rs) | Penyimpanan daftar layer [`SourceItem`](file:///d:/Rust/Nexora%20Studio/src/studio/source.rs) dengan urutan studio (**index 0 = layer teratas / foreground**, **index n-1 = layer terbawah / background**). |
| [`src/studio/source.rs`](file:///d:/Rust/Nexora%20Studio/src/studio/source.rs) | [`SourceItem`](file:///d:/Rust/Nexora%20Studio/src/studio/source.rs), [`KeyFilterKind`](file:///d:/Rust/Nexora%20Studio/src/studio/source.rs), [`SourceFilterConfig`](file:///d:/Rust/Nexora%20Studio/src/studio/source.rs), [`ChromaKeyPresetColor`](file:///d:/Rust/Nexora%20Studio/src/studio/source.rs), `SourceItem::audio_filters`/`supports_audio_filters()` | Definisi tipe source, rantai filter **audio** (`Vec<AudioFilter>`, hanya untuk 3 jenis audio capture), konfigurasi filter video transparansi real-time (**`Chroma Key`**, **`Color Key`**, **`Luma Key`**), teks bantuan hover, dan preset transformasi/crop. |
| [`src/capture/mod.rs`](file:///d:/Rust/Nexora%20Studio/src/capture/mod.rs) | [`CaptureCatalog`](file:///d:/Rust/Nexora%20Studio/src/capture/mod.rs), [`LiveCaptureEngine`](file:///d:/Rust/Nexora%20Studio/src/capture/mod.rs) | Deteksi perangkat asli (monitor, window, webcam, GPU) serta pemrosesan pixel buffer real-time untuk **`apply_key_filter_to_rgba`** (Chroma keying YCbCr + Spill suppression, Color keying Euclidean distance, dan Luma keying Rec.709). |
| [`src/platform/mod.rs`](file:///d:/Rust/Nexora%20Studio/src/platform/mod.rs) | Dialog Win32, WASAPI Audio, Projector Helper | Pemutar audio media native Windows (`IMFSourceReader` + WASAPI), dialog *multi-select* file (`OFN_ALLOWMULTISELECT`), dan kontrol *borderless fullscreen projector*. |
| [`src/audio/mod.rs`](file:///d:/Rust/Nexora%20Studio/src/audio/mod.rs) | [`AudioMixerState`](file:///d:/Rust/Nexora%20Studio/src/audio/mod.rs), [`SourceLevel`](file:///d:/Rust/Nexora%20Studio/src/audio/mod.rs), [`SourceMeterRequest`](file:///d:/Rust/Nexora%20Studio/src/audio/mod.rs) | Native Windows WASAPI Audio Mixer: 2 bus global (`Desktop Audio`, `Mic / Aux`) + registry meter live per source bus (`source_levels`, `#[serde(skip)]`), pembacaan hardware peak (`poll_peak`), dan ballistics meter frame-independent. |
| [`src/audio/bus.rs`](file:///d:/Rust/Nexora%20Studio/src/audio/bus.rs) | [`SourceBusSpec`](file:///d:/Rust/Nexora%20Studio/src/audio/bus.rs), [`SourceBusKind`](file:///d:/Rust/Nexora%20Studio/src/audio/bus.rs), [`AudioFilter`](file:///d:/Rust/Nexora%20Studio/src/audio/bus.rs) | Deskriptor runtime **Per-Source Audio Bus (OBS-style)**. Membawa `Vec<AudioFilter>` (rantai filter audio milik source). `routing_signature()` mengabaikan fader/mute/filter (agar device tidak di-*reopen*), `spec_signature()` mencakup semuanya. Tidak di-*serialize* ke project. |
| [`src/audio/filters.rs`](file:///d:/Rust/Nexora%20Studio/src/audio/filters.rs) | [`AudioFilter`](file:///d:/Rust/Nexora%20Studio/src/audio/filters.rs), [`AudioFilterKind`](file:///d:/Rust/Nexora%20Studio/src/audio/filters.rs), [`AudioFilterChainRuntime`](file:///d:/Rust/Nexora%20Studio/src/audio/filters.rs) | **Rantai filter audio per-source (OBS-style)**: daftar berurutan yang bisa ditambah/hapus/diurutkan/di-bypass, berbeda dari filter video yang hanya satu aktif. Implementasi DSP murni offline: `Gain` (trim -30..+30 dB dengan ramping anti-zipper), `NoiseGate` (hysteresis 3 dB + attack/hold/release + DC blocker), `NoiseSuppression` (downward expander broadband + rolloff 12 kHz). `clean_mic_preset()` menyediakan preset *Clean Mic*, dan `filters_signature()` dipakai encoder untuk mendeteksi perubahan parameter tanpa membuka ulang stream WASAPI. |
| [`src/audio/endpoints.rs`](file:///d:/Rust/Nexora%20Studio/src/audio/endpoints.rs) | `resolve_endpoint`, `score_device_name`, `normalize_device_name`, `find_pid_for_executable` | Resolusi nama device WASAPI yang toleran (exact → normalized → prefix → token coverage) via `EnumAudioEndpoints` + `PKEY_Device_FriendlyName`, serta pencarian PID via ToolHelp. Dipakai bersama oleh meter mixer dan encoder recording. |
| [`src/encoder/writer.rs`](file:///d:/Rust/Nexora%20Studio/src/encoder/writer.rs) | [`Mp4RecordingWriter`](file:///d:/Rust/Nexora%20Studio/src/encoder/writer.rs), `EncoderPacket`, `ActiveSourceBus` | Sinkronisasi Media Foundation. Mixer audio menjumlahkan **N bus**: `Desktop Audio` + `Mic / Aux` + seluruh per-source bus (masing-masing punya WASAPI stream, resampler 48 kHz, dan `AudioFilterChainRuntime` dari rantai filter source), lalu `soft_limit`. Bus global (`Desktop Audio`, `Mic / Aux`) tetap **mentah tanpa filter** sesuai model OBS, sehingga mic tidak pernah diproses dua kali. Menghindari audio ganda bila bus source & bus global menunjuk endpoint yang sama. |
| [`src/encoder/endpoint.rs`](file:///d:/Rust/Nexora%20Studio/src/encoder/endpoint.rs) | `CaptureEndpoint`, `LoopbackSilencePump`, `open_for_spec` | Pembukaan stream WASAPI: full-desktop loopback (render), capture input, dan **per-process loopback** (`ActivateAudioInterfaceAsync` + `VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK`, Windows build ≥ 20348) dengan fallback ke loopback desktop penuh. |

---

### C. Modul Antarmuka Pengguna (`src/ui/`)
| Path File | Fungsi Utama | Detail Tampilan & Interaksi |
| :--- | :--- | :--- |
| [`src/ui/icons.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/icons.rs) | [`install_tabler_icons`](file:///d:/Rust/Nexora%20Studio/src/ui/icons.rs) | Menyediakan konstanta Tabler Icons offline (`FILTER`, `ADJUSTMENTS`, `WAND`, `DEVICE_DESKTOP`, `EYE`, dll.). |
| [`src/ui/theme.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/theme.rs) | [`ThemePalette`](file:///d:/Rust/Nexora%20Studio/src/ui/theme.rs) | Palet warna tema Dark Mode dan Light Mode. |
| [`src/ui/workspace.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/workspace.rs) | [`render_preview_workspace`](file:///d:/Rust/Nexora%20Studio/src/ui/workspace.rs) | Dua viewport 16:9 (Preview & Program), jembatan transisi `Cut` & `Fade`, penyesuaian kursor mouse yang tepat (kursor panah standar untuk klik biasa, kursor 4-arah `Move` hanya saat drag posisi, kursor `Resize` hanya saat mengarahkan handle layer yang tidak terkunci), serta render layer berfilter transparansi. |
| [`src/ui/dock.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/dock.rs) | [`render_bottom_studio_dock`](file:///d:/Rust/Nexora%20Studio/src/ui/dock.rs) | 4 panel bawah (`Scenes`, `Source`, `Audio Mixer`, `Controls`). Footer panel `Scenes` hanya memuat tombol **`+`** (aksi `Duplicate` / `Rename` / `Delete` tersedia di menu klik kanan baris scene). Footer panel Sources dilengkapi tombol **`+ Add Source`** dan dropdown **`▾`**, menu pop-up kaya dengan ikon + nama + deskripsi subtitle, klik kanan baris `Source` dengan submenu **`Filters ▸`** (`None`, `Chroma Key`, `Color Key`, `Luma Key`), dan indikator badge filter pada kartu layer. Panel `Audio Mixer` memiliki pemilih gaya tampilan di header (ikon `icons::LAYOUT_GRID`): **`Strips`** (blok penuh, ada pemilih device), **`List`** (baris ringkas: MUTE, nama, tag `IN`/`OUT`/`APP` + `PGM`/`PRV`, fader, %), dan **`Columns`** (kolom berdiri berjejer ke kanan dengan fader vertikal). Gaya tersimpan di `settings.audio_mixer_view_mode` (`AudioMixerViewMode`). |
| [`src/ui/dialogs.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/dialogs.rs) | [`render_dialogs`](file:///d:/Rust/Nexora%20Studio/src/ui/dialogs.rs) | Merender dialog modal **`+ Add Source`** 2-kolom (daftar tipe layer di kiri, konfigurasi + validasi nama & file di kanan), modal **Properties**, modal **Manage Slide Show Images** (thumbnail `84×48 px`), modal **Filters for '<Source>'** (slider Similarity, Smoothness, Spill Reduction, Luma, Pipet Warna Layar), serta jendela **Projector Layar Kedua**. |

---

## 5. Standar Konsistensi Aplikasi & UI/UX (WAJIB DIPATUHI KEDEPANNYA)

1. **Urutan Layer `Source` (Z-Order Standar OBS Studio):**
   - Baris **paling atas di panel `Source` (`index 0`) adalah layer PALING DEPAN (foreground/overlay)**.
   - Baris **paling bawah di panel `Source` (`index n-1`) adalah layer PALING BELAKANG (background)**.
   - Menggambar kanvas & projector wajib menggunakan `sources.iter().rev()`.
   - Hit-testing klik di kanvas menggunakan `sources.iter()`.
   - Penambahan source baru menggunakan `insert(0, item)`.
   - Bounding box & handle seleksi digambar setelah seluruh layer selesai digambar agar selalu terlihat di atas kanvas.

2. **Aturan Kursor Mouse & Drag-Drop Window (Standard vs Drag & Drop):**
   - **Drag Modal Window Hanya Lewat Title Bar:**
     - Seluruh jendela modal (`Settings`, `Properties`, `Filters`, `Manage Slide Show Images`, `Rename`, `Save Layout Preset`) wajib memanggil `block_window_body_drag(ui)`. Pengguna hanya boleh menggeser posisi window dengan menarik **Title Bar** saja. Mengklik atau menggeser di dalam isi form/body tidak akan menggeser window.
   - **Kursor Standar untuk Hover & Klik:**
     - Untuk klik biasa (memilih baris kartu di panel `Scenes`, `Source`, tombol dialog, atau hovering kanvas biasa): **WAJIB menggunakan kursor standar (`CursorIcon::Default`)**.
   - **Kursor Khusus Hanya Aktif Saat Interaksi Terkait:**
     - Mengarahkan kursor tepat ke 8 handle resize/crop pada source yang **tidak terkunci**: gunakan kursor resize terarah (`ResizeNwSe`, `ResizeNeSw`, `ResizeVertical`, `ResizeHorizontal`).
     - Mengarahkan kursor ke dalam layer terpilih di kanvas: gunakan kursor standar (`CursorIcon::Default`), dan saat mouse sedang ditekan dan digeser aktif (*actively dragging*): gunakan `CursorIcon::Grabbing`.
     - Jika ada modal dialog yang sedang terbuka (`app.is_any_modal_open()`), kanvas otomatis menonaktifkan hit-testing dan tidak mengubah kursor mouse.
     - Jika layer dalam keadaan terkunci (`src.locked == true`), kursor harus selalu tetap berupa kursor standar (`CursorIcon::Default`).

3. **Sistem Filter Transparansi Video & Gambar (`Filters`) + Screen Color Picker:**
   - Setiap source visual (gambar, video, webcam, slideshow, window, display) mendukung filter transparansi real-time:
     - **Chroma Key**: Menghapus background hijau/biru secara halus dengan fitur *Spill Reduction*.
     - **Color Key**: Menghapus satu warna solid spesifik (custom RGB / Color Picker / Black / White).
     - **Luma Key**: Menghapus berdasarkan luminansi gelap-terang (potong background hitam pekat seperti api/partikel atau potong background putih).
   - **Screen Color Picker / Pipet Warna Layar:**
     - Menyediakan tombol `Pipet Layar` (`COLOR_PICKER`) di samping color swatch pada Chroma Key & Color Key.
     - Saat aktif, kursor berubah menjadi `CursorIcon::Crosshair` dan mengambil sampel warna RGB riil dari area layar manapun via Win32 API (`GetPixel`), dilengkapi floating magnifier pill dan pembatalan dengan tombol `Esc` atau klik kanan.
   - Menu klik kanan pada `Source` wajib menyediakan submenu cepat **`Filters ▸`** beserta keterangan tooltip lengkap tentang kegunaan masing-masing filter saat kursor mendekat.
   - Jendela dialog **`Filters for '<Source>'`** wajib menyediakan penjelasan fungsi filter di bagian atas, tombol preset instan, dan slider yang halus.

4. **Konsistensi Ukuran Jendela Modal & Input Width:**
   - Jendela modal wajib mengunci lebar (`resizable(false)`, `max_width`) agar tidak pernah memanjang ke kanan sendiri saat memuat path panjang.
   - Lebar dropdown dikunci seragam (`PROP_FIELD_W = 280.0`, `SETTINGS_FIELD_W = 270.0`, `FLT_FIELD_W = 270.0` dengan `.truncate()`).
   - Tombol ikon menggunakan `min_size(Vec2::new(w, 22.0..26.0))` atau `paint_icon_action_btn` agar tidak mepet dengan garis batas tombol.

5. **Audio Media Playback Hanya Aktif Saat Live (Program):**
   - Suara dari `MediaSource` hanya boleh berbunyi ketika sudah ditransisikan ke **Program (`LIVE`)**, bukan saat baru disiapkan di Preview.

5b. **Per-Source Audio Bus (OBS-Style) & Isolasi Penuh dari Mixer Global:**
   - Setiap source audio (`AudioInputCapture`, `AudioOutputCapture`, `ApplicationAudioCapture`) yang berada di **Preview atau Program** wajib memiliki **bus audio sendiri** yang muncul sebagai kartu baris penuh di panel `Audio Mixer`, lengkap dengan tombol `MUTE`, badge arah (`IN`/`OUT`/`APP`), badge status (`PGM`/`PRV`), pemilih device full-width, slider volume, readout dB, dan meter 3-zona ber-*peak hold*.
   - **Isolasi Wajib:** Perubahan pada bus source **TIDAK BOLEH** saling memengaruhi dengan `Desktop Audio` / `Mic / Aux`. Mute pada `Mic / Aux` tidak boleh menyenyapkan `AudioInputCapture`.
   - **Hanya Live di Program yang Masuk Rekaman:** Bus source hanya terdengar di rekaman saat source tersebut berada di `program_scene()` dan `visible`. Source yang hanya ada di Preview tetap menampilkan meter (agar device bisa diverifikasi) tetapi di-*silence* pada mix hingga ditransisikan.
   - **Tanpa Audio Ganda:** Bila sebuah bus source menunjuk endpoint WASAPI yang sama dengan bus global, bus global di-*suppress* otomatis (dibandingkan via `IMMDevice::GetId`).
   - **Tanpa Reopen Sia-Sia:** Perubahan fader/mute hanya di-*update in place*. WASAPI stream hanya dibuka ulang saat *routing* berubah (`SourceBusSpec::routing_signature`), bukan tiap frame.
   - Device global (`Desktop Audio`, `Mic / Aux`) kini benar-benar dihormati oleh encoder (nama device dari ComboBox di-*resolve* via `PKEY_Device_FriendlyName`; sebelumnya selalu memakai device default Windows).
   - `ApplicationAudioCapture` memakai **per-process loopback** (`ActivateAudioInterfaceAsync` + `VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK`, Windows build >= 20348). Jika PID tidak ditemukan atau platform menolak, otomatis fallback ke loopback desktop penuh.
   - Semua bus (global + per-source) dijumlahkan menjadi **satu track AAC** agar MP4 tetap kompatibel universal (arsitektur bus berbasis `Vec<SourceBusSpec>` siap dikembangkan ke multi-track tanpa refactor besar).

5c. **Audio Filters per Source Audio (Gain, Noise Gate, Noise Suppression):**
   - Filter audio hanya boleh menempel pada 3 jenis source audio capture (`AudioInputCapture`, `AudioOutputCapture`, `ApplicationAudioCapture`) via `SourceItem::audio_filters: Vec<AudioFilter>`. Bus global (`Desktop Audio`, `Mic / Aux`) tetap **mentah / tanpa filter** persis OBS, agar endpoint WASAPI yang sama tidak diproses dua kali.
   - **Model Rantai Berurut:** Beda dengan filter video (`SourceFilterConfig` = satu key filter aktif), audio memakai rantai `Vec<AudioFilter>` di [`src/audio/filters.rs`](file:///d:/Rust/Nexora%20Studio/src/audio/filters.rs). `Gain` + `Noise Gate` + `Noise Suppression` harus bisa aktif bersamaan. Urutan penting, tiap entri bisa di-bypass (`AudioFilter::enabled`).
   - Eksekusi terjadi di thread encoder via `AudioFilterChainRuntime` (`from_chain` hanya meng-instansiasi filter yang `enabled`). Rantai kosong / semua bypass wajib transparan 100% (`is_transparent()` mengembalikan sampel apa adanya tanpa alokasi per frame).
   - Parameter: `Gain` (−30..+30 dB, smoothing 20 ms), `Noise Gate` (Close Threshold −80..0 dB, Open Threshold otomatis +3 dB sebagai histeresis, Attack 0.5..50 ms, Hold 0..1000 ms, Release 10..2000 ms), `Noise Suppression` (downward expander broadband 2:1 + rolloff 12 kHz, Suppression 0..30 dB, Sensitivity 0..100 %).
   - **`clean_mic_preset()` menggantikan `MicConditioner`:** kondisioning mic yang dulu berjalan otomatis dan tak terlihat di setiap bus mic sudah **dihapus total**. Kini eksplisit lewat tombol `Clean Mic Preset` (Noise Suppression 12 dB + Noise Gate −45 dB) sehingga tidak terjadi *double-gating*. Konsekuensi: mic yang baru ditambahkan terekam mentah sampai preset dipasang.
   - **Wajib Tanpa Reopen Sia-Sia:** `SourceBusSpec::routing_signature()` sengaja mengabaikan filter (juga fader/mute) agar menggeser slider tidak menutup/membuka ulang WASAPI stream. Sebaliknya `spec_signature()` mencakup `filters_signature()` sehingga encoder tetap menerima perubahan parameter secara *in-place* lewat `ActiveSourceBus::update_spec`.
   - `filters_signature()` hanya memuat `tag:enabled:parameter` dan **sengaja tidak memuat `Uuid`**, supaya mengurutkan ulang filter atau undo tidak keliru dianggap perubahan parameter.
   - Filter audio ikut di-*serialize* sebagai bagian `SourceItem` (`#[serde(default)] pub audio_filters`), jadi project lama tetap bisa dibuka tanpa migrasi. (`SourceBusSpec` tetap murni runtime.)
   - **Standarisasi UI/UX Dialog Audio Filters & Baris Slider:**
     - Rel slider memiliki visual alur cekung/groove (`visuals.widgets.inactive.bg_fill` sunken tone) dan trailing fill aksen lembut (`visuals.slider_trailing_fill = true`) sehingga batang slider tampak jelas dan tidak melayang.
     - Layout formulir parameter tersusun rapi dalam 4 kolom presisi (`standard_slider_param_row`): Label (130 px), Slider Rail murni (185 px), Kotak Nilai abu-abu (64x22 px), dan Kotak Reset abu-abu (64x22 px).
     - Seluruh kotak abu-abu (Value box dan Reset box) serta label teks menggunakan font size identik 11.5 pt dan ukuran kotak identik 64x22 px sehingga seluruh tepi kotak lurus rata secara vertikal.
     - Indikator status per kartu filter memakai badge pill `ACTIVE` (teal) dan `BYPASSED` (muted) yang kontras.
     - Penjelasan panjang dan panduan broadcast dimasukkan ke dalam collapsible toggle (`Filter Guide & Tuning Tips`) default tertutup agar kartu filter tetap ramping, ringkas, dan bebas scroll yang berlebihan.
6. **Ikon Offline Tabler Icons & Larangan Fitur Streaming:**
   - Selalu gunakan konstanta ikon dari [`src/ui/icons.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/icons.rs).
   - Dilarang menambahkan fitur *live streaming* / RTMP / cloud / login.

7. **Fitur Add Source & Dialog Konfigurasi Layer:**
   - Tombol **`+ Add Source`** pada toolbar footer panel Sources membuka dialog modal 2-kolom (`render_add_source_dialog`):
     - Kolom kiri: Daftar seluruh jenis source dengan ikon, judul, status badge, dan deskripsi singkat.
     - Kolom kanan: Formulir konfigurasi nama source otomatis/unik (`Scene::generate_unique_source_name`), pemilih device/file native, opsi buka properties, pesan error validasi, dan tombol tambah.
   - Menu pop-up **Add Source** (`▾` dan klik kanan) mempertahankan desain baris tunggal ringkas (*compact single-line*) yang bersih dengan tombol peluncur dialog di bagian atas, serta dilengkapi dukungan scroll mouse wheel dan auto-scroll dengan indikator panah Tabler Icons (`icons::CHEVRON_UP` & `icons::CHEVRON_DOWN`) yang otomatis bergeser saat kursor mendekat atau diklik.
   - Setiap source baru disisipkan ke scene preview aktif pada index 0 (layer foreground paling depan).
   - **PowerPoint / PPT Source**:
     - Merender presentasi `.pptx` / `.ppt` ke resolusi penuh 1080p menggunakan COM Automation latar belakang (`Presentation.Export`) dengan cache otomatis di AppData.
     - Kontrol navigasi slide (`◀` / `▶`) tersedia langsung pada kartu layer di panel Sources, dialog Properties, tombol presenter remote pointer (`PageDown`/`PageUp`/`Space`/Arrows), serta klik pointer mouse langsung pada viewport Program Output dan Projector Layar Kedua. Perubahan slide selalu disinkronkan secara real-time ke Program Output (`program_scene_snapshot`).
   - **Waktu / Jam & Timer (`TimeSource`)**:
     - Mendukung 3 mode:
       1. **Waktu Sekarang (Real-time Clock)**: Menampilkan waktu sistem Windows saat ini (format 24 jam atau 12 jam AM/PM, dengan opsi titik dua solid `:` tanpa kedip).
       2. **Hitung Maju (Stopwatch / Count-up)**: Menghitung maju dari 00:00:00 (jam, menit, detik).
       3. **Hitung Mundur (Countdown Timer)**: Menghitung mundur dari durasi target ke 00:00:00 (bisa menampilkan teks kustom seperti `TIME'S UP!` atau `SELESAI`).
     - **Pilihan Efek Visual (`TimeVisualEffect`)**:
       - `DigitalNeon`: Kotak LED bercahaya ala panggung / scoreboard cyberpunk dengan border neon dan glow shadow.
       - `TvCornerBadge`: Kapsul broadcast berita TV dengan label badge di kiri dan waktu di kanan.
       - `MinimalistClean`: Teks mengambang transparan dengan bayangan lembut (*soft drop shadow*).
       - `DarkGlassCard`: Kartu glassmorphism gelap modern dengan hairline border dan lampu indikator status berkedip (*pulsing dot*).
     - **Kontrol Waktu Maju & Mundur**:
       - Tombol Start/Pause dan Reset.
       - Tombol cepat `-1m`, `+1m`, `-10s`, `+10s` untuk memajukan atau memundurkan waktu secara presisi.
       - Tombol Play/Pause dan Reset langsung tersedia di kartu baris layer panel Sources.
     - **Tipografi & Skalabilitas**:
       - Pilihan jenis font: `Monospace Digital` (angka sejajar tanpa jitter), `Modern Sans-Serif`, dan `Serif Klasik`.
       - Ukuran font bebas disesuaikan (`font_size` slider dari `16px` hingga `140px` untuk studio panggung besar).
       - Kustomisasi warna teks, background, dan aksen/border penuh.
   - Browser Source mendukung input URL dan parameter overlay siaran.


   - **Alur Tambah Scene (Add Scene) & Default Empty Sources:**
     - Tombol + pada dock Scenes atau opsi Add Scene... pada menu klik kanan membuka modal dialog *Add Scene*.
     - Modal dialog mem-prefill nama scene unik otomatis (Scene N), mengarahkan fokus ke teks input, dan mendukung tombol Enter / tombol OK untuk konfirmasi cepat atau Esc / Batal untuk membatalkan.
     - Scene baru yang ditambahkan selalu dimulai dengan layer yang **KOSONG** (sources: Vec::new()) tanpa menambahkan layer capture default apapun.


   - **Splitter Pemisah Canvas & Dock Bawah:**
     - Area garis pemisah vertikal antara viewport kanvas atas dan dock panel bawah menggunakan deteksi hit-test splitter (+/- 4px) dengan kursor arah vertikal (CursorIcon::ResizeVertical) dan highlight garis aksen interaktif saat di-hover maupun saat proses drag resizing berlangsung.

8. **Sistem Menu Transform & Manipulasi Kanvas Langsung (OBS-Style):**
   - **Submenu Transform Lengkap**:
     - Tersedia di klik kanan layer pada kanvas Preview dan klik kanan kartu Source pada dock Sources.
     - Opsi transformasi:
       - `Rotate 90 degrees CW` (+90° searah jarum jam)
       - `Rotate 90 degrees CCW` (-90° berlawanan arah jarum jam)
       - `Rotate 180 degrees` (180°)
       - `Flip Horizontal` (balik sumbu horizontal / cermin X)
       - `Flip Vertical` (balik sumbu vertikal / cermin Y)
       - `Fit to screen` (`Ctrl+F` — menyesuaikan layer ke kanvas tanpa merusak rasio aspek)
       - `Stretch to screen` (`Ctrl+S` — mengisi penuh kanvas 1920x1080)
       - `Center to screen` (`Ctrl+D` — menempatkan layer tepat di tengah kanvas)
       - `Center Vertically` (menyelaraskan posisi tengah secara vertikal)
       - `Center Horizontally` (menyelaraskan posisi tengah secara horizontal)
       - `Reset Transform` (`Ctrl+R` — mengembalikan rotasi, flip, dan skala layer ke default)
   - **Interaksi Kanvas Langsung**:
     - Mengklik kanan objek pada kanvas Preview otomatis memilih layer tersebut (*hit-testing*) dan langsung membuka menu konteks dengan submenu Transform.
     - Shortcut keyboard aktif langsung di kanvas saat layer dipilih: `Ctrl+F` (Fit), `Ctrl+S` (Stretch), `Ctrl+D` (Center), dan `Ctrl+R` (Reset).
     - Rendering rotasi dan flip menggunakan `egui::Mesh::with_texture` dan pemetaan UV dinamis sehingga rotasi dan flipping dirender secara native, halus, dan instan.
     - Zero-drift transform resize: Proyeksi pergerakan mouse delta piksel dihitung langsung terhadap vektor diagonal layar layer (delta_px.dot(diag) / diag_sq), sehingga handle resize 8 titik menempel 1:1 di ujung kursor mouse tanpa over-sensitivitas atau lag pada aspek rasio layer lebar/tipis.
