use egui::{
    Color32, CornerRadius, CursorIcon, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2,
    Widget, WidgetText,
};

use crate::settings::ThemeMode;
use crate::ui::theme::ThemePalette;

/// Visual variants for 3D tactile buttons
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Button3DVariant {
    #[default]
    Neutral, // Sleek studio dark keycap (Cancel, Close, Browse, Reset, Tools)
    Primary, // Neon Cyan/Blue Accent (Add Source, Apply, OK, Save, Transition)
    Danger,  // Broadcast Red (Start Recording, Stop Recording, Delete)
    Warning, // Amber/Gold (Pause, Resume, Alert)
    Success, // Studio Green (Live, Connect, Update Available)
}

/// Universal 3D Tactile Button Widget for NEXORA Studio.
///
/// Features:
/// - Physical top-edge specular highlight simulating overhead studio lighting.
/// - Realistic bottom drop shadow / base rim giving tangible elevation.
/// - 1.2px tactile press-down displacement when clicked (sunken active state).
/// - 100% Pure White text on colored action buttons (Danger, Primary, Warning, Success).
/// - Automatic width clamping so buttons never overflow their parent container.
/// - Full disabled state support.
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
    /// Creates a new 3D button with neutral studio styling.
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            icon: None,
            variant: Button3DVariant::Neutral,
            min_size: Vec2::new(0.0, 24.0),
            corner_radius: 4.5,
            selected: false,
            enabled: true,
            tooltip: None,
        }
    }

    /// Creates a primary accent (blue) 3D button.
    pub fn primary(text: impl Into<WidgetText>) -> Self {
        Self::new(text).variant(Button3DVariant::Primary)
    }

    /// Creates a danger (red broadcast) 3D button.
    pub fn danger(text: impl Into<WidgetText>) -> Self {
        Self::new(text).variant(Button3DVariant::Danger)
    }

    /// Creates a warning (amber) 3D button.
    pub fn warning(text: impl Into<WidgetText>) -> Self {
        Self::new(text).variant(Button3DVariant::Warning)
    }

    /// Creates a success (green) 3D button.
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

    /// Sets custom corner radius (default: 4.5).
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

        let style = ui.style();
        let font_id = egui::TextStyle::Button.resolve(style);

        // ALWAYS Pure Crisp White (#FFFFFF) text for colored buttons (Red Danger, Blue Primary, Amber Warning, Green Success)
        let text_color = if !enabled {
            p.text_muted
        } else {
            match self.variant {
                Button3DVariant::Neutral => {
                    if self.selected {
                        p.accent_primary
                    } else {
                        p.text_primary
                    }
                }
                Button3DVariant::Primary
                | Button3DVariant::Danger
                | Button3DVariant::Warning
                | Button3DVariant::Success => Color32::WHITE,
            }
        };

        // Layout text directly with explicit text_color so Light Mode text_color is never accidentally dark
        let text_str = self.text.text().to_string();
        let label_galley = if text_str.is_empty() {
            ui.painter().layout_no_wrap(String::new(), font_id.clone(), text_color)
        } else {
            ui.painter().layout_no_wrap(text_str, font_id.clone(), text_color)
        };

        let icon_galley = self.icon.as_ref().map(|ic| {
            let icon_font = FontId::proportional(font_id.size * 1.15);
            ui.painter().layout_no_wrap(ic.clone(), icon_font, text_color)
        });

        // Compute size with precise padding
        let has_text = !self.text.text().is_empty();

        let mut content_width = label_galley.size().x;
        let mut content_height = label_galley.size().y;

        if let Some(ref ig) = icon_galley {
            if has_text {
                content_width += ig.size().x + 5.0;
            } else {
                content_width = ig.size().x;
            }
            content_height = content_height.max(ig.size().y);
        }

        let padding_x = if has_text { 9.0_f32 } else { 4.0_f32 };
        let padding_y = 4.0_f32;

        let avail_w = ui.available_width();
        let natural_w = content_width + padding_x * 2.0;
        let mut target_w = natural_w.max(self.min_size.x);

        // Strictly clamp button width to available container width so it never overflows!
        if target_w > avail_w && avail_w > 10.0 {
            target_w = avail_w;
        }

        let desired_size = Vec2::new(
            target_w,
            (content_height + padding_y * 2.0).max(self.min_size.y),
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

        let hovered = enabled && response.hovered();
        let pressed = enabled && (response.is_pointer_button_down_on() || self.selected);

        if hovered {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }

        let cr = self.corner_radius;

        // Resolve 3D Palette Colors
        let (base_fill, top_highlight, bottom_rim, border_stroke_col) = match self.variant {
            Button3DVariant::Neutral => {
                if !enabled {
                    let fill = if is_dark { Color32::from_rgb(22, 25, 32) } else { Color32::from_rgb(238, 240, 244) };
                    (fill, Color32::TRANSPARENT, Color32::TRANSPARENT, p.border_subtle)
                } else if is_dark {
                    let fill = if pressed {
                        Color32::from_rgb(24, 28, 36)
                    } else if hovered {
                        Color32::from_rgb(38, 44, 56)
                    } else {
                        Color32::from_rgb(30, 34, 44)
                    };
                    let top_hi = if pressed {
                        Color32::from_black_alpha(75)
                    } else if hovered {
                        Color32::from_white_alpha(55)
                    } else {
                        Color32::from_white_alpha(32)
                    };
                    let bot_rim = if pressed {
                        Color32::from_white_alpha(20)
                    } else {
                        Color32::from_black_alpha(110)
                    };
                    let border = if hovered {
                        p.accent_primary.gamma_multiply(0.75)
                    } else {
                        p.border_subtle
                    };
                    (fill, top_hi, bot_rim, border)
                } else {
                    let fill = if pressed {
                        Color32::from_rgb(218, 224, 234)
                    } else if hovered {
                        Color32::from_rgb(244, 248, 254)
                    } else {
                        Color32::from_rgb(232, 237, 246)
                    };
                    let top_hi = if pressed {
                        Color32::from_black_alpha(45)
                    } else {
                        Color32::from_white_alpha(220)
                    };
                    let bot_rim = Color32::from_black_alpha(65);
                    let border = if hovered {
                        p.accent_primary
                    } else {
                        p.border_strong
                    };
                    (fill, top_hi, bot_rim, border)
                }
            }
            Button3DVariant::Primary => {
                let fill = if !enabled {
                    p.accent_primary.gamma_multiply(0.35)
                } else if pressed {
                    Color32::from_rgb(20, 85, 185)
                } else if hovered {
                    Color32::from_rgb(45, 130, 245)
                } else {
                    p.accent_primary
                };
                let top_hi = if pressed {
                    Color32::from_black_alpha(70)
                } else {
                    Color32::from_white_alpha(90)
                };
                let bot_rim = Color32::from_rgb(15, 60, 140);
                let border = if hovered {
                    Color32::from_rgb(120, 185, 255)
                } else {
                    Color32::from_rgb(25, 95, 205)
                };
                (fill, top_hi, bot_rim, border)
            }
            Button3DVariant::Danger => {
                let fill = if !enabled {
                    p.rec_red.gamma_multiply(0.35)
                } else if pressed {
                    Color32::from_rgb(180, 25, 25)
                } else if hovered {
                    Color32::from_rgb(245, 65, 65)
                } else {
                    p.rec_red
                };
                let top_hi = if pressed {
                    Color32::from_black_alpha(70)
                } else {
                    Color32::from_white_alpha(85)
                };
                let bot_rim = Color32::from_rgb(125, 15, 15);
                let border = if hovered {
                    Color32::from_rgb(255, 115, 115)
                } else {
                    Color32::from_rgb(175, 30, 30)
                };
                (fill, top_hi, bot_rim, border)
            }
            Button3DVariant::Warning => {
                let fill = if !enabled {
                    p.pause_amber.gamma_multiply(0.35)
                } else if pressed {
                    Color32::from_rgb(190, 115, 5)
                } else if hovered {
                    Color32::from_rgb(250, 175, 25)
                } else {
                    p.pause_amber
                };
                let top_hi = if pressed {
                    Color32::from_black_alpha(70)
                } else {
                    Color32::from_white_alpha(90)
                };
                let bot_rim = Color32::from_rgb(135, 75, 5);
                let border = if hovered {
                    Color32::from_rgb(255, 200, 75)
                } else {
                    Color32::from_rgb(185, 110, 8)
                };
                (fill, top_hi, bot_rim, border)
            }
            Button3DVariant::Success => {
                let fill = if !enabled {
                    p.meter_ok.gamma_multiply(0.35)
                } else if pressed {
                    Color32::from_rgb(18, 130, 60)
                } else if hovered {
                    Color32::from_rgb(42, 210, 105)
                } else {
                    p.meter_ok
                };
                let top_hi = if pressed {
                    Color32::from_black_alpha(70)
                } else {
                    Color32::from_white_alpha(85)
                };
                let bot_rim = Color32::from_rgb(12, 95, 42);
                let border = if hovered {
                    Color32::from_rgb(90, 235, 145)
                } else {
                    Color32::from_rgb(20, 145, 68)
                };
                (fill, top_hi, bot_rim, border)
            }
        };

        let painter = ui.painter();

        // 1. Bottom 3D Drop Shadow / Extrusion (only when unpressed and enabled)
        if enabled && !pressed {
            let shadow_h = 1.6_f32;
            let shadow_rect = Rect::from_min_max(
                Pos2::new(rect.min.x + 0.5, rect.max.y - shadow_h),
                Pos2::new(rect.max.x - 0.5, rect.max.y + shadow_h),
            );
            let shadow_color = if is_dark {
                Color32::from_black_alpha(110)
            } else {
                Color32::from_black_alpha(50)
            };
            painter.rect_filled(shadow_rect, CornerRadius::same(cr as u8), shadow_color);
        }

        // 2. Button Face Displacement (press down 1.2px when clicked)
        let face_rect = if pressed {
            rect.translate(Vec2::new(0.0, 1.2))
        } else {
            rect
        };

        // 3. Fill & Stroke
        painter.rect(
            face_rect,
            CornerRadius::same(cr as u8),
            base_fill,
            Stroke::new(1.0_f32, border_stroke_col),
            egui::StrokeKind::Inside,
        );

        // 4. Top Specular Bevel Line (Simulating overhead 3D studio light)
        if enabled {
            let bevel_inset_x = (cr * 0.8).clamp(2.0, 6.0);
            let top_y = face_rect.min.y + 1.2;
            painter.line_segment(
                [
                    Pos2::new(face_rect.min.x + bevel_inset_x, top_y),
                    Pos2::new(face_rect.max.x - bevel_inset_x, top_y),
                ],
                Stroke::new(1.0_f32, top_highlight),
            );

            // 5. Bottom Bevel Rim
            if !pressed {
                let bot_y = face_rect.max.y - 1.2;
                painter.line_segment(
                    [
                        Pos2::new(face_rect.min.x + bevel_inset_x, bot_y),
                        Pos2::new(face_rect.max.x - bevel_inset_x, bot_y),
                    ],
                    Stroke::new(1.0_f32, bottom_rim),
                );
            }
        }

        // 6. Draw Content (Icon + Label centered with tactile offset)
        let content_center = face_rect.center();
        let total_w = if let Some(ref ig) = icon_galley {
            if has_text {
                ig.size().x + 5.0 + label_galley.size().x
            } else {
                ig.size().x
            }
        } else {
            label_galley.size().x
        };

        let start_x = content_center.x - total_w * 0.5;

        if let Some(ref ig) = icon_galley {
            let icon_pos = Pos2::new(
                start_x,
                content_center.y - ig.size().y * 0.5,
            );
            painter.galley(icon_pos, ig.clone(), text_color);

            if has_text {
                let label_pos = Pos2::new(
                    start_x + ig.size().x + 5.0,
                    content_center.y - label_galley.size().y * 0.5,
                );
                painter.galley(label_pos, label_galley, text_color);
            }
        } else if has_text {
            let label_pos = Pos2::new(
                content_center.x - label_galley.size().x * 0.5,
                content_center.y - label_galley.size().y * 0.5,
            );
            painter.galley(label_pos, label_galley, text_color);
        }

        response
    }
}

/// Extension trait for `egui::Ui` providing ergonomic shorthand methods for 3D buttons.
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
