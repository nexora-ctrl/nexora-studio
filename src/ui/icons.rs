use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// Embedded offline Tabler Icons TrueType font (`assets/icons/tabler-icons.ttf`, v3.31.0, 5,936 icons).
/// Also accompanied in `assets/icons/` by `tabler-sprite.svg` and `tabler-icons-codepoints.json`
/// for offline reference and future expansion.
const TABLER_ICONS_TTF: &[u8] = include_bytes!("../../assets/icons/tabler-icons.ttf");

/// Registers the offline Tabler Icons font as a fallback for both Proportional and Monospace text in `egui`.
pub fn install_tabler_icons(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "tabler-icons".to_owned(),
        Arc::new(FontData::from_static(TABLER_ICONS_TTF)),
    );

    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.push("tabler-icons".to_owned());
    }
    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
        family.push("tabler-icons".to_owned());
    }

    ctx.set_fonts(fonts);
}

// ============================================================================
// Tabler Icons Unicode Constants (https://tabler.io/icons)
// Full 5,936 icon codepoint lookup is saved offline at:
// `assets/icons/tabler-icons-codepoints.json`
// ============================================================================

/// `ti-device-desktop` — Display / Monitor Capture
pub const DEVICE_DESKTOP: &str = "\u{ea89}";
/// `ti-app-window` — Application Window Capture
pub const APP_WINDOW: &str = "\u{efe6}";
/// `ti-video` — Video Capture Device
pub const VIDEO: &str = "\u{ed22}";
/// `ti-camera` — Camera / Webcam
pub const CAMERA: &str = "\u{ea54}";
/// `ti-photo` — Image / Overlay Source
pub const PHOTO: &str = "\u{eb0a}";

/// `ti-eye` — Source Layer Visible
pub const EYE: &str = "\u{ea9a}";
/// `ti-eye-off` — Source Layer Hidden
pub const EYE_OFF: &str = "\u{ecf0}";
/// `ti-lock` — Source Layer Locked
pub const LOCK: &str = "\u{eae2}";
/// `ti-lock-open` — Source Layer Unlocked
pub const LOCK_OPEN: &str = "\u{eae1}";

/// `ti-plus` — Add Scene / Add Source
pub const PLUS: &str = "\u{eb0b}";
/// `ti-minus` — Remove Scene / Remove Source
pub const MINUS: &str = "\u{eaf2}";
/// `ti-arrow-up` — Move Layer Up
pub const ARROW_UP: &str = "\u{ea25}";
/// `ti-arrow-down` — Move Layer Down
pub const ARROW_DOWN: &str = "\u{ea16}";
/// `ti-arrows-up-down` — Layer Order
pub const ARROWS_UP_DOWN: &str = "\u{edb6}";

/// `ti-settings` — Properties / Studio Settings
pub const SETTINGS: &str = "\u{eb20}";
/// `ti-trash` — Delete / Remove Item
pub const TRASH: &str = "\u{eb41}";
/// `ti-copy` — Duplicate / Copy Item
pub const COPY: &str = "\u{ea7a}";
/// `ti-edit` — Rename / Edit Item
pub const EDIT: &str = "\u{ea98}";

/// `ti-volume` — Desktop Audio Active
pub const VOLUME: &str = "\u{eb51}";
/// `ti-volume-off` — Desktop Audio Muted
pub const VOLUME_OFF: &str = "\u{f1c3}";
/// `ti-microphone` — Microphone Active
pub const MICROPHONE: &str = "\u{eaf0}";
/// `ti-microphone-off` — Microphone Muted
pub const MICROPHONE_OFF: &str = "\u{ed16}";

/// `ti-player-record-filled` — Start Recording
pub const PLAYER_RECORD: &str = "\u{f692}";
/// `ti-player-pause` — Pause Recording
pub const PLAYER_PAUSE: &str = "\u{ed45}";
/// `ti-player-play` — Play / Resume Recording
pub const PLAYER_PLAY: &str = "\u{ed46}";
/// `ti-player-stop` — Stop Recording
pub const PLAYER_STOP: &str = "\u{ed4a}";

/// `ti-folder` — Folder
pub const FOLDER: &str = "\u{eaad}";
/// `ti-folder-open` — Open Folder / Open Project
pub const FOLDER_OPEN: &str = "\u{faf7}";
/// `ti-file` — New Project File
pub const FILE: &str = "\u{eaa4}";
/// `ti-device-floppy` — Save Project
pub const DEVICE_FLOPPY: &str = "\u{eb62}";

/// `ti-sun` — Light Mode Theme
pub const SUN: &str = "\u{eb30}";
/// `ti-moon` — Dark Mode Theme
pub const MOON: &str = "\u{eaf8}";
/// `ti-refresh` — Refresh / Re-scan Devices
pub const REFRESH: &str = "\u{eb13}";
/// `ti-layout-columns` — Studio Dual-Viewport Mode
pub const LAYOUT_COLUMNS: &str = "\u{ead4}";
/// `ti-arrow-right` — Cut / Transition to Program
pub const ARROW_RIGHT: &str = "\u{ea1f}";
/// `ti-palette` — Set Color Label
pub const PALETTE: &str = "\u{eb01}";
/// `ti-transform` — Transform / PiP Layout
pub const TRANSFORM: &str = "\u{f38e}";
/// `ti-maximize` — Maximize / Fit
pub const MAXIMIZE: &str = "\u{eaea}";
/// `ti-minimize` — Minimize Panel
pub const MINIMIZE: &str = "\u{eaf1}";
/// `ti-movie` — Recordings Media Library
pub const MOVIE: &str = "\u{eafa}";

