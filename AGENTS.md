# AGENTS.md — Panduan Cepat untuk Agen AI di Workspace NEXORA Studio

Selamat datang di repositori **NEXORA Studio** (Aplikasi Desktop Screen Recording & Studio Presentation Native Windows berbasis Rust & `egui`).

Sebelum melakukan perubahan kode, **baca peta arsitektur lengkap di [`ARCHITECTURE_MAP.md`](file:///d:/Rust/Nexora%20Studio/ARCHITECTURE_MAP.md)**.

## Ringkasan Cepat Aturan Wajib

1. **Perintah Build & Test (Wajib di PowerShell):**
   - `cargo` berada di `$env:USERPROFILE\.cargo\bin`.
   - Toolchain menggunakan `stable-x86_64-pc-windows-gnu` dan `target-dir` berada di `D:/Rust/.nexora-target` (lihat [`.cargo/config.toml`](file:///d:/Rust/Nexora%20Studio/.cargo/config.toml)).
   - Selalu hentikan proses `nexora-studio` sebelum kompilasi agar tidak terkena *file lock*:
     ```powershell
     $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
     Get-Process nexora-studio -ErrorAction SilentlyContinue | Stop-Process -Force
     cargo test
     cargo build
     ```
   - Lokasi binary: `D:\Rust\.nexora-target\debug\nexora-studio.exe`.

2. **Sistem Ikon Offline (Tabler Icons):**
   - Seluruh 5.936 ikon **Tabler Icons** sudah diunduh secara offline di [`assets/icons/`](file:///d:/Rust/Nexora%20Studio/assets/icons) (`tabler-icons.ttf`, `tabler-sprite.svg`, `tabler-icons-codepoints.json`) dan di-*embed* melalui [`src/ui/icons.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/icons.rs).
   - Jangan gunakan karakter emoji mentah yang tidak didukung oleh font bawaan; gunakan konstanta dari [`src/ui/icons.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/icons.rs).

3. **Konvensi Konsistensi UI/UX & Arsitektur Studio:**
   - **Studio Mode & Transisi:** Pertahankan **Studio Mode** (*Preview* di kiri, *Program* di kanan) dengan rasio kanvas `16:9` serta jembatan transisi tengah yang simpel (`TRANSITION ->` dengan pilihan efek **`Cut`** & **`Fade`**).
   - **Urutan Layer `Source` (Z-Order Standar OBS):**
     - Baris **atas** di panel `Source` (`index 0`) = **di depan layar (foreground)**.
     - Baris **bawah** di panel `Source` (`index terakhir`) = **di belakang layar (background)**.
     - Render kanvas & projector wajib menggunakan `sources.iter().rev()`, sedangkan *hit-testing* klik kanvas menggunakan `sources.iter()`.
   - **Seleksi Baris Full-Width (`Scenes` & `Source`):**
     - Setiap item di panel `Scenes` dan `Source` wajib berupa kartu baris penuh (*full-width row card*) sehingga pengguna dapat memilih item dengan mengklik di mana saja pada baris tersebut (tidak harus tepat di atas teks).
     - Kartu `Scenes` wajib menampilkan indikator visual `👁 PREVIEW` (Teal) dan `● LIVE` (Merah).
   - **Konsistensi Form Modal & Padding Tombol:**
     - Kunci lebar jendela modal (`Properties`, `Settings`, `Manage Slide Show Images`) dan seragamkan lebar `ComboBox` (`PROP_FIELD_W = 280.0`, `SETTINGS_FIELD_W = 270.0` dengan `.truncate()`) agar jendela tidak pernah memanjang sendiri ke kanan.
     - Berikan ukuran minimum vertikal (`min_size` tinggi `20px–26px` atau `paint_icon_action_btn`) pada tombol agar ikon dan teks tidak pernah mepet dengan garis batas tombol.
   - **Audio Media Hanya Aktif Saat Live (`Program`):**
     - Suara dari `MediaSource` hanya boleh diputar ketika scene sudah ditransisikan ke **Program (`LIVE`)**, bukan saat baru disiapkan di Preview.
   - **Drag Modal Window Hanya Lewat Title Bar:**
     - Semua jendela modal (`Settings`, `Properties`, `Filters`, `Manage Slide Show Images`, `Rename`, `Save Layout Preset`) wajib memanggil `block_window_body_drag(ui)` di awal isi modal. Window hanya boleh digeser/dipindahkan melalui **Title Bar** saja untuk menjaga konsistensi dan mencegah pergeseran window yang tidak disengaja saat berinteraksi dengan widget/form.
   - **Standar Ikon Kursor & Drag-Drop:**
     - Kursor standar saat hovering/klik biasa adalah panah default (`CursorIcon::Default`).
     - Kursor arah resize langsung aktif secara instan (zero-latency) dengan prioritas 4 sudut diagonal (margin 12px) dan 4 sisi horizontal/vertikal (margin 7px) tanpa jeda/telat.
     - Kursor drag (`CursorIcon::Grabbing`) hanya aktif ketika mouse sedang benar-benar ditahan dan digeser (*actively dragging*), tidak boleh muncul sembarangan di seluruh area kanvas.
     - Jika modal dialog sedang terbuka (`app.is_any_modal_open()`), kanvas tidak boleh merebut kursor atau memicu resize/drag.
   - **Screen Color Picker / Pipet Warna Layar:**
     - Fitur pemilih warna pada filter (`Chroma Key`, `Color Key`) menyediakan tombol `Pipet Layar` (`COLOR_PICKER`) yang memungkinkan pengambilan sampel warna RGB asli dari area layar manapun secara real-time via Win32 API (`GetPixel`), dilengkapi floating pill pembesar dan pembatalan mudah via tombol `Esc` atau klik kanan.
   - **Projector Layar Kedua:**
     - Klik kanan kanvas -> `Open Preview/Program Projector` langsung melakukan *auto-fullscreen* ke monitor yang dipilih dan langsung tertutup saat tombol **`Esc`** ditekan.
   - **Fitur Add Source & Dialog Konfigurasi Layer:**
     - Tombol `+ Add Source` pada footer panel Sources membuka dialog modal modern 2 kolom (`render_add_source_dialog`), sedangkan menu pop-up (`▾` dan klik kanan) menggunakan format satu baris ringkas (*single-line*) bersih dengan scroll roda mouse (*mouse wheel*) serta auto-scroll indikator panah Tabler Icons (`icons::CHEVRON_UP` & `icons::CHEVRON_DOWN`).
     - Mendukung beragam jenis source termasuk **PowerPoint / PPT Source** (ekspor otomatis slide 1080p via COM dengan navigasi slide pointer / remote) serta **Waktu / Jam & Timer (`TimeSource`)**:
       - Multi-mode: Waktu Sekarang (Jam Real-Time sinkron waktu Windows), Hitung Maju (Stopwatch), dan Hitung Mundur (Countdown Timer).
       - Efek Visual: `DigitalNeon` (glow frame LED), `TvCornerBadge` (kapsul broadcast TV), `MinimalistClean` (transparan melayang dengan soft shadow), dan `DarkGlassCard` (glassmorphism modern dengan pulsing indicator dot).
       - Kontrol Waktu Interaktif: Start/Pause, Reset, tombol penyesuaian cepat `+1m`/`-1m`/`+10s`/`-10s` (bisa memajukan & memundurkan waktu), serta tombol quick play/pause & reset langsung di kartu row Sources dock.
       - Tipografi & Skalabilitas: Font type (`Monospace`, `Proportional`, `Serif`), font size slider (`16px..140px`), opsi format (tampilkan jam, titik dua solid `: ` tanpa kedip, format 24 jam), dan penyesuaian warna teks/background/aksen penuh.
     - Setiap source yang ditambahkan melalui dialog divalidasi namanya (harus unik dan tidak boleh kosong, otomatis diisi prefix unik seperti `Display Capture 2`), file diperiksa keberadaannya secara native via dialog picker, lalu disisipkan ke scene preview aktif pada index 0 (layer foreground terdepan).
   - **Alur Tambah Scene (Add Scene) & Default Empty Sources:**
     - Tombol + pada footer panel Scenes dan klik kanan -> Add Scene... memunculkan dialog modal konfirmasi nama scene baru terlebih dahulu (*Add Scene dialog*).
     - Input nama scene otomatis di-prefill dengan nama default unik (misal Scene 3). Pengguna dapat langsung menekan Enter atau mengklik OK untuk memakai nama default, atau mengetik nama kustom terlebih dahulu.
     - Scene baru yang ditambahkan selalu memiliki daftar layer/source yang **KOSONG** secara default (sources: Vec::new()) tanpa menambahkan layer default apapun.
   - **Menu Transform & Manipulasi Kanvas Langsung (OBS-Style):**
     - Klik kanan pada layer di Preview canvas atau panel Sources menampilkan submenu **Transform ▸**:
       - Rotasi: `Rotate 90 degrees CW`, `Rotate 90 degrees CCW`, `Rotate 180 degrees`.
       - Balik: `Flip Horizontal`, `Flip Vertical`.
       - Penyesuaian Kanvas: `Fit to screen` (`Ctrl+F`), `Stretch to screen` (`Ctrl+S`), `Center to screen` (`Ctrl+D`), `Center Vertically`, `Center Horizontally`.
       - Reset: `Reset Transform` (`Ctrl+R`).
     - Manipulasi langsung di canvas dapat dilakukan menggunakan tombol pintas keyboard di atas layer terpilih (`Ctrl+F`, `Ctrl+S`, `Ctrl+D`, `Ctrl+R`) serta klik kanan langsung pada objek layer untuk memilih dan membuka menu Transform.
   - **Larangan Mutlak:** Dilarang menambahkan fitur *live streaming* / RTMP / cloud / login.
