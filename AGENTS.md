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
   - **Studio Mode & Transisi:**
     - Pertahankan **Studio Mode** (*Preview* di kiri, *Program* di kanan) dengan rasio kanvas `16:9` serta jembatan transisi tengah yang simpel (`TRANSITION ->` dengan pilihan efek **`Cut`** & **`Fade`**).
     - **Isolasi Penuh Properti Source Preview & Program:** Perubahan apapun pada properti source (warna, ukuran font, ketebalan border, badge, slide PPT, filter, transform kanvas, dll.) di Preview **TIDAK BOLEH** langsung tercermin ke Program/Output sebelum pengguna mengklik tombol **`TRANSITION ->`**. Fungsi sinkronisasi runtime hanya boleh memperbarui audio/transport playback media live dan tidak boleh menimpa konfigurasi visual snapshot Program.
   - **Urutan Layer `Source` (Z-Order Standar OBS):**
     - Baris **atas** di panel `Source` (`index 0`) = **di depan layar (foreground)**.
     - Baris **bawah** di panel `Source` (`index terakhir`) = **di belakang layar (background)**.
     - Render kanvas & projector wajib menggunakan `sources.iter().rev()`, sedangkan *hit-testing* klik kanvas menggunakan `sources.iter()`.
   - **Seleksi Baris Full-Width (`Scenes` & `Source`):**
     - Setiap item di panel `Scenes` dan `Source` wajib berupa kartu baris penuh (*full-width row card*) sehingga pengguna dapat memilih item dengan mengklik di mana saja pada baris tersebut (tidak harus tepat di atas teks).
     - Kartu `Scenes` wajib menampilkan indikator visual `👁 PREVIEW` (Teal) dan `● LIVE` (Merah).
     - Panel `Scenes` menyediakan alih mode tampilan List (`icons::LIST`) dan Grid (`icons::LAYOUT_GRID`) lengkap dengan rendering thumbnail 16:9 real-time isi source masing-masing scene (`render_scene_thumbnail`).
     - Tipografi kartu baris Scenes: Menggunakan bobot font reguler yang nyaman dan bersih (tidak bold/tidak faux-bold), dengan jarak vertikal yang proporsional dan lega (`gap_y = 3.5_f32`) antara nama scene dan baris subtitle layer, tepat sejajar secara matematis di samping ikon.
   - **Konsistensi Form Modal, Auto-Sizing & Padding Tombol:**
     - **Auto-Sizing Tinggi Jendela Modal (`.auto_sized()`):** Seluruh jendela modal (`Virtual Camera`, `Properties`, `Settings`, `Add Scene`, `Rename`, `Filters`, `Save Layout Preset`, `Add Source`, dll.) **WAJIB** menyertakan `.auto_sized()` pada builder `egui::Window::new(...)`. Ukuran tinggi jendela modal wajib menyesuaikan secara otomatis dengan isi kontennya (*shrink-to-fit*), tidak boleh menyisakan ruang kosong vertikal yang terlalu panjang (*empty gap*) ataupun sempit/terpotong. Untuk modal form kompak seperti `Virtual Camera` (`.max_height(185.0)`), pasang batasan `.max_height(...)` agar pas membungkus isi form tanpa ruang kosong di bawah.
     - **Kerapian & Proporsi Lebar Modal:** Kunci lebar jendela modal dan seragamkan lebar widget (misalnya dialog `Virtual Camera` menggunakan lebar window `300px` dengan field `VCAM_FIELD_W = 280px`, form `Properties` menggunakan `PROP_FIELD_W = 280.0`, `PROP_FIELD_H = 24.0`, `SETTINGS_FIELD_W = 270.0` dengan `.truncate()`) agar form tampil proporsional, rapi, dan rata kanan-kiri tanpa memanjang sendiri ke kanan.
     - Standarisasi Text Input: Semua kotak input teks (seperti `Source Name:`, `Teks Badge (Label):`, `Teks Waktu Habis:`, dll.) wajib menggunakan helper standar `standard_singleline_text` dengan dimensi `Vec2::new(PROP_FIELD_W, PROP_FIELD_H)`, teks terpusat vertikal (`vertical_align(egui::Align::Center)`), margin seimbang (`egui::Margin::symmetric(6, 2)`), dan checkbox diletakkan pada kolom label agar lebar kotak input selalu seragam 100%.
     - Berikan ukuran minimum vertikal (`min_size` tinggi `20px–26px` atau `paint_icon_action_btn`) pada tombol agar ikon dan teks tidak pernah mepet dengan garis batas tombol.
     - **Tanpa Ekspansi Outline Hover/Active (`expansion = 0.0`):** Seluruh state interaksi widget (`noninteractive`, `inactive`, `hovered`, `active`, `open`) wajib memiliki `.expansion = 0.0` dan `visuals.clip_rect_margin = 0.0` pada tema visual ([`src/ui/theme.rs`](file:///d:/Rust/Nexora%20Studio/src/ui/theme.rs)). Hal ini mencegah tombol melompat atau garis outline biru/aksen fokus mekar melebihi batas kontrol (*no outline boundary overflow*), sehingga outline selalu persis mengikuti batas asli tombol.
   - **Audio Media Hanya Aktif Saat Live (`Program`):**
     - Suara dari `MediaSource` hanya boleh diputar ketika scene sudah ditransisikan ke **Program (`LIVE`)**, bukan saat baru disiapkan di Preview.
   - **Drag Modal Window Hanya Lewat Title Bar:**
     - Semua jendela modal (`Settings`, `Properties`, `Filters`, `Manage Slide Show Images`, `Rename`, `Save Layout Preset`) wajib memanggil `block_window_body_drag(ui)` di awal isi modal. Window hanya boleh digeser/dipindahkan melalui **Title Bar** saja untuk menjaga konsistensi dan mencegah pergeseran window yang tidak disengaja saat berinteraksi dengan widget/form.
   - **Standar Ikon Kursor & Drag-Drop:**
     - Aplikasi selalu terbuka otomatis dalam kondisi jendela maksimal (*maximized / full screen*) saat pertama kali dibuka (.with_maximized(true)).
     - Kursor standar saat hovering/klik biasa adalah panah default (`CursorIcon::Default`).
     - Kursor arah resize langsung aktif secara instan (zero-latency) dengan prioritas 4 sudut diagonal (margin 12px) dan 4 sisi horizontal/vertikal (margin 7px) tanpa jeda/telat.
     - Perhitungan drag resize sudut diagonal kanvas menggunakan proyeksi vektor diagonal layar sesungguhnya (delta_px.dot(diag) / diag_sq), menjamin handle resize menempel 1:1 di ujung kursor mouse tanpa drift atau ketidakseimbangan sensitivitas pada layer non-persegi (seperti TimeSource).
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
       - Tipografi & Skalabilitas: Font type (`Monospace`, `Proportional`, `Serif`), gaya tebal/bold toggle (`Tebal (Bold)`), font size slider (`16px..140px`), opsi badge/label (tampilkan/sembunyikan pill label, slider ukuran font badge 8px..80px, pilihan posisi anchor Kiri/Tengah/Kanan Atas/Bawah, serta slider pergeseran bebas X dan Y), kontrol ketebalan border/aksen (`0.0..12.0 px` / tanpa border), opsi format (tampilkan jam, titik dua solid `: ` tanpa kedip, format 24 jam), dan penyesuaian warna teks/background/aksen penuh.
      - **Kamera / Video Capture Device (`WebcamDevice`):**
        - Pengaturan Sudut Membulat (*Rounded Corner Radius*): Slider `0 px .. 160 px` lengkap dengan preset instan Persegi (`0`), Rounded Halus (`16`), Rounded Sedang (`32`), dan Bulat Penuh Lingkaran (`Circle / Avatar`). Rendering menggunakan GPU mesh triangulation yang halus dan presisi.
        - Pengaturan Garis Tepi (*Outline / Border*): Slider ketebalan `0.0 .. 20.0 px` (dengan preset `Off`, `2px`, `4px`, `6px`), pemilih warna RGB/Hex, serta palette chip warna populer siaran (Putih, Cyan Neon, Ungu Kamera, Merah Siaran, Hijau, Emas).
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
   - **Splitter Pemisah Canvas & Dock Bawah:**
     - Garis pemisah antara kanvas viewport (Preview & Program) dan panel dock bawah (Scenes, Source, Mixer, Controls) memiliki deteksi hover dan drag aktif dengan kursor panah vertikal ganda (CursorIcon::ResizeVertical) serta garis highlight aksen real-time yang jelas saat digeser. Jika modal dialog sedang terbuka (`app.is_any_modal_open()`), splitter dinonaktifkan total (tidak merebut kursor atau hover) dan rendering indikator menggunakan `LayerId::background()` agar tidak pernah tembus menimpa jendela modal.
   - **Fitur Salin & Tempel Layer Antar Scene (Copy & Paste Source Across Scenes):**
     - Pengguna dapat menyalin (*Copy*) layer apapun melalui klik kanan di panel Sources atau Preview canvas (Ctrl+C), berpindah ke scene lain, dan menempelkannya (*Paste*) menggunakan menu klik kanan atau pintasan keyboard (Ctrl+V).
     - Saat di-paste ke scene baru, seluruh konfigurasi layer (transformasi, crop, rounded corner, outline, chroma filter, media settings, dll.) disalin utuh dengan Uuid baru yang unik.
     - Jika nama layer belum ada di scene tujuan, nama aslinya dipertahankan. Jika sudah ada nama yang sama, otomatis ditambahkan imbuhan (Copy) atau (Copy N) beserta pergeseran offset kecil (+0.02) agar tidak bertumpuk 100%. Layer baru disisipkan pada index 0 (layer foreground paling depan).
   - **Fitur Virtual Camera & Desain Tombol Controls (OBS-Style):**
     - **Standarisasi Dimensi Tombol Controls:** Tombol **Start Recording** dan **Start / Stop Virtual Camera** di panel Controls wajib memiliki ukuran yang **PERSIS SAMA** (`width = body_size.x`, `height = 26.0 px`), tanpa ikon/bullet mentah di dalam teks tombol (hanya teks bersih terpusat), dengan jarak vertikal yang rapat dan proporsional (`item_spacing.y = 2.5`, `gap = 1.5 px`).
     - **Akses Pengaturan Virtual Camera:** Dialog modal pengaturan Virtual Camera dapat dibuka melalui klik kanan pada tombol **Start Virtual Camera** (`context_menu`) atau tombol `VCam` di footer panel Controls.
     - **Status Default Saat Dibuka:** Wajib selalu dalam kondisi nonaktif / **OFF** (`virtual_camera_active = false`) setiap kali aplikasi NEXORA Studio dibuka.
     - **Integrasi Driver Windows & Shared Memory (`virtual_camera`):**
       - Mendaftarkan entri driver kamera DirectShow di bawah kategori `CLSID_VideoInputDeviceCategory` (`HKCU\Software\Classes\CLSID\{860BB310-5D01-11d0-BD3B-00A0C911CE86}\Instance\{2B462C23-1F84-4829-9F8E-0062C1FE0E1A}`) dengan `FriendlyName: "NEXORA Virtual Camera"` (tanpa membutuhkan hak Administrator).
       - Menggunakan Shared Memory IPC (`OBSVirtualCamVideo`) berbasis ring buffer triple-buffered (NV12 format 1080p @ 30fps) sehingga langsung dapat dikenali dan diputar oleh Zoom, Microsoft Teams, Google Meet, Discord, dan WebRTC.
       - **Sumber Frame = Pixel Kanvas Asli:** Saat VCam aktif, `NexoraApp::update` mengirim `ViewportCommand::Screenshot` (~30 fps); frame berikutnya `forward_virtual_camera_screenshot` mengambil `Event::Screenshot`, meng-crop area `program_canvas_rect` / `preview_canvas_rect` (dikali `pixels_per_point`), lalu `VirtualCameraService::push_canvas_screenshot` mengirimnya ke thread VCam yang melakukan scale ke 1920x1080 → NV12. Jangan kembali ke `write_placeholder` kecuali belum ada frame pertama.
     - **Dialog Modal Virtual Camera:** Menampilkan pilihan *Output Type* (`Program (Default)`, `Preview`, `Scene`) serta *Output Selection* (hanya aktif saat memilih `Scene`).
     - **Indikator Siaran:** Menampilkan badge aktif pada status bar bawah (`VCam: LIVE (...)`) dan highlight visual tombol aktif di panel Controls saat Virtual Camera berjalan.
   - **Larangan Mutlak:** Dilarang menambahkan fitur *live streaming* / RTMP / cloud / login.
