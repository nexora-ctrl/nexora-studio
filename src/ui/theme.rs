use crate::settings::ThemeMode;
use egui::{Color32, CornerRadius, FontId, Margin, Stroke, Style, TextStyle, Visuals};

#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub bg_app: Color32,
    pub bg_panel: Color32,
    pub bg_elevated: Color32,
    pub bg_canvas_well: Color32,
    pub border_subtle: Color32,
    pub border_strong: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub accent_primary: Color32,
    pub accent_preview: Color32,
    pub accent_program: Color32,
    pub rec_red: Color32,
    pub pause_amber: Color32,
    pub meter_ok: Color32,
    pub meter_warn: Color32,
    pub meter_clip: Color32,
}

impl ThemePalette {
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self {
                bg_app: Color32::from_rgb(15, 17, 22),
                bg_panel: Color32::from_rgb(22, 25, 32),
                bg_elevated: Color32::from_rgb(30, 34, 44),
                bg_canvas_well: Color32::from_rgb(11, 13, 17),
                border_subtle: Color32::from_rgb(44, 50, 64),
                border_strong: Color32::from_rgb(68, 77, 98),
                text_primary: Color32::from_rgb(236, 240, 248),
                text_secondary: Color32::from_rgb(176, 184, 201),
                text_muted: Color32::from_rgb(122, 130, 148),
                accent_primary: Color32::from_rgb(56, 139, 253),
                accent_preview: Color32::from_rgb(34, 184, 207),
                accent_program: Color32::from_rgb(240, 68, 56),
                rec_red: Color32::from_rgb(239, 68, 68),
                pause_amber: Color32::from_rgb(245, 158, 11),
                meter_ok: Color32::from_rgb(34, 197, 94),
                meter_warn: Color32::from_rgb(234, 179, 8),
                meter_clip: Color32::from_rgb(239, 68, 68),
            },
            ThemeMode::Light => Self {
                bg_app: Color32::from_rgb(241, 244, 249),
                bg_panel: Color32::from_rgb(250, 251, 254),
                bg_elevated: Color32::from_rgb(233, 238, 246),
                bg_canvas_well: Color32::from_rgb(222, 228, 238),
                border_subtle: Color32::from_rgb(205, 213, 226),
                border_strong: Color32::from_rgb(164, 176, 196),
                text_primary: Color32::from_rgb(20, 25, 35),
                text_secondary: Color32::from_rgb(66, 76, 94),
                text_muted: Color32::from_rgb(108, 118, 138),
                accent_primary: Color32::from_rgb(29, 108, 224),
                accent_preview: Color32::from_rgb(12, 133, 153),
                accent_program: Color32::from_rgb(217, 45, 32),
                rec_red: Color32::from_rgb(220, 38, 38),
                pause_amber: Color32::from_rgb(217, 119, 6),
                meter_ok: Color32::from_rgb(22, 163, 74),
                meter_warn: Color32::from_rgb(202, 138, 4),
                meter_clip: Color32::from_rgb(220, 38, 38),
            },
        }
    }
}

pub fn apply_studio_theme(ctx: &egui::Context, mode: ThemeMode) {
    let p = ThemePalette::for_mode(mode);
    let mut style = Style::default();

    // Clear, balanced studio typography with Rubik Regular (weight 400) and Medium (weight 500)
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(14.8, crate::ui::icons::rubik_medium_family()),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(13.5, crate::ui::icons::rubik_family()),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(13.0, crate::ui::icons::rubik_family()),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(11.8, crate::ui::icons::rubik_family()),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.8, egui::FontFamily::Monospace),
    );

    // Spacing with comfortable window margins for clean modal breathing room
    style.spacing.item_spacing = egui::vec2(6.0, 5.0);
    style.spacing.button_padding = egui::vec2(7.0, 3.5);
    style.spacing.menu_margin = Margin::symmetric(6, 5);
    style.spacing.combo_width = 130.0;
    style.spacing.window_margin = Margin {
        left: 14,
        right: 14,
        top: 10,
        bottom: 12,
    };
    style.spacing.indent = 12.0;
    style.spacing.interact_size = egui::vec2(28.0, 22.0);

    // Disable text selection on static labels so the app feels 100% native desktop (OBS/Win32)
    style.interaction.selectable_labels = false;
    style.interaction.multi_widget_text_select = false;

    let mut visuals = match mode {
        ThemeMode::Dark => Visuals::dark(),
        ThemeMode::Light => Visuals::light(),
    };

    visuals.panel_fill = p.bg_panel;
    visuals.window_fill = p.bg_panel;
    visuals.window_highlight_topmost = true;
    visuals.extreme_bg_color = p.bg_canvas_well;
    visuals.faint_bg_color = p.bg_elevated;
    visuals.override_text_color = Some(p.text_primary);

    visuals.window_stroke = Stroke::new(1.0_f32, p.border_strong);
    visuals.selection.bg_fill = match mode {
        ThemeMode::Dark => Color32::from_rgb(38, 56, 88),
        ThemeMode::Light => Color32::from_rgb(212, 228, 252),
    };
    visuals.selection.stroke = Stroke::new(1.0_f32, p.accent_primary);

    let radius = CornerRadius::same(5);
    visuals.clip_rect_margin = 0.0;
    visuals.menu_corner_radius = CornerRadius::same(6);
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: if mode == ThemeMode::Dark {
            Color32::from_black_alpha(160)
        } else {
            Color32::from_black_alpha(65)
        },
    };
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: if mode == ThemeMode::Dark {
            Color32::from_black_alpha(180)
        } else {
            Color32::from_black_alpha(75)
        },
    };

    visuals.widgets.noninteractive.bg_fill = p.bg_panel;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border_subtle);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.noninteractive.corner_radius = radius;
    visuals.widgets.noninteractive.expansion = 0.0;

    // 3D Tactile Styling for all standard egui buttons and ComboBox dropdowns:
    // Solid background fills (no transparent washouts), defined border bevels, and sunken feedback
    visuals.widgets.inactive.bg_fill = match mode {
        ThemeMode::Dark => Color32::from_rgb(28, 32, 42),
        ThemeMode::Light => Color32::from_rgb(234, 239, 248),
    };
    visuals.widgets.inactive.weak_bg_fill = match mode {
        ThemeMode::Dark => Color32::from_rgb(26, 30, 40),
        ThemeMode::Light => Color32::from_rgb(236, 241, 250),
    };
    visuals.widgets.inactive.bg_stroke = Stroke::new(
        1.0_f32,
        match mode {
            ThemeMode::Dark => Color32::from_rgb(52, 58, 74),
            ThemeMode::Light => Color32::from_rgb(175, 186, 204),
        },
    );
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.inactive.corner_radius = radius;
    visuals.widgets.inactive.expansion = 0.0;

    visuals.widgets.hovered.bg_fill = match mode {
        ThemeMode::Dark => Color32::from_rgb(38, 44, 58),
        ThemeMode::Light => Color32::from_rgb(244, 248, 254),
    };
    visuals.widgets.hovered.weak_bg_fill = match mode {
        ThemeMode::Dark => Color32::from_rgb(36, 42, 54),
        ThemeMode::Light => Color32::from_rgb(244, 248, 254),
    };
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent_primary);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.hovered.corner_radius = radius;
    visuals.widgets.hovered.expansion = 0.0;

    visuals.widgets.active.bg_fill = match mode {
        ThemeMode::Dark => Color32::from_rgb(22, 26, 34),
        ThemeMode::Light => Color32::from_rgb(214, 220, 232),
    };
    visuals.widgets.active.weak_bg_fill = match mode {
        ThemeMode::Dark => Color32::from_rgb(20, 24, 32),
        ThemeMode::Light => Color32::from_rgb(212, 218, 230),
    };
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, p.accent_primary);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, p.accent_primary);
    visuals.widgets.active.corner_radius = radius;
    visuals.widgets.active.expansion = 0.0;

    // Dropdown open / expanded state
    visuals.widgets.open.bg_fill = match mode {
        ThemeMode::Dark => Color32::from_rgb(34, 40, 52),
        ThemeMode::Light => Color32::from_rgb(224, 230, 242),
    };
    visuals.widgets.open.weak_bg_fill = match mode {
        ThemeMode::Dark => Color32::from_rgb(34, 40, 52),
        ThemeMode::Light => Color32::from_rgb(224, 230, 242),
    };
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, p.accent_primary);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.open.corner_radius = radius;
    visuals.widgets.open.expansion = 0.0;

    style.visuals = visuals;
    ctx.set_style(style);
}

