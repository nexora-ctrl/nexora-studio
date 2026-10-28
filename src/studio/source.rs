use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MediaColorRange {
    #[default]
    Auto,
    Partial,
    Full,
}

impl MediaColorRange {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Partial => "Limited (16-235)",
            Self::Full => "Full (0-255)",
        }
    }

    pub fn all() -> &'static [MediaColorRange] {
        &[Self::Auto, Self::Partial, Self::Full]
    }
}

fn default_true() -> bool {
    true
}

fn default_speed_100() -> u32 {
    100
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SourceKind {
    DisplayCapture {
        monitor_id: String,
        monitor_name: String,
        resolution: (u32, u32),
        capture_cursor: bool,
    },
    WindowCapture {
        window_hwnd: u64,
        window_title: String,
        executable_name: String,
        capture_cursor: bool,
    },
    GameCapture {
        mode: String,
        window_hwnd: u64,
        window_title: String,
        executable_name: String,
        capture_cursor: bool,
        anti_cheat_hook: bool,
    },
    WebcamDevice {
        device_id: String,
        device_name: String,
        resolution: (u32, u32),
    },
    MediaSource {
        #[serde(default = "default_true")]
        is_local_file: bool,
        file_path: String,
        #[serde(default = "default_true")]
        loop_playback: bool,
        #[serde(default = "default_true")]
        restart_on_activate: bool,
        #[serde(default = "default_true")]
        use_hardware_decoding: bool,
        #[serde(default)]
        hide_when_ended: bool,
        #[serde(default)]
        close_when_inactive: bool,
        #[serde(default = "default_speed_100")]
        speed_percent: u32,
        #[serde(default = "default_one")]
        volume: f32,
        #[serde(default)]
        muted: bool,
        #[serde(default = "default_true")]
        is_playing: bool,
        #[serde(default)]
        color_range: MediaColorRange,
        #[serde(default)]
        restart_nonce: u64,
    },
    ImageSource {
        file_path: String,
        opacity: f32,
    },
    ImageSlideShow {
        files: Vec<String>,
        slide_time_ms: u32,
        loop_slides: bool,
    },
    AudioInputCapture {
        device_name: String,
        volume: f32,
        muted: bool,
    },
    AudioOutputCapture {
        device_name: String,
        volume: f32,
        muted: bool,
    },
    ApplicationAudioCapture {
        window_title: String,
        executable_name: String,
        volume: f32,
        muted: bool,
    },
    ColorSource {
        color_rgba: [u8; 4],
        width: u32,
        height: u32,
    },
    TextGdiPlus {
        text: String,
        font_size: f32,
        color_rgba: [u8; 4],
    },
    BrowserSource {
        url: String,
        width: u32,
        height: u32,
        fps: u32,
    },
    SceneSource {
        scene_name: String,
    },
}

impl SourceKind {
    pub fn is_audio_only_media(file_path: &str) -> bool {
        let lower = file_path.trim().to_ascii_lowercase();
        lower.ends_with(".mp3")
            || lower.ends_with(".wav")
            || lower.ends_with(".flac")
            || lower.ends_with(".ogg")
            || lower.ends_with(".m4a")
            || lower.ends_with(".aac")
            || lower.ends_with(".wma")
    }

    pub fn tabler_icon(&self) -> &'static str {
        match self {
            Self::DisplayCapture { .. } => crate::ui::icons::DEVICE_DESKTOP,
            Self::WindowCapture { .. } => crate::ui::icons::APP_WINDOW,
            Self::GameCapture { .. } => crate::ui::icons::DEVICE_GAMEPAD,
            Self::WebcamDevice { .. } => crate::ui::icons::CAMERA,
            Self::MediaSource { file_path, .. } => {
                if Self::is_audio_only_media(file_path) {
                    crate::ui::icons::MUSIC
                } else {
                    crate::ui::icons::PLAYER_PLAY
                }
            }
            Self::ImageSource { .. } => crate::ui::icons::PHOTO,
            Self::ImageSlideShow { .. } => crate::ui::icons::PRESENTATION,
            Self::AudioInputCapture { .. } => crate::ui::icons::MICROPHONE,
            Self::AudioOutputCapture { .. } | Self::ApplicationAudioCapture { .. } => {
                crate::ui::icons::VOLUME
            }
            Self::ColorSource { .. } => crate::ui::icons::BRUSH,
            Self::TextGdiPlus { .. } => crate::ui::icons::LETTER_CASE,
            Self::BrowserSource { .. } => crate::ui::icons::WORLD,
            Self::SceneSource { .. } => crate::ui::icons::LIST,
        }
    }

    pub fn type_badge(&self) -> &'static str {
        match self {
            Self::DisplayCapture { .. } => "DISP",
            Self::WindowCapture { .. } => "WIN",
            Self::GameCapture { .. } => "GAME",
            Self::WebcamDevice { .. } => "CAM",
            Self::MediaSource { .. } => "MEDIA",
            Self::ImageSource { .. } => "IMG",
            Self::ImageSlideShow { .. } => "SLIDE",
            Self::AudioInputCapture { .. } => "MIC",
            Self::AudioOutputCapture { .. } => "AUDIO",
            Self::ApplicationAudioCapture { .. } => "APP-AUD",
            Self::ColorSource { .. } => "COLOR",
            Self::TextGdiPlus { .. } => "TEXT",
            Self::BrowserSource { .. } => "WEB",
            Self::SceneSource { .. } => "SCENE",
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::DisplayCapture { .. } => "Display Capture",
            Self::WindowCapture { .. } => "Window Capture",
            Self::GameCapture { .. } => "Game Capture",
            Self::WebcamDevice { .. } => "Video Capture Device",
            Self::MediaSource { .. } => "Media Source",
            Self::ImageSource { .. } => "Image",
            Self::ImageSlideShow { .. } => "Image Slide Show",
            Self::AudioInputCapture { .. } => "Audio Input Capture",
            Self::AudioOutputCapture { .. } => "Audio Output Capture",
            Self::ApplicationAudioCapture { .. } => "Application Audio Capture (BETA)",
            Self::ColorSource { .. } => "Color Source",
            Self::TextGdiPlus { .. } => "Text (GDI+)",
            Self::BrowserSource { .. } => "Browser",
            Self::SceneSource { .. } => "Scene",
        }
    }

    pub fn summary(&self) -> String {
        match self {
            Self::DisplayCapture {
                monitor_name,
                resolution,
                ..
            } => format!("{} ({}x{})", monitor_name, resolution.0, resolution.1),
            Self::WindowCapture {
                window_title,
                executable_name,
                ..
            }
            | Self::GameCapture {
                window_title,
                executable_name,
                ..
            } => format!("[{}] {}", executable_name, window_title),
            Self::WebcamDevice {
                device_name,
                resolution,
                ..
            } => format!("{} ({}x{})", device_name, resolution.0, resolution.1),
            Self::MediaSource {
                file_path,
                loop_playback,
                ..
            } => {
                let fname = std::path::Path::new(file_path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(file_path.as_str());
                format!(
                    "{}{}",
                    if fname.is_empty() { "No media file selected" } else { fname },
                    if *loop_playback { " (Loop)" } else { "" }
                )
            }
            Self::ImageSource { file_path, .. } => file_path.clone(),
            Self::ImageSlideShow { files, .. } => format!("{} image(s)", files.len()),
            Self::AudioInputCapture { device_name, .. }
            | Self::AudioOutputCapture { device_name, .. } => device_name.clone(),
            Self::ApplicationAudioCapture {
                executable_name,
                window_title,
                ..
            } => format!("[{}] {}", executable_name, window_title),
            Self::ColorSource { color_rgba, .. } => format!(
                "RGBA({}, {}, {}, {})",
                color_rgba[0], color_rgba[1], color_rgba[2], color_rgba[3]
            ),
            Self::TextGdiPlus { text, .. } => text.clone(),
            Self::BrowserSource { url, .. } => url.clone(),
            Self::SceneSource { scene_name } => scene_name.clone(),
        }
    }
}

fn default_one() -> f32 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SourceTransform {
    /// Normalized position X in canvas (0.0..1.0)
    pub pos_x: f32,
    /// Normalized position Y in canvas (0.0..1.0)
    pub pos_y: f32,
    /// Uniform scale factor (1.0 = fit 100%)
    pub scale: f32,
    /// Independent horizontal stretch multiplier (1.0 = normal)
    #[serde(default = "default_one")]
    pub scale_x: f32,
    /// Independent vertical stretch multiplier (1.0 = normal)
    #[serde(default = "default_one")]
    pub scale_y: f32,
    /// Normalized crop from left edge (0.0..0.85)
    pub crop_left: f32,
    /// Normalized crop from right edge (0.0..0.85)
    pub crop_right: f32,
    /// Normalized crop from top edge (0.0..0.85)
    pub crop_top: f32,
    /// Normalized crop from bottom edge (0.0..0.85)
    pub crop_bottom: f32,
    /// Preserve source aspect ratio
    pub lock_aspect: bool,
}

impl Default for SourceTransform {
    fn default() -> Self {
        Self {
            pos_x: 0.0,
            pos_y: 0.0,
            scale: 1.0,
            scale_x: 1.0,
            scale_y: 1.0,
            crop_left: 0.0,
            crop_right: 0.0,
            crop_top: 0.0,
            crop_bottom: 0.0,
            lock_aspect: true,
        }
    }
}

impl SourceTransform {
    pub fn effective_width_norm(&self) -> f32 {
        (self.scale * self.scale_x).clamp(0.05, 2.5)
    }

    pub fn effective_height_norm(&self) -> f32 {
        (self.scale * self.scale_y).clamp(0.05, 2.5)
    }

    pub fn has_crop(&self) -> bool {
        self.crop_left > 0.002
            || self.crop_right > 0.002
            || self.crop_top > 0.002
            || self.crop_bottom > 0.002
    }

    pub fn reset_crop(&mut self) {
        self.crop_left = 0.0;
        self.crop_right = 0.0;
        self.crop_top = 0.0;
        self.crop_bottom = 0.0;
    }

    /// Returns `(u0, v0, u1, v1)` texture UV coordinates after applying crop
    pub fn uv_bounds(&self) -> (f32, f32, f32, f32) {
        let u0 = self.crop_left.clamp(0.0, 0.90);
        let v0 = self.crop_top.clamp(0.0, 0.90);
        let u1 = (1.0 - self.crop_right).clamp(u0 + 0.05, 1.0);
        let v1 = (1.0 - self.crop_bottom).clamp(v0 + 0.05, 1.0);
        (u0, v0, u1, v1)
    }

    /// Returns normalized `(min_x, min_y, width, height)` of the visible (cropped) box in canvas space
    pub fn visible_norm_rect(&self) -> (f32, f32, f32, f32) {
        let (u0, v0, u1, v1) = self.uv_bounds();
        let full_w = self.effective_width_norm();
        let full_h = self.effective_height_norm();
        let vis_x = self.pos_x + full_w * u0;
        let vis_y = self.pos_y + full_h * v0;
        let vis_w = (full_w * (u1 - u0)).max(0.02);
        let vis_h = (full_h * (v1 - v0)).max(0.02);
        (vis_x, vis_y, vis_w, vis_h)
    }

    pub fn reset_full_canvas(&mut self) {
        self.pos_x = 0.0;
        self.pos_y = 0.0;
        self.scale = 1.0;
        self.scale_x = 1.0;
        self.scale_y = 1.0;
        self.reset_crop();
        self.lock_aspect = true;
    }

    pub fn preset_pip_bottom_right(&mut self) {
        self.pos_x = 0.70;
        self.pos_y = 0.68;
        self.scale = 0.28;
        self.scale_x = 1.0;
        self.scale_y = 1.0;
        self.lock_aspect = true;
    }

    pub fn preset_pip_bottom_left(&mut self) {
        self.pos_x = 0.02;
        self.pos_y = 0.68;
        self.scale = 0.28;
        self.scale_x = 1.0;
        self.scale_y = 1.0;
        self.lock_aspect = true;
    }

    pub fn preset_pip_top_right(&mut self) {
        self.pos_x = 0.70;
        self.pos_y = 0.04;
        self.scale = 0.28;
        self.scale_x = 1.0;
        self.scale_y = 1.0;
        self.lock_aspect = true;
    }

    pub fn preset_center(&mut self) {
        let (_, _, vis_w, vis_h) = self.visible_norm_rect();
        let full_w = self.effective_width_norm();
        let full_h = self.effective_height_norm();
        let (u0, v0, _, _) = self.uv_bounds();
        self.pos_x = (1.0 - vis_w) * 0.5 - full_w * u0;
        self.pos_y = (1.0 - vis_h) * 0.5 - full_h * v0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceColorTag {
    None,
    Blue,
    Teal,
    Green,
    Amber,
    Red,
    Purple,
}

impl Default for SourceColorTag {
    fn default() -> Self {
        Self::None
    }
}

impl SourceColorTag {
    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "Default (None)",
            Self::Blue => "Blue",
            Self::Teal => "Teal",
            Self::Green => "Green",
            Self::Amber => "Amber",
            Self::Red => "Red",
            Self::Purple => "Purple",
        }
    }

    pub fn all() -> &'static [SourceColorTag] {
        &[
            Self::None,
            Self::Blue,
            Self::Teal,
            Self::Green,
            Self::Amber,
            Self::Red,
            Self::Purple,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceItem {
    pub id: Uuid,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub kind: SourceKind,
    pub transform: SourceTransform,
    #[serde(default)]
    pub color_tag: SourceColorTag,
}

impl SourceItem {
    pub fn new(name: impl Into<String>, kind: SourceKind) -> Self {
        let mut transform = SourceTransform::default();
        if matches!(kind, SourceKind::WebcamDevice { .. }) {
            transform.preset_pip_bottom_right();
        }
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            visible: true,
            locked: false,
            kind,
            transform,
            color_tag: SourceColorTag::None,
        }
    }
}

