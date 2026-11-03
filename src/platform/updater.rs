use anyhow::Context;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

pub const GITHUB_REPO: &str = "nexora-ctrl/nexora-studio";
pub const CURRENT_VERSION: &str = "0.9.0";

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

/// Parses semver string like "v0.5.0", "0.5.0", "0.5" into (major, minor, patch)
pub fn parse_version(s: &str) -> (u32, u32, u32) {
    let s = s.trim().trim_start_matches('v').trim_start_matches('V');
    let parts: Vec<u32> = s.split('.').filter_map(|p| p.parse().ok()).collect();
    let major = parts.first().copied().unwrap_or(0);
    let minor = parts.get(1).copied().unwrap_or(0);
    let patch = parts.get(2).copied().unwrap_or(0);
    (major, minor, patch)
}

/// Locks the shared updater state without risking a whole-app crash.
///
/// `Mutex::lock()` returns `Err` if the previous holder panicked while holding it, which poisons
/// the mutex forever after. Calling `.unwrap()` there would turn a contained background failure
/// into a panic in the UI thread — and the updater holds no invariants that a partial state could
/// corrupt, so recovering the inner value is strictly better than taking down the studio.
fn lock_state(state: &Mutex<UpdaterState>) -> std::sync::MutexGuard<'_, UpdaterState> {
    state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Checks for updates asynchronously without blocking the UI thread.
pub fn check_for_updates_async(state: Arc<Mutex<UpdaterState>>, ctx: egui::Context) {
    {
        let mut st = lock_state(&state);
        st.status = UpdateStatus::Checking;
    }
    ctx.request_repaint();

    thread::spawn(move || {
        let current_ver = {
            let st = lock_state(&state);
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
                                    body: Some("Update detected directly from GitHub repository main branch.".to_string()),
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

        let mut st = lock_state(&state);
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
                        release_notes: rel.body.unwrap_or_else(|| "Performance and stability updates.".to_string()),
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
                    error: "Unable to reach GitHub server. Please check your internet connection or try again later.".to_string(),
                };
            }
        }
        ctx.request_repaint();
    });
}

/// Downloads the new update binary asynchronously with real-time byte & progress tracking
pub fn start_download_async(
    state: Arc<Mutex<UpdaterState>>,
    download_url: String,
    latest_version: String,
    expected_size_bytes: Option<u64>,
    ctx: egui::Context,
) {
    let total_bytes_f32 = expected_size_bytes.unwrap_or(33_993_216) as f32; // Default ~32.4 MiB
    let total_mb = total_bytes_f32 / (1024.0 * 1024.0);

    {
        let mut st = lock_state(&state);
        st.status = UpdateStatus::Downloading {
            latest_version: latest_version.clone(),
            progress: 0.01,
            downloaded_mb: 0.0,
            total_mb,
        };
    }
    ctx.request_repaint();

    thread::spawn(move || {
        let temp_dir = std::env::temp_dir();
        let target_path = temp_dir.join(format!("nexora-studio-update-{}.exe", latest_version));

        // Clean up any stale partial download
        if target_path.exists() {
            let _ = std::fs::remove_file(&target_path);
        }

        let mut cmd = Command::new("curl.exe");
        cmd.args([
            "-L",
            "--retry", "3",
            "--retry-delay", "1",
            "-o",
            &target_path.to_string_lossy(),
            &download_url,
        ]);

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let mut st = lock_state(&state);
                st.status = UpdateStatus::Failed {
                    error: format!("Failed to launch downloader: {}", e),
                };
                ctx.request_repaint();
                return;
            }
        };

        // Poll downloaded file size on disk periodically to provide smooth real-time progress updates
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let mut st = lock_state(&state);
                    if status.success() && target_path.exists() {
                        if let Ok(meta) = std::fs::metadata(&target_path) {
                            if meta.len() > 1_000_000 {
                                st.status = UpdateStatus::ReadyToRestart {
                                    new_version: latest_version,
                                    downloaded_path: target_path,
                                };
                            } else {
                                st.status = UpdateStatus::Failed {
                                    error: "Downloaded binary file size is invalid.".to_string(),
                                };
                            }
                        } else {
                            st.status = UpdateStatus::Failed {
                                error: "Failed to read downloaded update file.".to_string(),
                            };
                        }
                    } else {
                        st.status = UpdateStatus::Failed {
                            error: format!("Download incomplete or interrupted (exit code: {})", status),
                        };
                    }
                    ctx.request_repaint();
                    break;
                }
                Ok(None) => {
                    // Downloader still active: inspect downloaded file size
                    if let Ok(meta) = std::fs::metadata(&target_path) {
                        let cur_bytes = meta.len() as f32;
                        let progress = (cur_bytes / total_bytes_f32).clamp(0.01, 0.98);
                        let downloaded_mb = cur_bytes / (1024.0 * 1024.0);

                        let mut st = lock_state(&state);
                        if let UpdateStatus::Downloading {
                            progress: ref mut p,
                            downloaded_mb: ref mut d,
                            ..
                        } = st.status
                        {
                            *p = progress;
                            *d = downloaded_mb;
                        }
                        ctx.request_repaint();
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
                Err(e) => {
                    let mut st = lock_state(&state);
                    st.status = UpdateStatus::Failed {
                        error: format!("Error monitoring downloader: {}", e),
                    };
                    ctx.request_repaint();
                    break;
                }
            }
        }
    });
}

/// Safely replaces the running executable on Windows using atomic rename,
/// launches the newly updated executable, and exits the current process.
pub fn apply_update_and_restart(new_binary_path: &Path) -> anyhow::Result<()> {
    let current_exe = std::env::current_exe().context("Failed to detect current executable path")?;
    let old_exe = current_exe.with_extension("exe.old");

    // 1. Remove old backup file if it exists
    if old_exe.exists() {
        let _ = std::fs::remove_file(&old_exe);
    }

    // 2. Windows allows renaming a currently running executable!
    std::fs::rename(&current_exe, &old_exe).context("Failed to rename active executable")?;

    // 3. Move the downloaded new executable into place
    std::fs::copy(new_binary_path, &current_exe)
        .context("Failed to install new executable")?;
    let _ = std::fs::remove_file(new_binary_path);

    // 4. Launch the updated application
    Command::new(&current_exe)
        .spawn()
        .context("Failed to restart application with new version")?;

    // 5. Spawn detached background delayed cleanup to guarantee old_exe is deleted once process exits
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let old_str = old_exe.to_string_lossy().to_string();
        let _ = Command::new("cmd.exe")
            .args(["/C", "timeout /t 2 /nobreak > NUL & del /f /q", &format!("\"{}\"", old_str)])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }

    // 6. Exit the current process cleanly
    std::process::exit(0);
}

/// Cleans up any leftover .exe.old from previous updates during application startup
pub fn cleanup_old_executable_if_needed() {
    if let Ok(current_exe) = std::env::current_exe() {
        let old_exe = current_exe.with_extension("exe.old");
        let alt_old_exe = current_exe.with_extension("old");
        let parent_dir = current_exe.parent().map(|p| p.to_path_buf());

        // Spawn a background thread to retry cleanup after previous process has fully exited
        std::thread::spawn(move || {
            // Immediate check
            if old_exe.exists() {
                let _ = std::fs::remove_file(&old_exe);
            }
            if alt_old_exe.exists() {
                let _ = std::fs::remove_file(&alt_old_exe);
            }

            // Retry for up to 3 seconds to ensure previous Windows file locks are released
            for _ in 0..6 {
                std::thread::sleep(std::time::Duration::from_millis(500));
                let mut found_any = false;

                if old_exe.exists() {
                    found_any = true;
                    let _ = std::fs::remove_file(&old_exe);
                }
                if alt_old_exe.exists() {
                    found_any = true;
                    let _ = std::fs::remove_file(&alt_old_exe);
                }

                // Also scan directory for any lingering .old files from previous updates
                if let Some(ref dir) = parent_dir {
                    if let Ok(entries) = std::fs::read_dir(dir) {
                        for entry in entries.flatten() {
                            let p = entry.path();
                            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                                if name.to_lowercase().ends_with(".exe.old")
                                    || (name.to_lowercase().contains("nexora") && name.to_lowercase().ends_with(".old"))
                                {
                                    found_any = true;
                                    let _ = std::fs::remove_file(&p);
                                }
                            }
                        }
                    }
                }

                if !found_any {
                    break;
                }
            }
        });
    }
}

