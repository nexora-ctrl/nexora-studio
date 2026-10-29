use crate::audio::AudioMixerState;
use crate::studio::StudioState;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl Default for ThemeMode {
    fn default() -> Self {
        Self::Dark
    }
}

impl ThemeMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }

    pub fn toggle(&mut self) {
        *self = match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ScenesViewMode {
    #[default]
    List,
    Grid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoPreset {
    FullHd60,
    FullHd30,
    Hd60,
    Hd30,
    Native60,
}

impl VideoPreset {
    pub fn label(&self) -> &'static str {
        match self {
            Self::FullHd60 => "1920x1080 @ 60 FPS",
            Self::FullHd30 => "1920x1080 @ 30 FPS",
            Self::Hd60 => "1280x720 @ 60 FPS",
            Self::Hd30 => "1280x720 @ 30 FPS",
            Self::Native60 => "Native Display @ 60 FPS",
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::FullHd60 | Self::FullHd30 | Self::Native60 => (1920, 1080),
            Self::Hd60 | Self::Hd30 => (1280, 720),
        }
    }

    pub fn fps(&self) -> u32 {
        match self {
            Self::FullHd60 | Self::Hd60 | Self::Native60 => 60,
            Self::FullHd30 | Self::Hd30 => 30,
        }
    }

    pub fn all() -> &'static [VideoPreset] {
        &[
            Self::FullHd60,
            Self::FullHd30,
            Self::Hd60,
            Self::Hd30,
            Self::Native60,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BitratePreset {
    Standard8Mbps,
    High12Mbps,
    Ultra20Mbps,
    Compact5Mbps,
}

impl BitratePreset {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Compact5Mbps => "5 Mbps (Compact)",
            Self::Standard8Mbps => "8 Mbps (Standard)",
            Self::High12Mbps => "12 Mbps (High Quality)",
            Self::Ultra20Mbps => "20 Mbps (Ultra Studio)",
        }
    }

    pub fn kbps(&self) -> u32 {
        match self {
            Self::Compact5Mbps => 5_000,
            Self::Standard8Mbps => 8_000,
            Self::High12Mbps => 12_000,
            Self::Ultra20Mbps => 20_000,
        }
    }

    pub fn all() -> &'static [BitratePreset] {
        &[
            Self::Compact5Mbps,
            Self::Standard8Mbps,
            Self::High12Mbps,
            Self::Ultra20Mbps,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelVisibility {
    pub show_scenes: bool,
    pub show_sources: bool,
    pub show_audio_mixer: bool,
    pub show_recording_controls: bool,
    pub show_library_drawer: bool,
    pub studio_mode: bool,
}

impl Default for PanelVisibility {
    fn default() -> Self {
        Self {
            show_scenes: true,
            show_sources: true,
            show_audio_mixer: true,
            show_recording_controls: true,
            show_library_drawer: false,
            studio_mode: true, // Default workspace: Studio Mode with dual preview
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum MediaDecodeScale {
    #[default]
    Performance540p,
    Balanced720p,
    Native1080p,
}

impl MediaDecodeScale {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Performance540p => "960×540 (Ultra Smooth — Low GPU/CPU)",
            Self::Balanced720p => "1280×720 (HD Balanced)",
            Self::Native1080p => "1920×1080 (Full Native Resolution)",
        }
    }

    pub fn max_dimensions(&self) -> (usize, usize) {
        match self {
            Self::Performance540p => (960, 540),
            Self::Balanced720p => (1280, 720),
            Self::Native1080p => (1920, 1080),
        }
    }

    pub fn all() -> &'static [MediaDecodeScale] {
        &[
            Self::Performance540p,
            Self::Balanced720p,
            Self::Native1080p,
        ]
    }
}

/// A saved Screen & Layout configuration profile including scenes, sources,
/// screen/camera transforms, audio mixer devices & levels, and UI dock layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenLayoutPreset {
    pub name: String,
    pub updated_at: String,
    pub panels: PanelVisibility,
    pub dock_height: f32,
    pub preview_fit_to_window: bool,
    pub preview_zoom: f32,
    pub show_safe_margins: bool,
    pub video_preset: VideoPreset,
    pub bitrate_preset: BitratePreset,
    pub studio: StudioState,
    pub audio_mixer: AudioMixerState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub theme: ThemeMode,
    pub output_directory: PathBuf,
    pub video_preset: VideoPreset,
    pub bitrate_preset: BitratePreset,
    pub container_format: String,
    pub panels: PanelVisibility,
    pub preview_fit_to_window: bool,
    pub preview_zoom: f32,
    pub show_safe_margins: bool,
    pub window_size: (f32, f32),
    pub window_pos: Option<(f32, f32)>,
    pub window_maximized: bool,
    pub dock_height: f32,
    pub library_drawer_width: f32,
    pub active_layout_name: String,
    pub saved_layout_presets: Vec<ScreenLayoutPreset>,
    pub preferred_gpu_index: Option<u32>,
    pub preferred_gpu_name: String,
    pub media_decode_scale: MediaDecodeScale,
    pub scenes_view_mode: ScenesViewMode,
}

impl Default for AppSettings {
    fn default() -> Self {
        let default_output = Self::default_recordings_dir();
        Self {
            theme: ThemeMode::Dark,
            output_directory: default_output,
            video_preset: VideoPreset::FullHd60,
            bitrate_preset: BitratePreset::High12Mbps,
            container_format: "mp4".to_string(),
            panels: PanelVisibility::default(),
            preview_fit_to_window: true,
            preview_zoom: 1.0,
            show_safe_margins: true,
            window_size: (1340.0, 760.0),
            window_pos: None,
            window_maximized: true,
            dock_height: 245.0,
            library_drawer_width: 275.0,
            active_layout_name: "Default Studio Layout".to_string(),
            saved_layout_presets: Vec::new(),
            preferred_gpu_index: Some(0),
            preferred_gpu_name: String::new(),
            media_decode_scale: MediaDecodeScale::Performance540p,
            scenes_view_mode: ScenesViewMode::List,
        }
    }
}

impl AppSettings {
    pub fn default_recordings_dir() -> PathBuf {
        if let Some(user_dirs) = directories::UserDirs::new() {
            if let Some(video_dir) = user_dirs.video_dir() {
                return video_dir.join("NEXORA Studio");
            }
            return user_dirs.home_dir().join("Videos").join("NEXORA Studio");
        }
        PathBuf::from("./Recordings")
    }

    pub fn config_file_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "Nexora", "NexoraStudio")
            .map(|dirs| dirs.config_dir().join("settings.json"))
    }

    pub fn load_or_default() -> Self {
        if let Some(path) = Self::config_file_path() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str::<Self>(&content) {
                    return settings;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(path) = Self::config_file_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(self)?;
            fs::write(path, json)?;
        }
        Ok(())
    }
}

