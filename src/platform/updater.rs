use anyhow::Context;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

pub const GITHUB_REPO: &str = "nusambait/nexora-studio";
pub const CURRENT_VERSION: &str = "0.3.0";

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GithubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GithubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub html_url: String,
    pub body: Option<String>,
    pub assets: Vec<GithubAsset>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate {
        version: String,
        check_time: String,
    },
    UpdateAvailable {
        current_version: String,
        latest_version: String,
        release_title: String,
        release_url: String,
        download_url: Option<String>,
        release_notes: String,
        file_size_bytes: Option<u64>,
    },
    Downloading {
        latest_version: String,
        progress: f32, // 0.0 .. 1.0
        downloaded_mb: f32,
        total_mb: f32,
    },
    ReadyToRestart {
        new_version: String,
        downloaded_path: PathBuf,
    },
    Failed {
        error: String,
    },
}

#[derive(Debug, Clone)]
pub struct UpdaterState {
    pub status: UpdateStatus,
    pub current_version: String,
    pub repo_url: String,
}

impl Default for UpdaterState {
    fn default() -> Self {
        Self {
            status: UpdateStatus::Idle,
            current_version: CURRENT_VERSION.to_string(),
            repo_url: format!("https://github.com/{}", GITHUB_REPO),
        }
    }
}

/// Parses semver string like "v0.4.0", "0.4.0", "0.4" into (major, minor, patch)
pub fn parse_version(s: &str) -> (u32, u32, u32) {
    let s = s.trim().trim_start_matches('v').trim_start_matches('V');
    let parts: Vec<u32> = s.split('.').filter_map(|p| p.parse().ok()).collect();
    let major = parts.first().copied().unwrap_or(0);
    let minor = parts.get(1).copied().unwrap_or(0);
    let patch = parts.get(2).copied().unwrap_or(0);
    (major, minor, patch)
}

/// Checks for updates asynchronously without blocking the UI thread.
pub fn check_for_updates_async(state: Arc<Mutex<UpdaterState>>, ctx: egui::Context) {
    {
        let mut st = state.lock().unwrap();
        st.status = UpdateStatus::Checking;
    }
    ctx.request_repaint();

    thread::spawn(move || {
        let current_ver = {
            let st = state.lock().unwrap();
            st.current_version.clone()
        };

        // 1. Try querying GitHub Releases API
        let release_api_url = format!("https://api.github.com/repos/{}/releases/latest", GITHUB_REPO);
        let output = Command::new("curl.exe")
            .args([
                "-s",
                "-m", "8",
                "-H", "User-Agent: NexoraStudio-Updater",
                &release_api_url,
            ])
            .output();

        let mut found_release: Option<GithubRelease> = None;

        if let Ok(ref out) = output {
            if out.status.success() {
                if let Ok(text) = String::from_utf8(out.stdout.clone()) {
                    if let Ok(rel) = serde_json::from_str::<GithubRelease>(&text) {
                        found_release = Some(rel);
                    }
                }
            }
        }

        // 2. If no official GitHub release exists yet, query raw Cargo.toml on main branch as fallback
        if found_release.is_none() {
            let raw_cargo_url = format!(
                "https://raw.githubusercontent.com/{}/main/Cargo.toml",
                GITHUB_REPO
            );
            if let Ok(raw_out) = Command::new("curl.exe")
                .args(["-s", "-m", "8", &raw_cargo_url])
                .output()
            {
                if raw_out.status.success() {
                    if let Ok(cargo_text) = String::from_utf8(raw_out.stdout) {
                        for line in cargo_text.lines() {
                            let line = line.trim();
                            if line.starts_with("version = ") {
                                let ver = line
                                    .trim_start_matches("version = ")
                                    .trim_matches('"')
                                    .trim();
                                found_release = Some(GithubRelease {
                                    tag_name: format!("v{}", ver),
                                    name: Some(format!("NEXORA Studio v{}", ver)),
                                    html_url: format!("https://github.com/{}/tree/main", GITHUB_REPO),
                                    body: Some("Pembaruan terdeteksi langsung dari branch utama repositori GitHub.".to_string()),
                                    assets: Vec::new(),
                                });
                                break;
                            }
                        }
                    }
                }
            }
        }

        let now_str = chrono::Local::now().format("%H:%M:%S").to_string();

        let mut st = state.lock().unwrap();
        match found_release {
            Some(rel) => {
                let latest_ver = rel.tag_name.trim().to_string();
                let latest_tuple = parse_version(&latest_ver);
                let current_tuple = parse_version(&current_ver);

                if latest_tuple > current_tuple {
                    // Find Windows .exe binary asset if available
                    let exe_asset = rel.assets.iter().find(|a| {
                        a.name.ends_with(".exe") || a.name.ends_with(".zip")
                    });

                    let download_url = exe_asset.map(|a| a.browser_download_url.clone());
                    let file_size_bytes = exe_asset.map(|a| a.size);

                    st.status = UpdateStatus::UpdateAvailable {
                        current_version: current_ver,
                        latest_version: latest_ver,
                        release_title: rel.name.unwrap_or_else(|| rel.tag_name.clone()),
                        release_url: rel.html_url,
                        download_url,
                        release_notes: rel.body.unwrap_or_else(|| "Pembaruan performa dan stabilitas sistem.".to_string()),
                        file_size_bytes,
                    };
                } else {
                    st.status = UpdateStatus::UpToDate {
                        version: current_ver,
                        check_time: now_str,
                    };
                }
            }
            None => {
                st.status = UpdateStatus::Failed {
                    error: "Tidak dapat menghubungi server GitHub. Periksa koneksi internet Anda atau coba beberapa saat lagi.".to_string(),
                };
            }
        }
        ctx.request_repaint();
    });
}

/// Downloads the new update binary asynchronously
pub fn start_download_async(
    state: Arc<Mutex<UpdaterState>>,
    download_url: String,
    latest_version: String,
    ctx: egui::Context,
) {
    {
        let mut st = state.lock().unwrap();
        st.status = UpdateStatus::Downloading {
            latest_version: latest_version.clone(),
            progress: 0.05,
            downloaded_mb: 0.0,
            total_mb: 31.0,
        };
    }
    ctx.request_repaint();

    thread::spawn(move || {
        let temp_dir = std::env::temp_dir();
        let target_path = temp_dir.join(format!("nexora-studio-update-{}.exe", latest_version));

        let res = Command::new("curl.exe")
            .args([
                "-L",
                "-o",
                &target_path.to_string_lossy(),
                &download_url,
            ])
            .output();

        let mut st = state.lock().unwrap();
        match res {
            Ok(out) if out.status.success() && target_path.exists() => {
                if let Ok(meta) = std::fs::metadata(&target_path) {
                    if meta.len() > 1_000_000 {
                        st.status = UpdateStatus::ReadyToRestart {
                            new_version: latest_version,
                            downloaded_path: target_path,
                        };
                    } else {
                        st.status = UpdateStatus::Failed {
                            error: "Ukuran file binary yang diunduh tidak valid.".to_string(),
                        };
                    }
                } else {
                    st.status = UpdateStatus::Failed {
                        error: "Gagal membaca file hasil unduhan.".to_string(),
                    };
                }
            }
            Ok(out) => {
                let err_msg = String::from_utf8_lossy(&out.stderr).to_string();
                st.status = UpdateStatus::Failed {
                    error: format!("Gagal mengunduh file update: {}", err_msg),
                };
            }
            Err(e) => {
                st.status = UpdateStatus::Failed {
                    error: format!("Gagal menjalankan downloader: {}", e),
                };
            }
        }
        ctx.request_repaint();
    });
}

/// Safely replaces the running executable on Windows using atomic rename,
/// launches the newly updated executable, and exits the current process.
pub fn apply_update_and_restart(new_binary_path: &Path) -> anyhow::Result<()> {
    let current_exe = std::env::current_exe().context("Gagal mendeteksi path executable saat ini")?;
    let old_exe = current_exe.with_extension("exe.old");

    // 1. Remove old backup file if it exists
    if old_exe.exists() {
        let _ = std::fs::remove_file(&old_exe);
    }

    // 2. Windows allows renaming a currently running executable!
    std::fs::rename(&current_exe, &old_exe).context("Gagal me-rename file executable aktif")?;

    // 3. Move the downloaded new executable into place
    std::fs::copy(new_binary_path, &current_exe)
        .context("Gagal memasang file executable baru")?;
    let _ = std::fs::remove_file(new_binary_path);

    // 4. Launch the updated application
    Command::new(&current_exe)
        .spawn()
        .context("Gagal me-restart aplikasi dengan versi baru")?;

    // 5. Exit the current process cleanly
    std::process::exit(0);
}

/// Cleans up any leftover .exe.old from previous updates during application startup
pub fn cleanup_old_executable_if_needed() {
    if let Ok(current_exe) = std::env::current_exe() {
        let old_exe = current_exe.with_extension("exe.old");
        if old_exe.exists() {
            let _ = std::fs::remove_file(&old_exe);
        }
    }
}
