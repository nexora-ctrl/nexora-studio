use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// Embedded offline Tabler Icons TrueType font (`assets/icons/tabler-icons.ttf`, v3.31.0, 5,936 icons).
/// Also accompanied in `assets/icons/` by `tabler-sprite.svg` and `tabler-icons-codepoints.json`
/// for offline reference and future expansion.
const TABLER_ICONS_TTF: &[u8] = include_bytes!("../../assets/icons/tabler-icons.ttf");
const APP_ICON_ICO: &[u8] = include_bytes!("../../assets/icons/icon.ico");

/// Decodes a 32-bit BGRA DIB entry from the embedded `assets/icons/icon.ico`
pub fn decode_embedded_ico_rgba(preferred_width: u32) -> Option<(u32, u32, Vec<u8>)> {
    let bytes = APP_ICON_ICO;
    if bytes.len() < 22 {
        return None;
    }
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    if count == 0 || bytes.len() < 6 + count * 16 {
        return None;
    }

    let mut chosen_idx = 0usize;
    let mut best_diff = u32::MAX;

    for i in 0..count {
        let off = 6 + i * 16;
        let w = if bytes[off] == 0 {
            256u32
        } else {
            bytes[off] as u32
        };
        let diff = w.abs_diff(preferred_width);
        if diff < best_diff {
            best_diff = diff;
            chosen_idx = i;
        }
    }

    let off = 6 + chosen_idx * 16;
    let width = if bytes[off] == 0 {
        256usize
    } else {
        bytes[off] as usize
    };
    let height = if bytes[off + 1] == 0 {
        256usize
    } else {
        bytes[off + 1] as usize
    };
    let size = u32::from_le_bytes([
        bytes[off + 8],
        bytes[off + 9],
        bytes[off + 10],
        bytes[off + 11],
    ]) as usize;
    let data_off = u32::from_le_bytes([
        bytes[off + 12],
        bytes[off + 13],
        bytes[off + 14],
        bytes[off + 15],
    ]) as usize;

    if data_off + size > bytes.len() || size < 40 {
        return None;
    }

    let header_size = u32::from_le_bytes([
        bytes[data_off],
        bytes[data_off + 1],
        bytes[data_off + 2],
        bytes[data_off + 3],
    ]) as usize;
    let pixel_off = data_off + header_size;
    let pixel_bytes = width * height * 4;
    if pixel_off + pixel_bytes > bytes.len() {
        return None;
    }

    let src = &bytes[pixel_off..pixel_off + pixel_bytes];
    let mut rgba = vec![0u8; pixel_bytes];

    // ICO DIB pixels are stored bottom-up in BGRA order
    for y in 0..height {
        let src_row = (height - 1 - y) * width * 4;
        let dst_row = y * width * 4;
        for x in 0..width {
            let s = src_row + x * 4;
            let d = dst_row + x * 4;
            rgba[d] = src[s + 2];     // R
            rgba[d + 1] = src[s + 1]; // G
            rgba[d + 2] = src[s];     // B
            rgba[d + 3] = src[s + 3]; // A
        }
    }

    Some((width as u32, height as u32, rgba))
}

/// Loads `assets/icons/icon.ico` into `egui::IconData` for the native window and taskbar icon
pub fn load_app_window_icon() -> Option<Arc<egui::IconData>> {
    let (width, height, rgba) = decode_embedded_ico_rgba(64)
        .or_else(|| decode_embedded_ico_rgba(256))?;
    Some(Arc::new(egui::IconData {
        rgba,
        width,
        height,
    }))
}

/// Registers the offline Tabler Icons font as a fallback for both Proportional and Monospace text in `egui`.
pub fn install_tabler_icons(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // 1. Tabler Icons (5,936 offline icon glyphs)
    fonts.font_data.insert(
        "tabler-icons".to_owned(),
        Arc::new(FontData::from_static(TABLER_ICONS_TTF)),
    );

    // 2. Rubik font family (Embedded Offline: Light 300, Regular 400, Medium 500, SemiBold 600)
    fonts.font_data.insert(
        RUBIK_LIGHT_NAME.to_owned(),
        Arc::new(FontData::from_static(RUBIK_LIGHT_TTF)),
    );
    fonts.font_data.insert(
        RUBIK_REGULAR_NAME.to_owned(),
        Arc::new(FontData::from_static(RUBIK_REGULAR_TTF)),
    );
    fonts.font_data.insert(
        RUBIK_MEDIUM_NAME.to_owned(),
        Arc::new(FontData::from_static(RUBIK_MEDIUM_TTF)),
    );
    fonts.font_data.insert(
        RUBIK_SEMIBOLD_NAME.to_owned(),
        Arc::new(FontData::from_static(RUBIK_SEMIBOLD_TTF)),
    );

    // 3. Brand Title Font: Exo Medium (weight 500), embedded offline from `assets/fonts/Exo-Medium.ttf`
    fonts.font_data.insert(
        BRAND_FONT_NAME.to_owned(),
        Arc::new(FontData::from_static(EXO_MEDIUM_TTF)),
    );

    let default_fallbacks = vec![
        "tabler-icons".to_owned(),
        "emoji-icon-font".to_owned(),
        "NotoEmoji-Regular".to_owned(),
        "Ubuntu-Light".to_owned(),
    ];

    // Make Rubik Regular (weight 400, clean, clear, perfectly balanced) the primary font across the entire UI
    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.insert(0, RUBIK_REGULAR_NAME.to_owned());
        family.push(RUBIK_MEDIUM_NAME.to_owned());
        family.push(RUBIK_LIGHT_NAME.to_owned());
        family.push("tabler-icons".to_owned());
    }
    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
        family.push("tabler-icons".to_owned());
    }

    // Specific named families with complete fallbacks (tabler-icons, emoji-icon-font, NotoEmoji, Ubuntu-Light)
    let make_family_list = |primary: &str| -> Vec<String> {
        let mut list = vec![primary.to_owned()];
        list.extend(default_fallbacks.clone());
        list
    };

    fonts.families.insert(
        FontFamily::Name(RUBIK_LIGHT_NAME.into()),
        make_family_list(RUBIK_LIGHT_NAME),
    );
    fonts.families.insert(
        FontFamily::Name(RUBIK_REGULAR_NAME.into()),
        make_family_list(RUBIK_REGULAR_NAME),
    );
    fonts.families.insert(
        FontFamily::Name(RUBIK_MEDIUM_NAME.into()),
        make_family_list(RUBIK_MEDIUM_NAME),
    );
    fonts.families.insert(
        FontFamily::Name(RUBIK_SEMIBOLD_NAME.into()),
        make_family_list(RUBIK_SEMIBOLD_NAME),
    );
    fonts.families.insert(
        FontFamily::Name(BRAND_FONT_NAME.into()),
        make_family_list(BRAND_FONT_NAME),
    );

    ctx.set_fonts(fonts);
}

/// Rubik Light (weight 300) — SIL Open Font License, see `assets/fonts/Rubik-OFL.txt`
const RUBIK_LIGHT_TTF: &[u8] = include_bytes!("../../assets/fonts/Rubik-Light.ttf");
/// Rubik Regular (weight 400) — SIL Open Font License, see `assets/fonts/Rubik-OFL.txt`
const RUBIK_REGULAR_TTF: &[u8] = include_bytes!("../../assets/fonts/Rubik-Regular.ttf");
/// Rubik Medium (weight 500) — SIL Open Font License
const RUBIK_MEDIUM_TTF: &[u8] = include_bytes!("../../assets/fonts/Rubik-Medium.ttf");
/// Rubik SemiBold (weight 600) — SIL Open Font License
const RUBIK_SEMIBOLD_TTF: &[u8] = include_bytes!("../../assets/fonts/Rubik-SemiBold.ttf");

/// Exo Medium (wght 500) — SIL Open Font License, see `assets/fonts/Exo-OFL.txt`
const EXO_MEDIUM_TTF: &[u8] = include_bytes!("../../assets/fonts/Exo-Medium.ttf");

pub const RUBIK_LIGHT_NAME: &str = "rubik-light";
pub const RUBIK_REGULAR_NAME: &str = "rubik-regular";
pub const RUBIK_MEDIUM_NAME: &str = "rubik-medium";
pub const RUBIK_SEMIBOLD_NAME: &str = "rubik-semibold";
pub const BRAND_FONT_NAME: &str = "exo-medium";

/// Font family handle for the brand title (Exo 500, works fully offline)
pub fn brand_font_family() -> FontFamily {
    FontFamily::Name(BRAND_FONT_NAME.into())
}

/// Font family handle for Rubik Light (weight 300, slender & clean)
pub fn rubik_light_family() -> FontFamily {
    FontFamily::Name(RUBIK_LIGHT_NAME.into())
}

/// Font family handle for Rubik Regular (weight 400)
pub fn rubik_family() -> FontFamily {
    FontFamily::Name(RUBIK_REGULAR_NAME.into())
}

/// Font family handle for Rubik Medium (weight 500)
pub fn rubik_medium_family() -> FontFamily {
    FontFamily::Name(RUBIK_MEDIUM_NAME.into())
}

/// Font family handle for Rubik SemiBold (weight 600)
pub fn rubik_semibold_family() -> FontFamily {
    FontFamily::Name(RUBIK_SEMIBOLD_NAME.into())
}

/// Helper to produce a FontId with Rubik Light at given size
pub fn font_rubik_light(size: f32) -> egui::FontId {
    egui::FontId::new(size, rubik_light_family())
}

/// Helper to produce a FontId with Rubik Regular at given size
pub fn font_rubik(size: f32) -> egui::FontId {
    egui::FontId::new(size, rubik_family())
}

/// Helper to produce a FontId with Rubik Medium at given size
pub fn font_rubik_medium(size: f32) -> egui::FontId {
    egui::FontId::new(size, rubik_medium_family())
}

/// Helper to produce a FontId with Rubik SemiBold at given size
pub fn font_rubik_semibold(size: f32) -> egui::FontId {
    egui::FontId::new(size, rubik_semibold_family())
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
/// `ti-clipboard` — Paste / Clipboard Item
pub const CLIPBOARD: &str = "\u{ea6f}";
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
/// `ti-arrow-left` — Previous Slide / Left Navigation
pub const ARROW_LEFT: &str = "\u{ea19}";
/// `ti-arrow-right` — Cut / Transition to Program / Next Slide
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

/// `ti-world` — Browser Source
pub const WORLD: &str = "\u{eb54}";
/// `ti-brush` — Color Source
pub const BRUSH: &str = "\u{ebb8}";
/// `ti-device-gamepad` — Game Capture
pub const DEVICE_GAMEPAD: &str = "\u{eb63}";
/// `ti-presentation` — Image Slide Show
pub const PRESENTATION: &str = "\u{eb70}";
/// `ti-pdf` — PDF Document / Presentation
pub const PDF: &str = "\u{f7ac}";
/// `ti-file-type-pdf` — PDF File Type Badge
pub const FILE_TYPE_PDF: &str = "\u{fb10}";
/// `ti-list` — Scene Source
pub const LIST: &str = "\u{eb6b}";
/// `ti-layout-grid` — Grid Mode View
pub const LAYOUT_GRID: &str = "\u{edba}";
/// `ti-letter-case` — Text (GDI+) Source
pub const LETTER_CASE: &str = "\u{eea5}";
/// `ti-music` — Audio / MP3 Media
pub const MUSIC: &str = "\u{eafc}";
/// `ti-arrow-back-up` — Undo Action
pub const UNDO: &str = "\u{eb77}";
/// `ti-arrow-forward-up` — Redo Action
pub const REDO: &str = "\u{eb78}";
/// `ti-filter` — Video Filters (Chroma Key, Color Key, Luma Key)
pub const FILTER: &str = "\u{eaa5}";
/// `ti-adjustments` — Adjustments / Sliders
pub const ADJUSTMENTS: &str = "\u{ea03}";
/// `ti-wand` — Keying FX Magic Wand
pub const WAND: &str = "\u{ebcb}";
/// `ti-color-picker` — Screen Color Picker / Eyedropper Pipet
pub const COLOR_PICKER: &str = "\u{ebe6}";

/// `ti-chevron-up` — Scroll Up / Menu Up Indicator
pub const CHEVRON_UP: &str = "\u{ea62}";
/// `ti-chevron-down` — Scroll Down / Dropdown Menu Indicator
pub const CHEVRON_DOWN: &str = "\u{ea5f}";
/// `ti-clock` — Time / Clock Source
pub const CLOCK: &str = "\u{ea70}";
/// `ti-stopwatch` — Stopwatch / Count-Up Timer
pub const STOPWATCH: &str = "\u{ff9b}";
/// `ti-hourglass` — Countdown Hourglass
pub const HOURGLASS: &str = "\u{ef93}";



