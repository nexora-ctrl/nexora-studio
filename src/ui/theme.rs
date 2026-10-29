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

    // Compact, slender studio typography with Rubik Light (clean, modern, balanced)
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(13.8, crate::ui::icons::rubik_family()),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(12.5, crate::ui::icons::rubik_light_family()),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(12.0, crate::ui::icons::rubik_light_family()),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(11.0, crate::ui::icons::rubik_light_family()),
    );
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.0, egui::FontFamily::Monospace),
    );

    // Compact spacing for 1366x768 and 1920x1080 efficiency
    style.spacing.item_spacing = egui::vec2(6.0, 5.0);
    style.spacing.button_padding = egui::vec2(7.0, 3.5);
    style.spacing.window_margin = Margin::same(8);
    style.spacing.indent = 12.0;
    style.spacing.interact_size = egui::vec2(28.0, 22.0);

    let mut visuals = match mode {
        ThemeMode::Dark => Visuals::dark(),
        ThemeMode::Light => Visuals::light(),
    };

    visuals.panel_fill = p.bg_panel;
    visuals.window_fill = p.bg_panel;
    visuals.extreme_bg_color = p.bg_canvas_well;
    visuals.faint_bg_color = p.bg_elevated;
    visuals.override_text_color = Some(p.text_primary);

    visuals.window_stroke = Stroke::new(1.0_f32, p.border_subtle);
    visuals.selection.bg_fill = p.accent_primary.gamma_multiply(0.28);
    visuals.selection.stroke = Stroke::new(1.0_f32, p.accent_primary);

    let radius = CornerRadius::same(4);
    visuals.clip_rect_margin = 0.0;

    visuals.widgets.noninteractive.bg_fill = p.bg_panel;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.border_subtle);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.noninteractive.corner_radius = radius;
    visuals.widgets.noninteractive.expansion = 0.0;

    visuals.widgets.inactive.bg_fill = p.bg_elevated;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.border_subtle);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.inactive.corner_radius = radius;
    visuals.widgets.inactive.expansion = 0.0;

    visuals.widgets.hovered.bg_fill = p.accent_primary.gamma_multiply(0.22);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent_primary);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.hovered.corner_radius = radius;
    visuals.widgets.hovered.expansion = 0.0;

    visuals.widgets.active.bg_fill = p.accent_primary.gamma_multiply(0.35);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, p.accent_primary);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.active.corner_radius = radius;
    visuals.widgets.active.expansion = 0.0;

    visuals.widgets.open.bg_fill = p.accent_primary.gamma_multiply(0.22);
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, p.accent_primary);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.open.corner_radius = radius;
    visuals.widgets.open.expansion = 0.0;

    style.visuals = visuals;
    ctx.set_style(style);
}

