use chrono::Local;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingStatus {
    Idle,
    Recording,
    Paused,
}

impl RecordingStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Idle => "READY",
            Self::Recording => "REC",
            Self::Paused => "PAUSED",
        }
    }
}

#[derive(Debug)]
pub struct RecordingController {
    pub status: RecordingStatus,
    started_at: Option<Instant>,
    accumulated: Duration,
    pub active_output_path: Option<PathBuf>,
    pub last_message: String,
    pub encoded_frames: u64,
    pub dropped_frames: u64,
}

impl Default for RecordingController {
    fn default() -> Self {
        Self {
            status: RecordingStatus::Idle,
            started_at: None,
            accumulated: Duration::ZERO,
            active_output_path: None,
            last_message: "Idle — Ready to start local session".to_string(),
            encoded_frames: 0,
            dropped_frames: 0,
        }
    }
}

impl RecordingController {
    pub fn build_timestamped_filename(ext: &str) -> String {
        let now = Local::now();
        format!("NEXORA_{}.{}", now.format("%Y-%m-%d_%H-%M-%S"), ext)
    }

    pub fn start_session(&mut self, output_dir: &Path, ext: &str) -> anyhow::Result<PathBuf> {
        let filename = Self::build_timestamped_filename(ext);
        let full_path = output_dir.join(filename);
        self.status = RecordingStatus::Recording;
        self.started_at = Some(Instant::now());
        self.accumulated = Duration::ZERO;
        self.encoded_frames = 0;
        self.dropped_frames = 0;
        self.active_output_path = Some(full_path.clone());
        self.last_message = format!("Session active -> {}", full_path.display());
        Ok(full_path)
    }

    pub fn toggle_pause(&mut self) {
        match self.status {
            RecordingStatus::Recording => {
                if let Some(start) = self.started_at.take() {
                    self.accumulated += start.elapsed();
                }
                self.status = RecordingStatus::Paused;
                self.last_message = "Recording paused".to_string();
            }
            RecordingStatus::Paused => {
                self.started_at = Some(Instant::now());
                self.status = RecordingStatus::Recording;
                self.last_message = "Recording resumed".to_string();
            }
            RecordingStatus::Idle => {}
        }
    }

    pub fn stop_session(&mut self) -> Option<(PathBuf, Duration)> {
        if self.status == RecordingStatus::Idle {
            return None;
        }
        let elapsed = self.elapsed();
        self.status = RecordingStatus::Idle;
        self.started_at = None;
        self.accumulated = Duration::ZERO;
        let path = self.active_output_path.take()?;
        self.last_message = format!(
            "Stopped ({:02}:{:02}) [Stage 3 MP4 encoder pending]",
            elapsed.as_secs() / 60,
            elapsed.as_secs() % 60
        );
        Some((path, elapsed))
    }

    pub fn elapsed(&self) -> Duration {
        match (self.status, self.started_at) {
            (RecordingStatus::Recording, Some(start)) => self.accumulated + start.elapsed(),
            _ => self.accumulated,
        }
    }

    pub fn formatted_timer(&self) -> String {
        let total_secs = self.elapsed().as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }
}

