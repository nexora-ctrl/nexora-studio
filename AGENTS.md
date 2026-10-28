# AGENTS.md — Panduan Cepat untuk Agen AI di Workspace NEXORA Studio

Selamat datang di repositori **NEXORA Studio** (Aplikasi Desktop Screen Recording Native Windows berbasis Rust & `egui`).

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

3. **Konvensi UI/UX:**
   - Pertahankan **Studio Mode** (*Preview* di kiri, *Program* di kanan) dengan rasio kanvas `16:9`.
   - Label deskriptif scene & source hanya boleh tampil otomatis di bagian *header* atas kanvas/frame.
   - Panel bawah terdiri dari **`Scenes`**, **`Source`**, **`Audio Mixer`**, dan **`Controls`** dengan tata letak vertikal (`Layout::top_down`).
   - Dilarang menambahkan fitur *live streaming* / RTMP / cloud / login.

