# RELEASE_WORKFLOW.md — Panduan Otomatis Rilis & Pembaruan Versi NEXORA Studio

Dokumen ini adalah instruksi operasional standar (**SOP**) untuk Agen AI Antigravity / Pengembang setiap kali Pengguna meminta rilis atau pembaruan versi baru (*version bump & release*).

Ketika Pengguna mengatakan:
> *"Lakukan update versi baru"* atau *"Jalankan rilis sesuai RELEASE_WORKFLOW.md"*

Agen AI wajib mengeksekusi langkah-langkah di bawah ini secara berurutan dan tuntas.

---

## 📌 Aturan Identitas & Git Wajib
- **Akun Git Resmi:**
  - `user.name`: `nexora-ctrl`
  - `user.email`: `nexora.suport.id@gmail.com`
- **Repositori Resmi:** `https://github.com/nexora-ctrl/nexora-studio.git`
- **Target Dir Kompilasi:** `D:\Rust\.nexora-target`
- **Binary Output:** `D:\Rust\.nexora-target\release\nexora-studio.exe`

---

## 📋 Langkah Demi Langkah Eksekusi (Step-by-Step SOP)

### Langkah 1: Tentukan Nomor Versi Baru (SemVer)
Tentukan versi berikutnya berdasarkan jenis perubahan:
- **Patch (Perbaikan bug / tweak UI kecil):** Contoh `0.6.0` ➔ `0.6.1`
- **Minor (Fitur baru / rombakan modul):** Contoh `0.6.0` ➔ `0.7.0`
- **Major (Rombakan besar arsitektur):** Contoh `0.6.0` ➔ `1.0.0`

---

### Langkah 2: Perbarui Nomor Versi di 6 Berkas Kode
Ganti versi lama dengan versi baru (`X.Y.Z` dan `vX.Y`) pada berkas berikut:

1. **`Cargo.toml`** (baris 3):
   ```toml
   version = "X.Y.Z"
   ```
2. **`src/platform/updater.rs`** (baris 8):
   ```rust
   pub const CURRENT_VERSION: &str = "X.Y.Z";
   ```
3. **`src/ui/top_bar.rs`** (label versi Top Bar):
   ```rust
   RichText::new("vX.Y")
   ```
4. **`src/ui/dialogs.rs`** (label di modal Settings / About):
   ```rust
   ui.label(RichText::new("Version vX.Y.Z (Native Windows 64-bit)").small().color(p.text_secondary));
   ```
5. **`src/main.rs`** (unit test updater):
   ```rust
   assert_eq!(updater.current_version, "X.Y.Z");
   ```
6. **`README.md`** (badge versi):
   ```markdown
   [![Release](https://img.shields.io/badge/version-vX.Y.Z-success.svg?style=flat-square)](https://github.com/nexora-ctrl/nexora-studio/releases)
   ```

---

### Langkah 3: Eksekusi Test & Kompilasi Binary (PowerShell)
Hentikan proses yang berjalan untuk menghindari *file lock*, jalankan pengujian unit, lalu bangun binary release & debug:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
Get-Process nexora-studio -ErrorAction SilentlyContinue | Stop-Process -Force

# 1. Jalankan unit test
cargo test

# 2. Bangun binary Release teroptimasi
cargo build --release

# 3. Bangun binary Debug
cargo build
```

> [!IMPORTANT]
> Pastikan seluruh test lulus (`13 passed; 0 failed`) dan file `D:\Rust\.nexora-target\release\nexora-studio.exe` berhasil terbuat.

---

### Langkah 4: Commit & Buat Git Tag Baru
Simpan perubahan versi ke git dan buat annotated tag:

```powershell
git add Cargo.lock Cargo.toml README.md src/main.rs src/platform/updater.rs src/ui/dialogs.rs src/ui/top_bar.rs
git commit -m "chore(release): bump version to vX.Y.Z and update UI indicators"
git tag -a vX.Y.Z -m "Release vX.Y.Z: <Ringkasan Perubahan>"
git push origin main
git push origin vX.Y.Z
```

---

### Langkah 5: Buat GitHub Release & Upload Binary Otomatis
Gunakan script PowerShell berikut untuk mengambil token otentikasi Git dan mengunggah binary executable ke GitHub Releases:

```powershell
# 1. Ambil token otentikasi dari Windows Git Credential Manager
$credInfo = echo "protocol=https`nhost=github.com" | git credential fill
$token = ($credInfo | Select-String "password=").ToString().Trim().Replace("password=", "")

$headers = @{
    "Authorization" = "Bearer $token"
    "Accept"        = "application/vnd.github+json"
    "User-Agent"    = "NEXORA-Studio-Release-Bot"
}

# 2. Buat Release di GitHub
$releaseBody = @"
## What's New in vX.Y.Z

- Deskripsi fitur baru atau perbaikan di sini...

### Downloads
Download ``nexora-studio.exe`` di bawah ini (Portable Windows 64-bit binary).
"@

$bodyObj = @{
    tag_name   = "vX.Y.Z"
    name       = "NEXORA Studio vX.Y.Z"
    body       = $releaseBody
    draft      = $false
    prerelease = $false
} | ConvertTo-Json

$createRes = Invoke-RestMethod -Uri "https://api.github.com/repos/nexora-ctrl/nexora-studio/releases" -Method Post -Headers $headers -Body $bodyObj -ContentType "application/json"
$releaseId = $createRes.id

# 3. Upload nexora-studio.exe sebagai Release Asset
$uploadUri = "https://uploads.github.com/repos/nexora-ctrl/nexora-studio/releases/$releaseId/assets?name=nexora-studio.exe"
$uploadRes = curl.exe -s -X POST -H "Authorization: Bearer $token" -H "Content-Type: application/octet-stream" --data-binary "@D:\Rust\.nexora-target\release\nexora-studio.exe" $uploadUri | ConvertFrom-Json

Write-Host "✅ Release vX.Y.Z berhasil dipublikasikan!"
Write-Host "🔗 URL Download: $($uploadRes.browser_download_url)"
```

---

### Langkah 6: Verifikasi Hasil Rilis
Pastikan API GitHub mengembalikan data rilis terbaru dengan aset `.exe` yang terpasang:

```powershell
curl.exe -s "https://api.github.com/repos/nexora-ctrl/nexora-studio/releases/latest" | ConvertFrom-Json | Select-Object tag_name, name, html_url, @{n="Assets";e={$_.assets.name}}
```

Pastikan output menampilkan:
- `tag_name`: `vX.Y.Z`
- `Assets`: `nexora-studio.exe`

---

## 💡 Dampak pada In-App Updater
Setelah langkah di atas selesai, pengguna yang masih menggunakan versi lama akan secara otomatis:
1. Mendeteksi versi baru via background scanner saat aplikasi dibuka.
2. Memunculkan badge hijau smooth **`Pembaruan tersedia: Update Sekarang`** di samping badge Studio Mode di Top Bar.
3. Mengklik badge untuk mengunduh `nexora-studio.exe` langsung dari GitHub Releases dan menghapus file `.old` secara otomatis.
