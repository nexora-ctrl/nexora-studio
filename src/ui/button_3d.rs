use egui::{
    Color32, CursorIcon, Response, Sense, Stroke, Ui, Vec2, Widget, WidgetText,
};

use crate::settings::ThemeMode;
use crate::ui::theme::ThemePalette;

/// Visual variants for action buttons.
///
/// The name is kept for source compatibility across the codebase; there is no longer any 3D
/// geometry behind it. Each variant only selects a fill / border / text colour triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Button3DVariant {
    #[default]
    Neutral, // Standard studio keycap (Cancel, Close, Browse, Reset, Tools)
    Primary, // Accent blue (Add Source, Apply, OK, Save, Transition)
    Danger,  // Broadcast red (Start Recording, Stop Recording, Delete)
    Warning, // Amber (Pause, Resume, Alert)
    Success, // Studio green (Live, Connect, Update Available)
}

/// Standard flat button widget for NEXORA Studio.
///
/// Renders using the exact same widget states as every other egui control in the app
/// (`visuals.widgets.inactive / hovered / active`), so buttons line up with ComboBoxes,
/// text fields and DragValues without any bespoke bevel, drop shadow or press displacement.
pub struct Button3D {
    text: WidgetText,
    icon: Option<String>,
    variant: Button3DVariant,
    min_size: Vec2,
    corner_radius: f32,
    selected: bool,
    enabled: bool,
    tooltip: Option<String>,
}

impl Button3D {
    /// Creates a new button with neutral studio styling.
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            icon: None,
            variant: Button3DVariant::Neutral,
            min_size: Vec2::new(0.0, 24.0),
            corner_radius: 5.0,
            selected: false,
            enabled: true,
            tooltip: None,
        }
    }

    /// Creates a primary accent (blue) button.
    pub fn primary(text: impl Into<WidgetText>) -> Self {
        Self::new(text).variant(Button3DVariant::Primary)
    }

    /// Creates a danger (red broadcast) button.
    pub fn danger(text: impl Into<WidgetText>) -> Self {
        Self::new(text).variant(Button3DVariant::Danger)
    }

    /// Creates a warning (amber) button.
    pub fn warning(text: impl Into<WidgetText>) -> Self {
        Self::new(text).variant(Button3DVariant::Warning)
    }

    /// Creates a success (green) button.
    pub fn success(text: impl Into<WidgetText>) -> Self {
        Self::new(text).variant(Button3DVariant::Success)
    }

    /// Sets the visual variant of the button.
    pub fn variant(mut self, variant: Button3DVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Prepends an icon character (e.g. from `icons::*`) before the label.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets the minimum size of the button (e.g. `Vec2::new(120.0, 26.0)`).
    pub fn min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    /// Sets custom corner radius (default: 5.0).
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Sets whether the button is currently in a toggled / selected active state.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Sets whether the button is enabled and interactable.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Attaches a hover tooltip to the button.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

impl Widget for Button3D {
    fn ui(self, ui: &mut Ui) -> Response {
        let is_dark = ui.visuals().dark_mode;
        let mode = if is_dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        };
        let p = ThemePalette::for_mode(mode);

        let enabled = self.enabled && ui.is_enabled();
        let radius = egui::CornerRadius::same(self.corner_radius.round() as u8);

        // Compose the display string once; the icon is a glyph from the Tabler font so it
        // belongs on the same baseline as the label.
        let text_str = self.text.text().to_string();
        let display = match (&self.icon, text_str.is_empty()) {
            (Some(ic), false) => format!("{}  {}", ic, text_str),
            (Some(ic), true) => ic.clone(),
            (None, _) => text_str,
        };

        let font_id = egui::TextStyle::Button.resolve(ui.style());

        // Colored variants carry a solid accent face, so they need white text to stay legible.
        let is_colored = !matches!(self.variant, Button3DVariant::Neutral);

        // Measure from the laid-out text first, so the button can be sized before it is
        // allocated. The colour used here is irrelevant — only `galley.size()` is read — so a
        // throwaway transparent galley avoids baking a colour into the measurement pass.
        let galley = ui
            .painter()
            .layout_no_wrap(display.clone(), font_id.clone(), Color32::TRANSPARENT);

        let padding = ui.spacing().button_padding;
        let content_size = Vec2::new(
            galley.size().x + padding.x * 2.0,
            galley.size().y + padding.y * 2.0,
        );

        // Clamp to the available width so a long label can never overflow its parent container.
        let avail_w = ui.available_width();
        let mut target_w = content_size.x.max(self.min_size.x);
        if target_w > avail_w && avail_w > 10.0 {
            target_w = avail_w;
        }
        let desired_size = Vec2::new(
            target_w,
            content_size.y.max(self.min_size.y),
        );

        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };

        let (rect, mut response) = ui.allocate_exact_size(desired_size, sense);

        if let Some(tip) = self.tooltip {
            response = response.on_hover_text(tip);
        }

        if !ui.is_rect_visible(rect) {
            return response;
        }

        // Re-resolve interaction using the real response now that it exists.
        let visuals = ui.style().interact_selectable(&response, self.selected);

        let hovered = enabled && response.hovered();
        if hovered {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }

        // --- Colour resolution: standard widget palette, with accent overrides per variant. ---
        //
        // Hover feedback must move *away* from the surrounding surface in both themes: in light
        // mode that means darkening, in dark mode lightening. Reusing a single `lighten()` for
        // both (as before) washed coloured buttons out on hover in light mode.
        let (fill, stroke_col, text_col) = if !enabled {
            if is_colored {
                // Keep the variant hue recognisable while clearly reading as inert.
                let face = variant_face(self.variant, &p, is_dark, true, false);
                (face, variant_border(self.variant, &p, is_dark, false), p.text_muted)
            } else {
                (
                    visuals.weak_bg_fill,
                    visuals.bg_stroke.color,
                    p.text_muted,
                )
            }
        } else if self.selected {
            if is_colored {
                let face = variant_face(self.variant, &p, is_dark, false, hovered);
                (face, variant_border(self.variant, &p, is_dark, hovered), Color32::WHITE)
            } else {
                (
                    p.accent_primary,
                    shade(p.accent_primary, 22.0),
                    Color32::WHITE,
                )
            }
        } else if is_colored {
            let face = variant_face(self.variant, &p, is_dark, false, hovered);
            (face, variant_border(self.variant, &p, is_dark, hovered), Color32::WHITE)
        } else {
            // Neutral buttons follow the standard widget palette. Light mode previously reused
            // `visuals.fg_stroke.color` here, which routes through `override_text_color` and made
            // the label wash out against the near-white panel, so every value is now resolved
            // from the palette directly.
            let fill = if !is_dark && !hovered {
                visuals.weak_bg_fill
            } else {
                visuals.bg_fill
            };
            (fill, visuals.bg_stroke.color, p.text_primary)
        };

        let painter = ui.painter();
        painter.rect(
            rect,
            radius,
            fill,
            Stroke::new(1.0_f32, stroke_col),
            egui::StrokeKind::Inside,
        );

        // Centered icon + label. Re-layout with the resolved `text_col`: `painter.galley`'s
        // colour argument is only a fallback, so a galley that already carries a baked-in colour
        // would silently ignore it and render white text on a light-mode button face.
        let galley = ui
            .painter()
            .layout_no_wrap(display, font_id, text_col);
        let text_pos = rect.center() - galley.size() * 0.5;
        painter.galley(text_pos, galley, text_col);

        response
    }
}

/// Base accent colour for a coloured variant.
fn variant_base(variant: Button3DVariant, p: &ThemePalette) -> Color32 {
    match variant {
        Button3DVariant::Primary => p.accent_primary,
        Button3DVariant::Danger => p.rec_red,
        Button3DVariant::Warning => p.pause_amber,
        Button3DVariant::Success => p.meter_ok,
        Button3DVariant::Neutral => p.accent_primary,
    }
}

/// Moves a colour toward black by `amount` percent (0..=100).
fn shade(c: Color32, amount: f32) -> Color32 {
    let k = (1.0 - amount / 100.0).clamp(0.0, 1.0);
    Color32::from_rgb(
        (c.r() as f32 * k).round() as u8,
        (c.g() as f32 * k).round() as u8,
        (c.b() as f32 * k).round() as u8,
    )
}

/// Moves a colour toward white by `amount` percent (0..=100).
fn tint(c: Color32, amount: f32) -> Color32 {
    let k = amount / 100.0;
    let mix = |v: u8| (v as f32 + (255.0 - v as f32) * k).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgb(mix(c.r()), mix(c.g()), mix(c.b()))
}

/// Resolved face colour for a coloured variant.
///
/// In light mode the raw palette hues sit on a near-white panel, so a straight solid fill reads
/// as a flat pastel block. Hover therefore darkens the face (increasing contrast against the
/// panel) instead of lightening it, and the disabled face is desaturated well below the enabled
/// one so an inert button can never be mistaken for a live one.
fn variant_face(
    variant: Button3DVariant,
    p: &ThemePalette,
    is_dark: bool,
    disabled: bool,
    hovered: bool,
) -> Color32 {
    let base = variant_base(variant, p);
    if disabled {
        return base.gamma_multiply(if is_dark { 0.28 } else { 0.42 });
    }
    match (is_dark, hovered) {
        (false, true) => shade(base, 18.0),
        (false, false) => base,
        (true, true) => tint(base, 18.0),
        (true, false) => base,
    }
}

/// Border colour for a coloured variant.
///
/// Always distinct from the face so the button keeps a crisp edge against the panel background;
/// previously the border was set to the face colour itself, which erased the outline entirely.
fn variant_border(
    variant: Button3DVariant,
    p: &ThemePalette,
    is_dark: bool,
    hovered: bool,
) -> Color32 {
    let base = variant_base(variant, p);
    match (is_dark, hovered) {
        (false, true) => shade(base, 34.0),
        (false, false) => shade(base, 22.0),
        (true, true) => tint(base, 42.0),
        (true, false) => tint(base, 20.0),
    }
}

/// Extension trait for `egui::Ui` providing ergonomic shorthand methods for action buttons.
pub trait Ui3DExt {
    fn btn_3d(&mut self, text: impl Into<WidgetText>) -> Response;
    fn btn_3d_primary(&mut self, text: impl Into<WidgetText>) -> Response;
    fn btn_3d_danger(&mut self, text: impl Into<WidgetText>) -> Response;
    fn btn_3d_with_icon(&mut self, icon: &str, text: impl Into<WidgetText>, variant: Button3DVariant) -> Response;
}

impl Ui3DExt for Ui {
    fn btn_3d(&mut self, text: impl Into<WidgetText>) -> Response {
        self.add(Button3D::new(text))
    }

    fn btn_3d_primary(&mut self, text: impl Into<WidgetText>) -> Response {
        self.add(Button3D::primary(text))
    }

    fn btn_3d_danger(&mut self, text: impl Into<WidgetText>) -> Response {
        self.add(Button3D::danger(text))
    }

    fn btn_3d_with_icon(&mut self, icon: &str, text: impl Into<WidgetText>, variant: Button3DVariant) -> Response {
        self.add(Button3D::new(text).icon(icon).variant(variant))
    }
}
