use crate::audio::AudioMixerState;
use crate::encoder::EncoderConfig;
use crate::settings::{BitratePreset, VideoPreset};
use crate::studio::StudioState;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexoraProject {
    pub schema_version: u32,
    pub project_name: String,
    pub updated_at: DateTime<Local>,
    pub video_preset: VideoPreset,
    pub bitrate_preset: BitratePreset,
    pub studio: StudioState,
    pub audio_mixer: AudioMixerState,
    #[serde(default)]
    pub encoder_config: EncoderConfig,
}

impl Default for NexoraProject {
    fn default() -> Self {
        Self {
            schema_version: 1,
            project_name: "Untitled Studio Project".to_string(),
            updated_at: Local::now(),
            video_preset: VideoPreset::FullHd60,
            bitrate_preset: BitratePreset::High12Mbps,
            studio: StudioState::default(),
            audio_mixer: AudioMixerState::default(),
            encoder_config: EncoderConfig::default(),
        }
    }
}

impl NexoraProject {
    pub fn save_to_file(&mut self, path: &Path) -> anyhow::Result<()> {
        self.updated_at = Local::now();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &Path) -> anyhow::Result<Self> {
        let raw = fs::read_to_string(path)?;
        let project: Self = serde_json::from_str(&raw)?;
        if project.studio.scenes.is_empty() {
            anyhow::bail!("Invalid project file: project contains zero scenes.");
        }
        Ok(project)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProjectManager {
    pub current_project_path: Option<PathBuf>,
    pub project_name: String,
    pub is_dirty: bool,
    pub recent_projects: Vec<PathBuf>,
}

impl ProjectManager {
    pub fn new() -> Self {
        Self {
            current_project_path: None,
            project_name: "Default Studio Session".to_string(),
            is_dirty: false,
            recent_projects: Vec::new(),
        }
    }
}

