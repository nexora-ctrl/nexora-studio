use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    WebcamDevice {
        device_id: String,
        device_name: String,
        resolution: (u32, u32),
    },
    ImageSource {
        file_path: String,
        opacity: f32,
    },
}

impl SourceKind {
    pub fn tabler_icon(&self) -> &'static str {
        match self {
            Self::DisplayCapture { .. } => crate::ui::icons::DEVICE_DESKTOP,
            Self::WindowCapture { .. } => crate::ui::icons::APP_WINDOW,
            Self::WebcamDevice { .. } => crate::ui::icons::CAMERA,
            Self::ImageSource { .. } => crate::ui::icons::PHOTO,
        }
    }

    pub fn type_badge(&self) -> &'static str {
        match self {
            Self::DisplayCapture { .. } => "DISP",
            Self::WindowCapture { .. } => "WIN",
            Self::WebcamDevice { .. } => "CAM",
            Self::ImageSource { .. } => "IMG",
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Self::DisplayCapture { .. } => "Display Capture",
            Self::WindowCapture { .. } => "Window Capture",
            Self::WebcamDevice { .. } => "Video Capture Device",
            Self::ImageSource { .. } => "Image Source",
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
            } => format!("[{}] {}", executable_name, window_title),
            Self::WebcamDevice {
                device_name,
                resolution,
                ..
            } => format!("{} ({}x{})", device_name, resolution.0, resolution.1),
            Self::ImageSource { file_path, .. } => file_path.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SourceTransform {
    /// Normalized position X in canvas (0.0..1.0)
    pub pos_x: f32,
    /// Normalized position Y in canvas (0.0..1.0)
    pub pos_y: f32,
    /// Scale factor (1.0 = fit 100%)
    pub scale: f32,
    /// Preserve source aspect ratio
    pub lock_aspect: bool,
}

impl Default for SourceTransform {
    fn default() -> Self {
        Self {
            pos_x: 0.0,
            pos_y: 0.0,
            scale: 1.0,
            lock_aspect: true,
        }
    }
}

impl SourceTransform {
    pub fn reset_full_canvas(&mut self) {
        self.pos_x = 0.0;
        self.pos_y = 0.0;
        self.scale = 1.0;
        self.lock_aspect = true;
    }

    pub fn preset_pip_bottom_right(&mut self) {
        self.pos_x = 0.70;
        self.pos_y = 0.68;
        self.scale = 0.28;
        self.lock_aspect = true;
    }

    pub fn preset_pip_bottom_left(&mut self) {
        self.pos_x = 0.02;
        self.pos_y = 0.68;
        self.scale = 0.28;
        self.lock_aspect = true;
    }

    pub fn preset_pip_top_right(&mut self) {
        self.pos_x = 0.70;
        self.pos_y = 0.04;
        self.scale = 0.28;
        self.lock_aspect = true;
    }

    pub fn preset_center(&mut self) {
        let s = self.scale.clamp(0.15, 1.0);
        self.pos_x = (1.0 - s) * 0.5;
        self.pos_y = (1.0 - s) * 0.5;
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

