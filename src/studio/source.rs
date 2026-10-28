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

fn default_scroll_speed() -> f32 {
    95.0
}

fn default_loop_gap_px() -> f32 {
    80.0
}

fn default_text_bg_rgba() -> [u8; 4] {
    [15, 23, 42, 225]
}

fn default_text_accent_rgba() -> [u8; 4] {
    [220, 38, 38, 255]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TextAnimationEffect {
    #[default]
    Static,
    ScrollLeft,
    ScrollRight,
    ScrollUp,
    BounceHorizontal,
    Typewriter,
    PulseGlow,
    FadeInOut,
}

impl TextAnimationEffect {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Static => "Static (Diam)",
            Self::ScrollLeft => "Running Text <- Ke Kiri (TV Ticker)",
            Self::ScrollRight => "Running Text -> Ke Kanan",
            Self::ScrollUp => "Scroll Ke Atas (Credits Roll)",
            Self::BounceHorizontal => "Pantul Kiri <-> Kanan (Ping-Pong)",
            Self::Typewriter => "Efek Ketik Berjalan (Typewriter)",
            Self::PulseGlow => "Berdenyut Halus (Pulse Live)",
            Self::FadeInOut => "Fade In / Out Berulang",
        }
    }

    pub fn is_scrolling(&self) -> bool {
        matches!(
            self,
            Self::ScrollLeft | Self::ScrollRight | Self::ScrollUp | Self::BounceHorizontal
        )
    }

    pub fn is_animated(&self) -> bool {
        *self != Self::Static
    }

    pub fn all() -> &'static [TextAnimationEffect] {
        &[
            Self::Static,
            Self::ScrollLeft,
            Self::ScrollRight,
            Self::ScrollUp,
            Self::BounceHorizontal,
            Self::Typewriter,
            Self::PulseGlow,
            Self::FadeInOut,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TvBannerStyle {
    #[default]
    PlainText,
    SolidBanner,
    BreakingNewsTv,
    LowerThirdCard,
    HeadlineStrap,
    CornerBugLive,
}

impl TvBannerStyle {
    pub fn label(&self) -> &'static str {
        match self {
            Self::PlainText => "Teks Polos (Tanpa Kotak Latar)",
            Self::SolidBanner => "Banner Latar Solid (Ticker Strip)",
            Self::BreakingNewsTv => "TV Breaking News (Badge + Running Bar)",
            Self::LowerThirdCard => "TV Lower-Third (Kartu Nama / Acara)",
            Self::HeadlineStrap => "TV Headline Strap (Garis Aksen Atas/Bawah)",
            Self::CornerBugLive => "TV Corner Bug (Kapsul Pojok LIVE)",
        }
    }

    pub fn uses_badge(&self) -> bool {
        matches!(
            self,
            Self::BreakingNewsTv
                | Self::LowerThirdCard
                | Self::HeadlineStrap
                | Self::CornerBugLive
        )
    }

    pub fn uses_background(&self) -> bool {
        *self != Self::PlainText
    }

    pub fn all() -> &'static [TvBannerStyle] {
        &[
            Self::PlainText,
            Self::SolidBanner,
            Self::BreakingNewsTv,
            Self::LowerThirdCard,
            Self::HeadlineStrap,
            Self::CornerBugLive,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TvTextTemplate {
    BreakingNewsTicker,
    InfoRunningMarquee,
    TalkshowLowerThird,
    HeadlineBroadcastBanner,
    LiveCornerBug,
    EndingCreditsRoll,
}

impl TvTextTemplate {
    pub fn label(&self) -> &'static str {
        match self {
            Self::BreakingNewsTicker => "TV Breaking News Ticker (Bawah)",
            Self::InfoRunningMarquee => "TV Running Text Info (Marquee)",
            Self::TalkshowLowerThird => "TV Talkshow Lower-Third (Kiri Bawah)",
            Self::HeadlineBroadcastBanner => "TV Headline Strap (Judul Utama)",
            Self::LiveCornerBug => "TV LIVE Corner Bug (Pojok Atas)",
            Self::EndingCreditsRoll => "TV Ending Credits (Roll ke Atas)",
        }
    }

    pub fn all() -> &'static [TvTextTemplate] {
        &[
            Self::BreakingNewsTicker,
            Self::InfoRunningMarquee,
            Self::TalkshowLowerThird,
            Self::HeadlineBroadcastBanner,
            Self::LiveCornerBug,
            Self::EndingCreditsRoll,
        ]
    }
}

fn default_slide_transition_ms() -> u32 {
    650
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SlideTransitionEffect {
    Cut,
    #[default]
    CrossFade,
    SlideLeft,
    SlideRight,
    SlideUp,
    SlideDown,
    ZoomInKenBurns,
    ZoomOut,
}

impl SlideTransitionEffect {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Cut => "Cut (Berganti Instan)",
            Self::CrossFade => "Fade / Dissolve (Transisi Halus)",
            Self::SlideLeft => "Geser ke Kiri (Slide Left)",
            Self::SlideRight => "Geser ke Kanan (Slide Right)",
            Self::SlideUp => "Geser ke Atas (Slide Up)",
            Self::SlideDown => "Geser ke Bawah (Slide Down)",
            Self::ZoomInKenBurns => "Zoom In Sinematik (Ken Burns)",
            Self::ZoomOut => "Zoom Out Reveal",
        }
    }

    pub fn all() -> &'static [SlideTransitionEffect] {
        &[
            Self::Cut,
            Self::CrossFade,
            Self::SlideLeft,
            Self::SlideRight,
            Self::SlideUp,
            Self::SlideDown,
            Self::ZoomInKenBurns,
            Self::ZoomOut,
        ]
    }
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
        #[serde(default)]
        transition_effect: SlideTransitionEffect,
        #[serde(default = "default_slide_transition_ms")]
        transition_duration_ms: u32,
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
        #[serde(default)]
        effect: TextAnimationEffect,
        #[serde(default = "default_scroll_speed")]
        scroll_speed: f32,
        #[serde(default = "default_true")]
        loop_animation: bool,
        #[serde(default = "default_loop_gap_px")]
        loop_gap_px: f32,
        #[serde(default)]
        banner_style: TvBannerStyle,
        #[serde(default = "default_text_bg_rgba")]
        bg_color_rgba: [u8; 4],
        #[serde(default = "default_text_accent_rgba")]
        accent_color_rgba: [u8; 4],
        #[serde(default)]
        badge_text: String,
        #[serde(default = "default_true")]
        text_shadow: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum KeyFilterKind {
    #[default]
    None,
    ChromaKey,
    ColorKey,
    LumaKey,
}

impl KeyFilterKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "None (Original / Tanpa Filter)",
            Self::ChromaKey => "Chroma Key (Green / Blue Screen)",
            Self::ColorKey => "Color Key (Hapus Warna Tertentu)",
            Self::LumaKey => "Luma Key (Hapus Gelap / Terang)",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::ChromaKey => "Chroma Key",
            Self::ColorKey => "Color Key",
            Self::LumaKey => "Luma Key",
        }
    }

    pub fn hover_explanation(&self) -> &'static str {
        match self {
            Self::None => "Menampilkan gambar/video asli tanpa transparansi atau pemotongan warna latar.",
            Self::ChromaKey => "Chroma Key: Menghapus latar belakang Green Screen atau Blue Screen secara halus.\n• Cocok untuk: Video presenter/webcam dengan kain hijau/biru, animasi transisi / subscribe green screen.\n• Dilengkapi: Fitur Spill Reduction untuk membersihkan sisa pantulan warna hijau di tepi rambut/badan objek.",
            Self::ColorKey => "Color Key: Menghapus satu warna spesifik pilihan Anda menjadi transparan.\n• Cocok untuk: Logo, stiker, atau grafik overlay yang memiliki latar belakang satu warna solid (misal putih, merah, atau biru).\n• Fleksibel: Warna target dapat dipilih langsung menggunakan color picker.",
            Self::LumaKey => "Luma Key: Menghapus bagian gambar/video berdasarkan tingkat kecerahan / gelap-terang (Luminance).\n• Cocok untuk: Menghilangkan background hitam pekat (seperti video efek api, partikel debu, kilatan cahaya, ledakan, atau logo berlatar hitam pekat tanpa green screen).\n• Juga dapat memotong area putih terang jika disesuaikan.",
        }
    }

    pub fn all() -> &'static [KeyFilterKind] {
        &[Self::None, Self::ChromaKey, Self::ColorKey, Self::LumaKey]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ChromaKeyPresetColor {
    #[default]
    Green,
    Blue,
    Magenta,
    Custom,
}

impl ChromaKeyPresetColor {
    pub fn rgb(&self) -> [u8; 3] {
        match self {
            Self::Green => [0, 255, 0],
            Self::Blue => [0, 71, 187],
            Self::Magenta => [255, 0, 255],
            Self::Custom => [0, 255, 0],
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Green => "Green Screen (Hijau Standard)",
            Self::Blue => "Blue Screen (Biru Studio)",
            Self::Magenta => "Magenta Screen",
            Self::Custom => "Custom Color",
        }
    }

    pub fn all() -> &'static [ChromaKeyPresetColor] {
        &[Self::Green, Self::Blue, Self::Magenta, Self::Custom]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceFilterConfig {
    pub kind: KeyFilterKind,
    pub enabled: bool,
    pub key_color_preset: ChromaKeyPresetColor,
    pub key_color_rgb: [u8; 3],
    /// Similarity threshold (1..=1000)
    pub similarity: u32,
    /// Smoothness feathering (1..=1000)
    pub smoothness: u32,
    /// Chroma spill reduction (1..=1000)
    pub spill_reduction: u32,
    /// Luma Key minimum cut (cuts dark/black pixels) 0.0..=1.0
    pub luma_min: f32,
    /// Luma Key minimum smoothness feathering 0.0..=0.5
    pub luma_min_smooth: f32,
    /// Luma Key maximum cut (cuts bright/white pixels) 0.0..=1.0
    pub luma_max: f32,
    /// Luma Key maximum smoothness feathering 0.0..=0.5
    pub luma_max_smooth: f32,
    /// Overall layer opacity (0.0..=1.0)
    pub opacity: f32,
}

impl Default for SourceFilterConfig {
    fn default() -> Self {
        Self {
            kind: KeyFilterKind::None,
            enabled: true,
            key_color_preset: ChromaKeyPresetColor::Green,
            key_color_rgb: [0, 255, 0],
            similarity: 400,
            smoothness: 80,
            spill_reduction: 100,
            luma_min: 0.08,
            luma_min_smooth: 0.06,
            luma_max: 1.00,
            luma_max_smooth: 0.04,
            opacity: 1.0,
        }
    }
}

impl SourceFilterConfig {
    pub fn is_active(&self) -> bool {
        self.enabled && self.kind != KeyFilterKind::None
    }

    pub fn cache_signature(&self) -> String {
        if !self.is_active() {
            return "NONE".to_string();
        }
        match self.kind {
            KeyFilterKind::None => "NONE".to_string(),
            KeyFilterKind::ChromaKey => format!(
                "CHK:{:02x}{:02x}{:02x}:{}:{}:{}",
                self.key_color_rgb[0],
                self.key_color_rgb[1],
                self.key_color_rgb[2],
                self.similarity,
                self.smoothness,
                self.spill_reduction
            ),
            KeyFilterKind::ColorKey => format!(
                "CLK:{:02x}{:02x}{:02x}:{}:{}",
                self.key_color_rgb[0],
                self.key_color_rgb[1],
                self.key_color_rgb[2],
                self.similarity,
                self.smoothness
            ),
            KeyFilterKind::LumaKey => format!(
                "LUM:{:.3}:{:.3}:{:.3}:{:.3}",
                self.luma_min,
                self.luma_min_smooth,
                self.luma_max,
                self.luma_max_smooth
            ),
        }
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
    #[serde(default)]
    pub filter: SourceFilterConfig,
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
            filter: SourceFilterConfig::default(),
        }
    }

    pub fn apply_tv_text_template(&mut self, template: TvTextTemplate) {
        self.transform.reset_crop();
        match template {
            TvTextTemplate::BreakingNewsTicker => {
                self.name = "TV Breaking News Ticker".to_string();
                self.transform.pos_x = 0.0;
                self.transform.pos_y = 0.90;
                self.transform.scale = 1.0;
                self.transform.scale_x = 1.0;
                self.transform.scale_y = 0.10;
                self.transform.lock_aspect = false;
                self.kind = SourceKind::TextGdiPlus {
                    text: "SELAMAT DATANG DI NEXORA STUDIO LIVE • SIARAN BERKUALITAS TINGGI FULL HD 60 FPS • UPDATE INFORMASI TERKINI SETIAP SAAT".to_string(),
                    font_size: 22.0,
                    color_rgba: [255, 255, 255, 255],
                    effect: TextAnimationEffect::ScrollLeft,
                    scroll_speed: 110.0,
                    loop_animation: true,
                    loop_gap_px: 90.0,
                    banner_style: TvBannerStyle::BreakingNewsTv,
                    bg_color_rgba: [15, 23, 42, 235],
                    accent_color_rgba: [220, 38, 38, 255],
                    badge_text: "BREAKING NEWS".to_string(),
                    text_shadow: true,
                };
            }
            TvTextTemplate::InfoRunningMarquee => {
                self.name = "TV Running Text Info".to_string();
                self.transform.pos_x = 0.0;
                self.transform.pos_y = 0.92;
                self.transform.scale = 1.0;
                self.transform.scale_x = 1.0;
                self.transform.scale_y = 0.08;
                self.transform.lock_aspect = false;
                self.kind = SourceKind::TextGdiPlus {
                    text: "Jangan lupa ikuti agenda acara hari ini • Hubungi redaksi untuk informasi lebih lanjut • Terima kasih telah menyaksikan".to_string(),
                    font_size: 20.0,
                    color_rgba: [254, 240, 138, 255],
                    effect: TextAnimationEffect::ScrollLeft,
                    scroll_speed: 85.0,
                    loop_animation: true,
                    loop_gap_px: 80.0,
                    banner_style: TvBannerStyle::SolidBanner,
                    bg_color_rgba: [9, 14, 26, 230],
                    accent_color_rgba: [245, 158, 11, 255],
                    badge_text: "INFO".to_string(),
                    text_shadow: true,
                };
            }
            TvTextTemplate::TalkshowLowerThird => {
                self.name = "TV Talkshow Lower-Third".to_string();
                self.transform.pos_x = 0.04;
                self.transform.pos_y = 0.77;
                self.transform.scale = 0.44;
                self.transform.scale_x = 1.0;
                self.transform.scale_y = 0.28;
                self.transform.lock_aspect = false;
                self.kind = SourceKind::TextGdiPlus {
                    text: "Bpk. Ahmad Pratama — Pembicara Utama".to_string(),
                    font_size: 24.0,
                    color_rgba: [255, 255, 255, 255],
                    effect: TextAnimationEffect::Static,
                    scroll_speed: 80.0,
                    loop_animation: true,
                    loop_gap_px: 80.0,
                    banner_style: TvBannerStyle::LowerThirdCard,
                    bg_color_rgba: [15, 23, 42, 230],
                    accent_color_rgba: [14, 165, 233, 255],
                    badge_text: "LIVE TALKSHOW".to_string(),
                    text_shadow: true,
                };
            }
            TvTextTemplate::HeadlineBroadcastBanner => {
                self.name = "TV Headline Banner".to_string();
                self.transform.pos_x = 0.08;
                self.transform.pos_y = 0.81;
                self.transform.scale = 0.84;
                self.transform.scale_x = 1.0;
                self.transform.scale_y = 0.14;
                self.transform.lock_aspect = false;
                self.kind = SourceKind::TextGdiPlus {
                    text: "LAPORAN UTAMA: TRANSFORMASI DIGITAL & TEKNOLOGI MASA DEPAN".to_string(),
                    font_size: 24.0,
                    color_rgba: [255, 255, 255, 255],
                    effect: TextAnimationEffect::Typewriter,
                    scroll_speed: 95.0,
                    loop_animation: true,
                    loop_gap_px: 80.0,
                    banner_style: TvBannerStyle::HeadlineStrap,
                    bg_color_rgba: [17, 24, 39, 235],
                    accent_color_rgba: [239, 68, 68, 255],
                    badge_text: "HEADLINE".to_string(),
                    text_shadow: true,
                };
            }
            TvTextTemplate::LiveCornerBug => {
                self.name = "TV LIVE Corner Bug".to_string();
                self.transform.pos_x = 0.77;
                self.transform.pos_y = 0.04;
                self.transform.scale = 0.19;
                self.transform.scale_x = 1.0;
                self.transform.scale_y = 0.34;
                self.transform.lock_aspect = false;
                self.kind = SourceKind::TextGdiPlus {
                    text: "STUDIO 1 HD".to_string(),
                    font_size: 18.0,
                    color_rgba: [255, 255, 255, 255],
                    effect: TextAnimationEffect::PulseGlow,
                    scroll_speed: 80.0,
                    loop_animation: true,
                    loop_gap_px: 60.0,
                    banner_style: TvBannerStyle::CornerBugLive,
                    bg_color_rgba: [15, 23, 42, 220],
                    accent_color_rgba: [220, 38, 38, 255],
                    badge_text: "LIVE".to_string(),
                    text_shadow: true,
                };
            }
            TvTextTemplate::EndingCreditsRoll => {
                self.name = "TV Ending Credits Roll".to_string();
                self.transform.pos_x = 0.18;
                self.transform.pos_y = 0.12;
                self.transform.scale = 0.64;
                self.transform.scale_x = 1.0;
                self.transform.scale_y = 1.15;
                self.transform.lock_aspect = false;
                self.kind = SourceKind::TextGdiPlus {
                    text: "TIM PRODUKSI NEXORA STUDIO\nEksekutif Produser: Studio Lead\nPengarah Acara: Broadcast Director\nEditor Visual: Creative Team\nTerima Kasih Atas Perhatian Anda".to_string(),
                    font_size: 24.0,
                    color_rgba: [255, 255, 255, 255],
                    effect: TextAnimationEffect::ScrollUp,
                    scroll_speed: 55.0,
                    loop_animation: true,
                    loop_gap_px: 70.0,
                    banner_style: TvBannerStyle::PlainText,
                    bg_color_rgba: [15, 23, 42, 180],
                    accent_color_rgba: [56, 189, 248, 255],
                    badge_text: String::new(),
                    text_shadow: true,
                };
            }
        }
    }
}

