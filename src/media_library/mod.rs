use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingEntry {
    pub file_name: String,
    pub full_path: PathBuf,
    pub recorded_at: DateTime<Local>,
    pub size_bytes: u64,
    pub duration_label: String,
}

impl RecordingEntry {
    pub fn formatted_size(&self) -> String {
        let mb = self.size_bytes as f64 / (1024.0 * 1024.0);
        if mb >= 1024.0 {
            format!("{:.2} GB", mb / 1024.0)
        } else if mb >= 1.0 {
            format!("{:.1} MB", mb)
        } else {
            format!("{} KB", (self.size_bytes / 1024).max(1))
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MediaLibrary {
    pub entries: Vec<RecordingEntry>,
}

impl MediaLibrary {
    pub fn scan_directory(&mut self, dir: &Path) {
        self.entries.clear();
        let Ok(read_dir) = fs::read_dir(dir) else {
            return;
        };
        for item in read_dir.flatten() {
            let path = item.path();
            if !path.is_file() {
                continue;
            }
            let is_video = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "mp4" | "mkv" | "mov"))
                .unwrap_or(false);
            if !is_video {
                continue;
            }
            if let Ok(meta) = item.metadata() {
                let recorded_at: DateTime<Local> = meta
                    .modified()
                    .map(DateTime::<Local>::from)
                    .unwrap_or_else(|_| Local::now());
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("recording.mp4")
                    .to_string();
                self.entries.push(RecordingEntry {
                    file_name,
                    full_path: path,
                    recorded_at,
                    size_bytes: meta.len(),
                    duration_label: "--:--".to_string(),
                });
            }
        }
        self.entries
            .sort_by(|a, b| b.recorded_at.cmp(&a.recorded_at));
    }
}

