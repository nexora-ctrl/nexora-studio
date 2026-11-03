use crate::app::NexoraApp;
use crate::audio::AudioChannel;
use crate::recording::RecordingStatus;
use crate::studio::source::{
    SlideTransitionEffect, SourceColorTag, SourceItem, SourceKind, TextAnimationEffect,
    TvBannerStyle, TvTextTemplate,
};
use crate::ui::icons;
use crate::ui::theme::ThemePalette;
use egui::{Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use uuid::Uuid;

pub fn render_bottom_studio_dock(ctx: &egui::Context, app: &mut NexoraApp) {
    let vis = &app.settings.panels;
    if !vis.show_scenes
        && !vis.show_sources
        && !vis.show_audio_mixer
        && !vis.show_recording_controls
    {
        return;
    }

    let p = ThemePalette::for_mode(app.settings.theme);

    let dock_h = app.settings.dock_height.clamp(210.0, 380.0);
    let is_modal_open = app.is_any_modal_open();
    let dock_resp = egui::TopBottomPanel::bottom("nexora_studio_dock")
        .resizable(!is_modal_open)
        .default_height(dock_h)
        .height_range(210.0..=380.0)
        .frame(
            egui::Frame::new()
                .fill(p.bg_app)
                .inner_margin(egui::Margin::symmetric(8, 6))
                .stroke(Stroke::new(1.0_f32, p.border_subtle)),
        )
        .show(ctx, |ui| {
            let total_w = ui.available_width();
            let total_h = ui.available_height();
            let gap = 6.0;

            let visible = [
                app.settings.panels.show_scenes,
                app.settings.panels.show_sources,
                app.settings.panels.show_audio_mixer,
                app.settings.panels.show_recording_controls,
            ];
            let active_count = visible.into_iter().filter(|b| *b).count().max(1);

            let usable_w = (total_w - gap * (active_count.saturating_sub(1) as f32)).max(240.0);
            let card_h = total_h.floor().max(160.0);

            // Distribute the width by the user-adjustable weights instead of splitting equally.
            // Weights are normalized over the visible panels only, so hiding a panel gives its
            // share to the others rather than leaving a hole in the dock.
            let col_widths = resolve_dock_column_widths(
                &mut app.settings.dock_panel_widths,
                visible,
                usable_w,
            );

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;

                let mut drawn = 0usize;

                // The splitter between two cards is drawn by the *left* card, so each card only
                // needs to know the width of its right neighbour.
                let draw_panel = |ui: &mut egui::Ui,
                                  app: &mut NexoraApp,
                                  panel_idx: usize,
                                  right_panel_idx: Option<usize>,
                                  vis_idx: usize| {
                    let size = Vec2::new(col_widths[vis_idx], card_h);
                    let rect = match panel_idx {
                        0 => render_scenes_panel(ui, app, size),
                        1 => render_sources_panel(ui, app, size),
                        2 => render_audio_mixer_panel(ui, app, size),
                        _ => render_recording_controls_panel(ui, app, size),
                    };
                    // The splitter redistributes width between `panel_idx` and the next *visible*
                    // panel — a hidden panel in between must keep its stored weight untouched.
                    if let (Some(right_idx), Some(rect)) = (right_panel_idx, rect) {
                        let right_w = col_widths[vis_idx + 1];
                        draw_dock_column_splitter(ui, ctx, app, rect, right_w, panel_idx, right_idx);
                    }
                };

                let visible_idx: Vec<usize> = (0..visible.len()).filter(|i| visible[*i]).collect();
                for (vis_idx, panel_idx) in visible_idx.iter().enumerate() {
                    let right_panel_idx = visible_idx.get(vis_idx + 1).copied();
                    draw_panel(ui, app, *panel_idx, right_panel_idx, vis_idx);
                    drawn += 1;
                }
                debug_assert_eq!(drawn, active_count);
            });
        });

    let measured_h = dock_resp.response.rect.height();
    if (measured_h - app.settings.dock_height).abs() > 1.0 {
        app.settings.dock_height = measured_h.clamp(210.0, 380.0);
    }

    // Splitter bar hit-testing & visual feedback between canvas viewports and bottom dock
    // If any modal dialog is currently open, disable splitter interaction and visual indicator completely
    if is_modal_open {
        ctx.data_mut(|d| d.insert_temp(egui::Id::new("is_resizing_dock_splitter"), false));
        return;
    }

    let dock_top_y = dock_resp.response.rect.min.y;
    let splitter_hit_rect = Rect::from_min_max(
        Pos2::new(dock_resp.response.rect.min.x, dock_top_y - 4.0),
        Pos2::new(dock_resp.response.rect.max.x, dock_top_y + 4.0),
    );

    let is_resizing_id = egui::Id::new("is_resizing_dock_splitter");
    let mut is_resizing = ctx.data_mut(|d| d.get_temp::<bool>(is_resizing_id).unwrap_or(false));

    let is_hovered = ctx.pointer_hover_pos().map(|p| splitter_hit_rect.contains(p)).unwrap_or(false);
    if is_hovered {
        ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);
        if ctx.input(|i| i.pointer.any_down()) {
            is_resizing = true;
        }
    }

    if is_resizing {
        if ctx.input(|i| i.pointer.any_down()) {
            ctx.set_cursor_icon(egui::CursorIcon::ResizeVertical);
        } else {
            is_resizing = false;
        }
    }
    ctx.data_mut(|d| d.insert_temp(is_resizing_id, is_resizing));

    if is_hovered || is_resizing {
        ctx.request_repaint();
        let highlight_col = if is_resizing {
            p.accent_preview
        } else {
            p.accent_primary.gamma_multiply(0.7)
        };
        ctx.layer_painter(egui::LayerId::background())
            .line_segment(
                [
                    Pos2::new(dock_resp.response.rect.min.x, dock_top_y),
                    Pos2::new(dock_resp.response.rect.max.x, dock_top_y),
                ],
                Stroke::new(1.8_f32, highlight_col),
            );
    }
}

struct DockCardRegions {
    header: Rect,
    body: Rect,
    footer: Rect,
}

impl DockCardRegions {
    fn with_header(&self, ui: &mut egui::Ui, f: impl FnOnce(&mut egui::Ui)) {
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(self.header)
                .layout(Layout::left_to_right(Align::Center)),
        );
        child.set_clip_rect(self.header);
        f(&mut child);
    }

    fn with_body(&self, ui: &mut egui::Ui, f: impl FnOnce(&mut egui::Ui, Vec2)) {
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(self.body)
                .layout(Layout::top_down(Align::Min)),
        );
        child.set_clip_rect(self.body);
        f(&mut child, self.body.size());
    }

    fn with_footer(&self, ui: &mut egui::Ui, f: impl FnOnce(&mut egui::Ui)) {
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(self.footer)
                .layout(Layout::left_to_right(Align::Center)),
        );
        child.set_clip_rect(self.footer);
        child.spacing_mut().item_spacing.x = 4.0;
        child.spacing_mut().interact_size.y = 24.0;
        f(&mut child);
    }
}

/// Paints a dock card and returns both its inner regions and its outer rect.
///
/// The outer rect is handed back so the caller can place a resize splitter on the card's right
/// edge; the regions alone describe the inside of the card and cannot be used for that.
fn paint_and_split_dock_card(
    ui: &mut egui::Ui,
    p: &ThemePalette,
    card_size: Vec2,
) -> (DockCardRegions, Rect) {
    let (card_rect, _) = ui.allocate_exact_size(card_size, Sense::hover());
    let painter = ui.painter();
    painter.rect(
        card_rect,
        CornerRadius::same(4),
        p.bg_panel,
        Stroke::new(1.0_f32, p.border_subtle),
        egui::StrokeKind::Inside,
    );

    let inner = card_rect.shrink(6.0);
    let header_h = 22.0;
    let footer_h = 26.0;

    let header = Rect::from_min_max(inner.min, Pos2::new(inner.max.x, inner.min.y + header_h));
    let top_sep_y = inner.min.y + header_h + 3.0;
    let bot_sep_y = inner.max.y - footer_h - 3.0;
    let body = Rect::from_min_max(
        Pos2::new(inner.min.x, top_sep_y + 4.0),
        Pos2::new(inner.max.x, (bot_sep_y - 4.0).max(top_sep_y + 24.0)),
    );
    let footer = Rect::from_min_max(Pos2::new(inner.min.x, inner.max.y - footer_h), inner.max);

    painter.line_segment(
        [
            Pos2::new(inner.min.x, top_sep_y),
            Pos2::new(inner.max.x, top_sep_y),
        ],
        Stroke::new(1.0_f32, p.border_subtle),
    );
    painter.line_segment(
        [
            Pos2::new(inner.min.x, bot_sep_y),
            Pos2::new(inner.max.x, bot_sep_y),
        ],
        Stroke::new(1.0_f32, p.border_subtle),
    );

    (
        DockCardRegions {
            header,
            body,
            footer,
        },
        card_rect,
    )
}

fn paint_icon_action_btn(
    ui: &mut egui::Ui,
    icon: &str,
    accent_color: Option<Color32>,
    p: &ThemePalette,
    tooltip: &str,
) -> bool {
    paint_icon_action_btn_sized(ui, icon, accent_color, p, tooltip, Vec2::new(24.0, 22.0))
}

/// Same as [`paint_icon_action_btn`], but with an explicit size.
///
/// The button reserves exactly `size` in its parent, so it can be used as one column of a row
/// whose columns have to line up with the columns of the other rows.
fn paint_icon_action_btn_sized(
    ui: &mut egui::Ui,
    icon: &str,
    accent_color: Option<Color32>,
    p: &ThemePalette,
    tooltip: &str,
    size: Vec2,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hovered = resp.hovered();

    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    // Standard widget states — no drop shadow, no press displacement, no specular bevel.
    let visuals = ui.style().interact(&resp);

    let icon_col = accent_color.unwrap_or(if hovered {
        p.text_primary
    } else {
        p.text_secondary
    });

    // Light mode needs the stronger `weak_bg_fill` for the resting face; `bg_fill` there is
    // nearly identical to the panel behind it, which made the icon buttons look disabled.
    let is_dark = ui.visuals().dark_mode;
    let fill = if !is_dark && !hovered {
        ui.visuals().widgets.inactive.weak_bg_fill
    } else {
        visuals.bg_fill
    };

    ui.painter().rect(
        rect,
        CornerRadius::same(5),
        fill,
        Stroke::new(1.0_f32, visuals.bg_stroke.color),
        egui::StrokeKind::Inside,
    );

    let font_id = egui::FontId::proportional(11.5);
    let galley = ui
        .painter()
        .layout_no_wrap(icon.to_string(), font_id, icon_col);
    let pos = Pos2::new(
        rect.center().x - galley.size().x * 0.5,
        rect.center().y - galley.size().y * 0.5,
    );
    ui.painter().galley(pos, galley, icon_col);

    let clicked = resp.clicked();
    if !tooltip.is_empty() {
        resp.on_hover_text(tooltip);
    }
    clicked
}

/// Paints a single line of text inside `rect`, vertically centred.
///
/// The text is laid out by hand instead of through a `Label`, because a label is laid out by the
/// parent `Ui` and claims only as much width as the text happens to need. A column built from
/// labels would therefore be as wide as its longest entry, and the columns next to it would no
/// longer line up. Here the caller reserves the column first and the text is simply drawn into it.
///
/// With `truncate` the text is cut with an ellipsis when it does not fit; without it the text is
/// drawn at its natural width and may stick out of the rect.
fn paint_single_line_text(
    ui: &egui::Ui,
    rect: Rect,
    text: &str,
    font_id: egui::FontId,
    color: Color32,
    align: egui::Align,
    truncate: bool,
) {
    let galley = if truncate {
        let mut job = egui::text::LayoutJob::default();
        job.append(
            text,
            0.0,
            egui::text::TextFormat {
                font_id,
                color,
                ..Default::default()
            },
        );
        job.wrap.max_width = rect.width();
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        job.halign = egui::Align::Min;
        ui.fonts(|f| f.layout_job(job))
    } else {
        ui.painter().layout_no_wrap(text.to_owned(), font_id, color)
    };

    let x = match align {
        egui::Align::Min => rect.left(),
        egui::Align::Center => rect.center().x - galley.size().x * 0.5,
        egui::Align::Max => rect.right() - galley.size().x,
    };
    let pos = Pos2::new(x, rect.center().y - galley.size().y * 0.5);
    ui.painter().galley(pos, galley, color);
}

/// Measures how wide a status pill will be, without drawing it.
///
/// The scene rows need this *before* they place the scene name: the badges have a fixed width, so
/// measuring them first lets the name take exactly the leftover space instead of the badge
/// overrunning it.
fn status_pill_width(ui: &egui::Ui, text: &str) -> f32 {
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(10.0),
        Color32::TRANSPARENT,
    );
    galley.size().x + 6.5 * 2.0
}

fn paint_status_pill(
    ui: &mut egui::Ui,
    text: &str,
    bg_fill: Color32,
    border_col: Color32,
    text_col: Color32,
    tooltip: &str,
) {
    let font_id = egui::FontId::proportional(10.0);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font_id, text_col);
    let pad = Vec2::new(6.5, 2.5);
    let desired = Vec2::new(galley.size().x + pad.x * 2.0, (galley.size().y + pad.y * 2.0).max(18.0));
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(5), bg_fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(5),
        Stroke::new(1.0_f32, border_col),
        egui::StrokeKind::Inside,
    );
    let text_pos = Pos2::new(
        rect.center().x - galley.size().x * 0.5,
        rect.center().y - galley.size().y * 0.5,
    );
    ui.painter().galley(text_pos, galley, text_col);
    if !tooltip.is_empty() {
        resp.on_hover_text(tooltip);
    }
}

/// Minimum width a dock panel may be shrunk to by dragging.
///
/// Below this the card's header controls (the `—` button and the status badges) start to overlap,
/// so the splitter refuses to go further instead of producing an unusable panel.
///
/// This is the fallback for panels without a tailored limit; see [`dock_panel_min_w`].
const DOCK_PANEL_MIN_W: f32 = 150.0;

/// Width a mixer row needs before the numeric dB readout is worth showing.
///
/// Below it the readout is dropped and the fader takes the freed space. The fader's tooltip still
/// reports the exact value, so nothing is actually lost — only the always-on number disappears.
const MIXER_DB_READOUT_MIN_W: f32 = 200.0;

/// Minimum room kept for a scene name in the Scenes list.
///
/// The name is the whole point of a scene row, so the status badges give way before it does.
const SCENE_NAME_MIN_W: f32 = 44.0;

/// Rough advance width of one glyph of the 12 pt scene name, in px.
///
/// The scene name shares its row with the status badges, so it has to be truncated by the space
/// that is actually left over rather than by a character count.
const SCENE_NAME_CHAR_W: f32 = 6.4;

/// Width of the mute button in the compact mixer list.
const MIXER_MUTE_W: f32 = 24.0;

/// Width of the transport button slot in the compact mixer list.
///
/// The slot is reserved in every row, even in the rows of channels that cannot be paused, so that
/// the faders of all rows start at the same x.
const MIXER_TRANSPORT_W: f32 = 24.0;

/// Height of one row in the compact mixer list.
const MIXER_ROW_H: f32 = 20.0;

/// Font size of the channel name in the compact mixer list.
const MIXER_NAME_FONT_SIZE: f32 = 11.0;

/// Font size of the `IN` / `OUT` / `APP` / `PGM` tags in the compact mixer list.
const MIXER_TAG_FONT_SIZE: f32 = 9.5;

/// Font size of the percentage readout in the compact mixer list.
///
/// Only the percentage is shown; the exact dB lives in the fader's tooltip.
const MIXER_READOUT_FONT_SIZE: f32 = 10.5;

/// Width of one column in the OBS-style mixer layout.
const MIXER_COL_W: f32 = 86.0;

/// The lowest dB the OBS-style mixer columns print on their scale.
const MIXER_SCALE_MIN_DB: f32 = -60.0;

/// The dB value of a linear gain, floored at -60 dB like a broadcast meter.
fn mixer_volume_to_db(volume: f32) -> f32 {
    if volume <= 0.0001 {
        MIXER_SCALE_MIN_DB
    } else {
        (20.0 * volume.log10()).clamp(MIXER_SCALE_MIN_DB, 0.0)
    }
}

/// The linear gain of a dB value, with -60 dB standing for silence.
fn mixer_db_to_volume(db: f32) -> f32 {
    if db <= MIXER_SCALE_MIN_DB {
        0.0
    } else {
        10.0_f32.powf(db / 20.0)
    }
}

/// Minimum row width for the compact mixer list to show the `IN` / `OUT` / `PGM` tags.
///
/// Below this the tags are dropped for every row at once, so the faders stay aligned.
const MIXER_TAGS_MIN_ROW_W: f32 = 330.0;

/// Minimum width of the channel name in the compact mixer list.
const MIXER_NAME_MIN_W: f32 = 44.0;

/// Minimum width of the fader in the compact mixer list.
///
/// Below this the fader cannot be grabbed reliably, which is worse than losing a name.
const MIXER_FADER_MIN_W: f32 = 54.0;

/// Per-panel lower bound for the width splitter, indexed like [`resolve_dock_column_widths`].
///
/// A single shared minimum could not be right for all four cards: `Scenes` carries a `LIVE` /
/// `PREVIEW` badge row plus the list/grid toggle, while `Source` now holds only a `+` and a `v`.
/// Holding the wide one at a width its header cannot render, or forcing the narrow one to stay
/// wider than its contents, both look broken. Each limit is the width at which that specific card
/// still renders correctly:
///
/// - `Scenes` (200) — the list/grid toggle, the minimize button and the `Duplicate` / `Rename`
///   footer buttons all fit side by side.
/// - `Source` (150) — the header and the `+` / `v` footer pair, plus enough room for the layer
///   name to stay readable; longer names clip, as they always did.
/// - `Audio Mixer` (240) — one bus row needs `MUTE` + name + fader + dB readout. The dB readout
///   drops out first when the row is squeezed, but the fader itself must stay usable.
/// - `Controls` (200) — the three footer buttons (`Open Folder`, `Library`, `VCam`) in full.
///
/// The four floors sum to 790 px. A narrower window cannot honour them all, and the resolver
/// scales the row down uniformly instead — the panels stay reachable, just cramped.
const DOCK_PANEL_MIN_W_BY_INDEX: [f32; 4] = [200.0, 150.0, 240.0, 200.0];

/// The shrink limit that applies to one dock panel.
///
/// Unknown indices fall back to the generic [`DOCK_PANEL_MIN_W`] rather than panicking, so a future
/// fifth panel degrades gracefully instead of crashing on a missing entry.
fn dock_panel_min_w(panel_idx: usize) -> f32 {
    DOCK_PANEL_MIN_W_BY_INDEX
        .get(panel_idx)
        .copied()
        .unwrap_or(DOCK_PANEL_MIN_W)
}

/// Turns the stored per-panel weights into concrete pixel widths for the visible panels.
///
/// The four weights are normalized over *visible* panels only, so hiding one panel hands its share
/// to the others instead of leaving an empty column.
///
/// The returned widths always sum to exactly `usable_w`. When the minimum-width floor is more than
/// the dock can afford, every panel is scaled down proportionally rather than letting the columns
/// overflow — the cards would otherwise be pushed past the right edge of the window and become
/// unreachable. Panels may therefore end up narrower than `DOCK_PANEL_MIN_W` on a small window,
/// which is the lesser problem: a cramped panel is still usable, an off-screen one is not.
fn resolve_dock_column_widths(
    weights: &mut [f32; 4],
    visible: [bool; 4],
    usable_w: f32,
) -> Vec<f32> {
    // A weight of zero or worse (possible from a hand-edited config file) would collapse a panel
    // to nothing and make it impossible to grab its splitter again.
    for w in weights.iter_mut() {
        if !w.is_finite() || *w <= 0.0 {
            *w = 1.0;
        }
    }

    let visible_idx: Vec<usize> = (0..4).filter(|i| visible[*i]).collect();
    if visible_idx.is_empty() {
        return Vec::new();
    }

    let budget = usable_w.max(1.0);
    let weight_sum: f32 = visible_idx.iter().map(|i| weights[*i]).sum();

    // Each panel clamps to its own floor, so a card whose contents need more room is not squeezed
    // to the same width as one that only holds two small buttons.
    let floors: Vec<f32> = visible_idx.iter().map(|i| dock_panel_min_w(*i)).collect();

    let mut widths: Vec<f32> = visible_idx
        .iter()
        .enumerate()
        .map(|(n, i)| {
            let share = if weight_sum > 0.0 {
                weights[*i] / weight_sum
            } else {
                1.0 / visible_idx.len() as f32
            };
            (budget * share).max(floors[n])
        })
        .collect();

    // Shrink whatever exceeds its own floor, widest first, to absorb the overshoot the
    // minimum-width clamps introduced.
    let mut overflow = widths.iter().sum::<f32>() - budget;
    while overflow > 0.5 {
        let Some(widest) = widths
            .iter()
            .enumerate()
            .filter(|(n, w)| **w > floors[*n])
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i)
        else {
            break;
        };
        let give = overflow.min(widths[widest] - floors[widest]);
        widths[widest] -= give;
        overflow -= give;
    }

    // Still over budget means the floors themselves do not fit (narrow window, four panels). Scale
    // everything down uniformly so the row stays inside the dock instead of overflowing.
    if overflow > 0.5 {
        let total: f32 = widths.iter().sum();
        let scale = budget / total;
        for w in widths.iter_mut() {
            *w *= scale;
        }
    }

    widths
}

/// Draws and handles the resize splitter that sits between two dock panels.
///
/// Dragging transfers width between the card on the left (`rect`) and the one on the right
/// (`right_w`): the pair's combined width is held constant, so one panel growing always means its
/// neighbour shrinking. That keeps the rest of the dock from shifting while a single boundary is
/// dragged, which is what makes the interaction feel predictable.
fn draw_dock_column_splitter(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    app: &mut NexoraApp,
    rect: Rect,
    right_w: f32,
    panel_idx: usize,
    right_panel_idx: usize,
) {
    let p = ThemePalette::for_mode(app.settings.theme);
    let gap_w = ui.spacing().item_spacing.x;
    let hit_rect = Rect::from_min_max(
        Pos2::new(rect.max.x, rect.min.y),
        Pos2::new(rect.max.x + gap_w, rect.max.y),
    );

    let resp = ui.interact(
        hit_rect,
        egui::Id::new(("nexora_dock_col_splitter", panel_idx)),
        Sense::drag(),
    );

    let hovered = resp.hovered() || resp.dragged();
    if hovered {
        ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        ctx.request_repaint();
    }

    let drag_delta = resp.drag_delta().x;
    if resp.dragged() && drag_delta.abs() > 0.0 {
        let left_w = rect.width();
        let total = left_w + right_w;
        // Each side is held to its own floor, so a drag cannot collapse a neighbour to nothing and
        // cannot squeeze a card below the width its contents need. The pair's total stays constant:
        // whatever the left panel gains, the right one gives up.
        let left_min = dock_panel_min_w(panel_idx);
        let right_min = dock_panel_min_w(right_panel_idx);
        // `lo`/`hi` are ordered defensively: when the pair cannot even hold both floors, the
        // remaining space is split rather than panicking on an inverted clamp range.
        let lo = left_min.min(total - right_min);
        let hi = (total - right_min).max(left_min);
        let new_left = (left_w + drag_delta).clamp(lo, hi);
        if right_panel_idx < app.settings.dock_panel_widths.len() {
            let cur_left = app.settings.dock_panel_widths[panel_idx];
            let cur_right = app.settings.dock_panel_widths[right_panel_idx];
            // Convert the pixel sizes back into weights so the split survives a window resize.
            let left_ratio = if total > 0.0 { new_left / total } else { 0.5 };
            let weight_total = cur_left + cur_right;
            if left_ratio > 0.0 && left_ratio < 1.0 {
                app.settings.dock_panel_widths[panel_idx] = weight_total * left_ratio;
                app.settings.dock_panel_widths[right_panel_idx] = weight_total * (1.0 - left_ratio);
            }
        }
    }

    if hovered {
        let col = if resp.dragged() {
            p.accent_preview
        } else {
            p.accent_primary.gamma_multiply(0.7)
        };
        ui.painter().line_segment(
            [
                Pos2::new(hit_rect.center().x, hit_rect.min.y + 4.0),
                Pos2::new(hit_rect.center().x, hit_rect.max.y - 4.0),
            ],
            Stroke::new(1.8_f32, col),
        );
    }
}

/// Returns the card's outer rect so the caller can attach a width splitter to its right edge.
fn render_scenes_panel(ui: &mut egui::Ui, app: &mut NexoraApp, size: Vec2) -> Option<Rect> {
    let p = ThemePalette::for_mode(app.settings.theme);
    let (regions, card_rect) = paint_and_split_dock_card(ui, &p, size);

    regions.with_header(ui, |ui| {
        ui.label(
            RichText::new("SCENES")
                .size(13.5)
                .strong()
                .color(p.text_primary),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("—").small())
                        .min_size(Vec2::new(22.0, 20.0)),
                )
                .on_hover_text("Minimize Scenes panel")
                .clicked()
            {
                app.settings.panels.show_scenes = false;
            }
            ui.add_space(3.0);

            // Toggle Tampilan List vs Grid
            let is_grid = app.settings.scenes_view_mode == crate::settings::ScenesViewMode::Grid;
            let (grid_bg, grid_fg) = if is_grid {
                (p.accent_preview.gamma_multiply(0.35), p.accent_preview)
            } else {
                (Color32::TRANSPARENT, p.text_muted)
            };
            if ui
                .add(
                    egui::Button::new(RichText::new(icons::LAYOUT_GRID).size(13.0).color(grid_fg))
                        .fill(grid_bg)
                        .stroke(if is_grid {
                            Stroke::new(1.0_f32, p.accent_preview)
                        } else {
                            Stroke::NONE
                        })
                        .min_size(Vec2::new(22.0, 20.0)),
                )
                .on_hover_text("Grid View (with Source Thumbnails)")
                .clicked()
            {
                app.settings.scenes_view_mode = crate::settings::ScenesViewMode::Grid;
            }

            let (list_bg, list_fg) = if !is_grid {
                (p.accent_preview.gamma_multiply(0.35), p.accent_preview)
            } else {
                (Color32::TRANSPARENT, p.text_muted)
            };
            if ui
                .add(
                    egui::Button::new(RichText::new(icons::LIST).size(13.0).color(list_fg))
                        .fill(list_bg)
                        .stroke(if !is_grid {
                            Stroke::new(1.0_f32, p.accent_preview)
                        } else {
                            Stroke::NONE
                        })
                        .min_size(Vec2::new(22.0, 20.0)),
                )
                .on_hover_text("List View")
                .clicked()
            {
                app.settings.scenes_view_mode = crate::settings::ScenesViewMode::List;
            }

            ui.add_space(4.0);
            // The legend explains the row badges below, so it is the first thing to go when the
            // header runs out of room — the same information is still on every scene row.
            let legend_needed = status_pill_width(ui, &format!("{} LIVE", icons::PLAYER_RECORD))
                + status_pill_width(ui, &format!("{} PREVIEW", icons::EYE))
                + 4.0;
            if ui.available_width() >= legend_needed {
                paint_status_pill(
                    ui,
                    &format!("{} LIVE", icons::PLAYER_RECORD),
                    Color32::from_rgba_unmultiplied(
                        p.accent_program.r(),
                        p.accent_program.g(),
                        p.accent_program.b(),
                        36,
                    ),
                    p.accent_program,
                    p.accent_program,
                    "Red = Scene currently LIVE on Program output",
                );
                paint_status_pill(
                    ui,
                    &format!("{} PREVIEW", icons::EYE),
                    Color32::from_rgba_unmultiplied(
                        p.accent_preview.r(),
                        p.accent_preview.g(),
                        p.accent_preview.b(),
                        36,
                    ),
                    p.accent_preview,
                    p.accent_preview,
                    "Teal = Scene currently edited in Preview",
                );
            }
        });
    });

    regions.with_body(ui, |ui, body_size| {
        egui::ScrollArea::vertical()
            .id_salt("scenes_scroll")
            .max_height(body_size.y)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut select_preview = None;
                let mut transition_scene_to_live = None;
                let mut rename_scene_target = None;
                let mut duplicate_scene_target = None;
                let mut delete_scene_target = None;
                let mut add_new_scene_requested = false;

                let preview_id = app.studio.preview_scene_id;
                let program_id = app.studio.program_scene_id;
                let total_scenes = app.studio.scenes.len();
                let is_grid_mode =
                    app.settings.scenes_view_mode == crate::settings::ScenesViewMode::Grid;

                if is_grid_mode {
                    // ========================================================
                    // GRID VIEW MODE (Kartu Grid dengan Thumbnail Real-Time)
                    // ========================================================
                    let avail_w = (body_size.x - 8.0).max(100.0);
                    let cols = if avail_w < 230.0 {
                        1
                    } else {
                        ((avail_w + 6.0) / 140.0).floor().max(2.0) as usize
                    };
                    let gap = 6.0;
                    let col_w =
                        ((avail_w - (cols - 1) as f32 * gap) / cols as f32).floor().max(96.0);
                    let thumb_w = (col_w - 8.0).max(80.0);
                    let thumb_h = (thumb_w * 9.0 / 16.0).round().max(45.0);
                    let card_h = thumb_h + 39.0;

                    let time_sec = ui.input(|i| i.time);
                    let egui_ctx = ui.ctx().clone();

                    let scenes_snapshot: Vec<_> = app.studio.scenes.clone();
                    for chunk in scenes_snapshot.chunks(cols) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            for scene in chunk {
                                let is_prv = scene.id == preview_id;
                                let is_pgm = scene.id == program_id;
                                let src_count = scene.sources.len();

                                let (card_rect, card_resp) = ui
                                    .allocate_exact_size(Vec2::new(col_w, card_h), Sense::click());
                                let hovered = card_resp.hovered();

                                // Theme-aware Card Fill & Stroke
                                let (card_fill, border_stroke) = match (is_pgm, is_prv) {
                                    (true, true) => (
                                        Color32::from_rgba_unmultiplied(
                                            p.accent_program.r(),
                                            p.accent_program.g(),
                                            p.accent_program.b(),
                                            if hovered { 40 } else { 26 },
                                        ),
                                        Stroke::new(1.6_f32, p.accent_program),
                                    ),
                                    (true, false) => (
                                        Color32::from_rgba_unmultiplied(
                                            p.accent_program.r(),
                                            p.accent_program.g(),
                                            p.accent_program.b(),
                                            if hovered { 35 } else { 20 },
                                        ),
                                        Stroke::new(1.5_f32, p.accent_program),
                                    ),
                                    (false, true) => (
                                        Color32::from_rgba_unmultiplied(
                                            p.accent_preview.r(),
                                            p.accent_preview.g(),
                                            p.accent_preview.b(),
                                            if hovered { 38 } else { 24 },
                                        ),
                                        Stroke::new(1.5_f32, p.accent_preview),
                                    ),
                                    (false, false) => (
                                        if hovered {
                                            p.bg_elevated
                                        } else {
                                            Color32::from_rgba_unmultiplied(
                                                p.bg_elevated.r(),
                                                p.bg_elevated.g(),
                                                p.bg_elevated.b(),
                                                140,
                                            )
                                        },
                                        Stroke::new(1.0_f32, p.border_subtle),
                                    ),
                                };

                                ui.painter().rect_filled(
                                    card_rect,
                                    CornerRadius::same(6),
                                    card_fill,
                                );

                                // 16:9 Thumbnail canvas rect
                                let thumb_rect = Rect::from_min_size(
                                    Pos2::new(card_rect.min.x + 4.0, card_rect.min.y + 4.0),
                                    Vec2::new(thumb_w, thumb_h),
                                );

                                // Render live source preview of this scene!
                                crate::ui::workspace::render_scene_thumbnail(
                                    ui.painter(),
                                    &egui_ctx,
                                    app,
                                    scene,
                                    thumb_rect,
                                    time_sec,
                                );

                                // Scene Name & Status badges in bottom area of card (keeps 16:9 thumbnail 100% clean and unblocked)
                                let label_y = thumb_rect.max.y + 4.0;
                                let pill_w = 38.0;
                                let pill_h = 12.5;
                                let badge_x = card_rect.max.x - pill_w - 5.0;

                                // Right-aligned status badges stacked vertically (atas-bawah) so text is not obstructed
                                match (is_pgm, is_prv) {
                                    (true, true) => {
                                        // Top badge: LIVE (Program)
                                        let live_rect = Rect::from_min_size(
                                            Pos2::new(badge_x, label_y),
                                            Vec2::new(pill_w, pill_h),
                                        );
                                        ui.painter().rect_filled(
                                            live_rect,
                                            CornerRadius::same(3),
                                            p.accent_program,
                                        );
                                        ui.painter().text(
                                            live_rect.center(),
                                            Align2::CENTER_CENTER,
                                            format!("{} LIVE", icons::PLAYER_RECORD),
                                            FontId::proportional(7.5),
                                            Color32::WHITE,
                                        );

                                        // Bottom badge: PRV (Preview)
                                        let prv_rect = Rect::from_min_size(
                                            Pos2::new(badge_x, label_y + 14.0),
                                            Vec2::new(pill_w, pill_h),
                                        );
                                        ui.painter().rect_filled(
                                            prv_rect,
                                            CornerRadius::same(3),
                                            p.accent_preview,
                                        );
                                        ui.painter().text(
                                            prv_rect.center(),
                                            Align2::CENTER_CENTER,
                                            format!("{} PRV", icons::EYE),
                                            FontId::proportional(7.5),
                                            Color32::WHITE,
                                        );
                                    }
                                    (true, false) => {
                                        let live_rect = Rect::from_min_size(
                                            Pos2::new(badge_x, label_y),
                                            Vec2::new(pill_w, pill_h),
                                        );
                                        ui.painter().rect_filled(
                                            live_rect,
                                            CornerRadius::same(3),
                                            p.accent_program,
                                        );
                                        ui.painter().text(
                                            live_rect.center(),
                                            Align2::CENTER_CENTER,
                                            format!("{} LIVE", icons::PLAYER_RECORD),
                                            FontId::proportional(7.5),
                                            Color32::WHITE,
                                        );
                                    }
                                    (false, true) => {
                                        let prv_rect = Rect::from_min_size(
                                            Pos2::new(badge_x, label_y),
                                            Vec2::new(pill_w, pill_h),
                                        );
                                        ui.painter().rect_filled(
                                            prv_rect,
                                            CornerRadius::same(3),
                                            p.accent_preview,
                                        );
                                        ui.painter().text(
                                            prv_rect.center(),
                                            Align2::CENTER_CENTER,
                                            format!("{} PRV", icons::EYE),
                                            FontId::proportional(7.5),
                                            Color32::WHITE,
                                        );
                                    }
                                    (false, false) => {}
                                }

                                // Left-aligned Scene Title (dynamically fitted so it NEVER overlaps badges)
                                let font_id = FontId::proportional(11.0);
                                let avail_w = if is_pgm || is_prv {
                                    (card_rect.width() - pill_w - 14.0).max(20.0)
                                } else {
                                    (card_rect.width() - 12.0).max(20.0)
                                };

                                let mut display_title = scene.name.clone();
                                let g = ui.painter().layout_no_wrap(display_title.clone(), font_id.clone(), p.text_primary);
                                if g.size().x > avail_w {
                                    let mut truncated = scene.name.clone();
                                    display_title = "...".to_string();
                                    while !truncated.is_empty() {
                                        truncated.pop();
                                        let candidate = format!("{}…", truncated);
                                        let cand_g = ui.painter().layout_no_wrap(candidate.clone(), font_id.clone(), p.text_primary);
                                        if cand_g.size().x <= avail_w {
                                            display_title = candidate;
                                            break;
                                        }
                                    }
                                }

                                ui.painter().text(
                                    Pos2::new(card_rect.min.x + 6.0, label_y),
                                    Align2::LEFT_TOP,
                                    display_title,
                                    font_id,
                                    p.text_primary,
                                );

                                let layer_info = format!("{} layer(s)", src_count);
                                ui.painter().text(
                                    Pos2::new(card_rect.min.x + 6.0, label_y + 14.0),
                                    Align2::LEFT_TOP,
                                    layer_info,
                                    FontId::proportional(9.0),
                                    p.text_secondary,
                                );

                                // Card border stroke
                                ui.painter().rect_stroke(
                                    card_rect,
                                    CornerRadius::same(6),
                                    border_stroke,
                                    egui::StrokeKind::Inside,
                                );

                                let scene_id = scene.id;
                                let scene_name_for_menu = scene.name.clone();
                                if card_resp.double_clicked() {
                                    transition_scene_to_live = Some(scene_id);
                                } else if card_resp.clicked() {
                                    select_preview = Some(scene_id);
                                }

                                card_resp.context_menu(|ui| {
                                    ui.set_min_width(190.0);
                                    if ui
                                        .button(format!(
                                            "{} Transition to LIVE (Program)",
                                            icons::ARROW_RIGHT
                                        ))
                                        .clicked()
                                    {
                                        transition_scene_to_live = Some(scene_id);
                                        ui.close_menu();
                                    }
                                    ui.separator();
                                    if ui
                                        .button(format!("{} Add Scene...", icons::PLUS))
                                        .clicked()
                                    {
                                        add_new_scene_requested = true;
                                        ui.close_menu();
                                    }
                                    if ui
                                        .button(format!("{} Rename Scene...", icons::EDIT))
                                        .clicked()
                                    {
                                        rename_scene_target =
                                            Some((scene_id, scene_name_for_menu.clone()));
                                        ui.close_menu();
                                    }
                                    if ui
                                        .button(format!("{} Duplicate Scene", icons::COPY))
                                        .clicked()
                                    {
                                        duplicate_scene_target = Some(scene_id);
                                        ui.close_menu();
                                    }
                                    if total_scenes > 1
                                        && ui
                                            .button(
                                                RichText::new(format!(
                                                    "{} Delete Scene",
                                                    icons::TRASH
                                                ))
                                                .color(p.rec_red),
                                            )
                                            .clicked()
                                    {
                                        delete_scene_target = Some(scene_id);
                                        ui.close_menu();
                                    }
                                });

                                card_resp.on_hover_text(format!(
                                    "{}\n• Click: Open in Preview\n• Double-Click: Transition to LIVE\n• Right-Click: Scene Options",
                                    scene.name
                                ));
                            }
                        });
                        ui.add_space(gap);
                    }
                } else {
                    // ========================================================
                    // LIST VIEW MODE (Daftar Baris Full-Width Presisi)
                    // ========================================================
                    let row_w = (body_size.x - 4.0).max(150.0);
                    ui.spacing_mut().item_spacing.y = 5.0;

                    for (idx, scene) in app.studio.scenes.iter().enumerate() {
                        let is_prv = scene.id == preview_id;
                        let is_pgm = scene.id == program_id;
                        let src_count = scene.sources.len();
                        let active_src_count =
                            scene.sources.iter().filter(|s| s.visible).count();

                        let row_h = 38.0;
                        let (row_rect, row_resp) =
                            ui.allocate_exact_size(Vec2::new(row_w, row_h), Sense::click());

                        let hovered = row_resp.hovered();

                        // Card Fill & Border based on LIVE / PREVIEW / STANDBY state
                        let (card_fill, border_stroke) = match (is_pgm, is_prv) {
                            (true, true) => (
                                Color32::from_rgba_unmultiplied(
                                    p.accent_program.r(),
                                    p.accent_program.g(),
                                    p.accent_program.b(),
                                    if hovered { 38 } else { 26 },
                                ),
                                Stroke::new(1.5_f32, p.accent_program),
                            ),
                            (true, false) => (
                                Color32::from_rgba_unmultiplied(
                                    p.accent_program.r(),
                                    p.accent_program.g(),
                                    p.accent_program.b(),
                                    if hovered { 32 } else { 20 },
                                ),
                                Stroke::new(1.4_f32, p.accent_program),
                            ),
                            (false, true) => (
                                Color32::from_rgba_unmultiplied(
                                    p.accent_preview.r(),
                                    p.accent_preview.g(),
                                    p.accent_preview.b(),
                                    if hovered { 36 } else { 24 },
                                ),
                                Stroke::new(1.4_f32, p.accent_preview),
                            ),
                            (false, false) => (
                                if hovered {
                                    p.bg_elevated
                                } else {
                                    Color32::from_rgba_unmultiplied(
                                        p.bg_elevated.r(),
                                        p.bg_elevated.g(),
                                        p.bg_elevated.b(),
                                        140,
                                    )
                                },
                                Stroke::new(1.0_f32, p.border_subtle),
                            ),
                        };

                        ui.painter()
                            .rect_filled(row_rect, CornerRadius::same(6), card_fill);

                        // Left Status Accent Bar (5px wide)
                        let strip_rect = Rect::from_min_max(
                            row_rect.min,
                            Pos2::new(row_rect.min.x + 5.0, row_rect.max.y),
                        );
                        match (is_pgm, is_prv) {
                            (true, true) => {
                                let mid_y = row_rect.center().y;
                                let top_half = Rect::from_min_max(
                                    strip_rect.min,
                                    Pos2::new(strip_rect.max.x, mid_y),
                                );
                                let bot_half = Rect::from_min_max(
                                    Pos2::new(strip_rect.min.x, mid_y),
                                    strip_rect.max,
                                );
                                ui.painter().rect_filled(
                                    top_half,
                                    CornerRadius {
                                        nw: 6,
                                        ne: 0,
                                        sw: 0,
                                        se: 0,
                                    },
                                    p.accent_preview,
                                );
                                ui.painter().rect_filled(
                                    bot_half,
                                    CornerRadius {
                                        nw: 0,
                                        ne: 0,
                                        sw: 6,
                                        se: 0,
                                    },
                                    p.accent_program,
                                );
                            }
                            (true, false) => {
                                ui.painter().rect_filled(
                                    strip_rect,
                                    CornerRadius {
                                        nw: 6,
                                        ne: 0,
                                        sw: 6,
                                        se: 0,
                                    },
                                    p.accent_program,
                                );
                            }
                            (false, true) => {
                                ui.painter().rect_filled(
                                    strip_rect,
                                    CornerRadius {
                                        nw: 6,
                                        ne: 0,
                                        sw: 6,
                                        se: 0,
                                    },
                                    p.accent_preview,
                                );
                            }
                            (false, false) => {
                                ui.painter().rect_filled(
                                    strip_rect,
                                    CornerRadius {
                                        nw: 6,
                                        ne: 0,
                                        sw: 6,
                                        se: 0,
                                    },
                                    p.border_subtle,
                                );
                            }
                        }

                        ui.painter().rect_stroke(
                            row_rect,
                            CornerRadius::same(6),
                            border_stroke,
                            egui::StrokeKind::Inside,
                        );

                        // Mathematically centered text block & icon
                        let mid_y = row_rect.center().y;

                        let icon_col = if is_pgm {
                            p.accent_program
                        } else if is_prv {
                            p.accent_preview
                        } else {
                            p.text_secondary
                        };

                        // Draw icon exactly vertically centered on mid_y
                        let icon_pos = Pos2::new(row_rect.min.x + 13.0, mid_y);
                        ui.painter().text(
                            icon_pos,
                            Align2::LEFT_CENTER,
                            icons::LAYOUT_COLUMNS,
                            FontId::proportional(14.0),
                            icon_col,
                        );

                        let state_desc = match (is_pgm, is_prv) {
                            (true, true) => format!(
                                "Live & Preview • {}/{} layers",
                                active_src_count, src_count
                            ),
                            (true, false) => {
                                format!("On Air (Live) • {}/{} layers", active_src_count, src_count)
                            }
                            (false, true) => format!(
                                "Editing in Preview • {}/{} layers",
                                active_src_count, src_count
                            ),
                            (false, false) => {
                                format!("Standby • {}/{} layers", active_src_count, src_count)
                            }
                        };

                        // The status badges have a fixed width that does not depend on the panel
                        // size, so they are measured *before* the scene name is laid out. The name
                        // then gets exactly the leftover room instead of being overrun by a badge.
                        let text_x = row_rect.min.x + 33.0;

                        let live_label = format!("{} LIVE", icons::PLAYER_RECORD);
                        let preview_label = format!("{} PREVIEW", icons::EYE);
                        let standby_label = "STANDBY".to_string();

                        // Icon-only fallbacks: the glyph alone still reads as live / preview, so a
                        // narrow row drops the words rather than letting the badge overlap the name.
                        let live_compact = icons::PLAYER_RECORD.to_string();
                        let preview_compact = icons::EYE.to_string();
                        let standby_compact = icons::MINUS.to_string();

                        // (full label, compact label, background, border, text colour, tooltip)
                        let mut badges: Vec<(&str, &str, Color32, Color32, Color32, &str)> =
                            Vec::new();
                        if is_pgm {
                            badges.push((
                                live_label.as_str(),
                                live_compact.as_str(),
                                p.accent_program,
                                p.accent_program,
                                Color32::WHITE,
                                "This scene is currently LIVE on Program output",
                            ));
                        }
                        if is_prv {
                            badges.push((
                                preview_label.as_str(),
                                preview_compact.as_str(),
                                Color32::from_rgba_unmultiplied(
                                    p.accent_preview.r(),
                                    p.accent_preview.g(),
                                    p.accent_preview.b(),
                                    45,
                                ),
                                p.accent_preview,
                                p.accent_preview,
                                "This scene is currently open in Preview editor",
                            ));
                        }
                        if badges.is_empty() {
                            badges.push((
                                standby_label.as_str(),
                                standby_compact.as_str(),
                                Color32::TRANSPARENT,
                                p.border_subtle,
                                p.text_secondary,
                                "Click to open in Preview • Double-click to Transition to LIVE",
                            ));
                        }

                        let pill_gap = 4.0;
                        let badges_right = row_rect.max.x - 7.0;
                        // Room the badges may take while the scene name still keeps its minimum.
                        let badges_room = (badges_right - text_x - SCENE_NAME_MIN_W).max(0.0);

                        let measure = |ui: &egui::Ui, pick: usize| -> f32 {
                            badges
                                .iter()
                                .map(|b| {
                                    let label = if pick == 0 { b.0 } else { b.1 };
                                    status_pill_width(ui, label)
                                })
                                .sum::<f32>()
                                + pill_gap * (badges.len() as f32 - 1.0)
                        };
                        let full_w = measure(ui, 0);
                        let compact_w = measure(ui, 1);

                        // Fall back to the icon-only pills when the worded ones would not fit.
                        let use_compact = full_w > badges_room;
                        let badges_w = if use_compact { compact_w } else { full_w };
                        let badges_rect = Rect::from_min_max(
                            Pos2::new(badges_right - badges_w, row_rect.min.y + 2.0),
                            Pos2::new(badges_right, row_rect.max.y - 2.0),
                        );

                        // The name may occupy everything between the icon column and the badges.
                        let name_area_max_x = badges_rect.min.x - 6.0;
                        let name_max_w = (name_area_max_x - text_x).max(0.0);

                        let name_text = truncate_to_width(&scene.name, name_max_w, SCENE_NAME_CHAR_W);
                        let name_galley = ui.painter().layout_no_wrap(
                            name_text,
                            FontId::proportional(12.0),
                            p.text_primary,
                        );
                        let desc_galley = ui.painter().layout_no_wrap(
                            state_desc,
                            FontId::proportional(9.5),
                            p.text_secondary,
                        );

                        // Comfortable vertical spacing between scene title and subtitle
                        let gap_y = 3.5_f32;
                        let name_h = name_galley.size().y;
                        let desc_h = desc_galley.size().y;
                        let total_text_h = name_h + desc_h + gap_y;
                        let text_top_y = (mid_y - total_text_h * 0.5).round();

                        // Both text lines are clipped to the name column so a long subtitle can
                        // never run underneath the badges.
                        let text_clip = Rect::from_min_max(
                            Pos2::new(text_x, row_rect.min.y),
                            Pos2::new(name_area_max_x, row_rect.max.y),
                        );
                        let painter = ui.painter().with_clip_rect(text_clip);

                        // Draw clean, regular weight scene name (not bold)
                        painter.galley(
                            Pos2::new(text_x, text_top_y),
                            name_galley,
                            p.text_primary,
                        );

                        // Draw subtitle with comfortable breathing room below scene name
                        painter.galley(
                            Pos2::new(text_x, text_top_y + name_h + gap_y),
                            desc_galley,
                            p.text_secondary,
                        );

                        // Right-side visual status badges
                        let mut badges_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(badges_rect)
                                .layout(Layout::right_to_left(Align::Center)),
                        );
                        badges_ui.spacing_mut().item_spacing.x = pill_gap;
                        for badge in &badges {
                            let (label, bg_fill, border_col, text_col, tooltip) = if use_compact {
                                (badge.1, badge.2, badge.3, badge.4, badge.5)
                            } else {
                                (badge.0, badge.2, badge.3, badge.4, badge.5)
                            };
                            paint_status_pill(
                                &mut badges_ui,
                                label,
                                bg_fill,
                                border_col,
                                text_col,
                                tooltip,
                            );
                        }

                        let scene_id = scene.id;
                        let scene_name_for_menu = scene.name.clone();
                        if row_resp.double_clicked() {
                            transition_scene_to_live = Some(scene_id);
                        } else if row_resp.clicked() {
                            select_preview = Some(scene_id);
                        }

                        row_resp.context_menu(|ui| {
                            ui.set_min_width(190.0);
                            if ui
                                .button(format!(
                                    "{} Transition to LIVE (Program)",
                                    icons::ARROW_RIGHT
                                ))
                                .clicked()
                            {
                                transition_scene_to_live = Some(scene_id);
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui
                                .button(format!("{} Add Scene...", icons::PLUS))
                                .clicked()
                            {
                                add_new_scene_requested = true;
                                ui.close_menu();
                            }
                            if ui
                                .button(format!("{} Rename Scene...", icons::EDIT))
                                .clicked()
                            {
                                rename_scene_target =
                                    Some((scene_id, scene_name_for_menu.clone()));
                                ui.close_menu();
                            }
                            if ui
                                .button(format!("{} Duplicate Scene", icons::COPY))
                                .clicked()
                            {
                                duplicate_scene_target = Some(scene_id);
                                ui.close_menu();
                            }
                            if total_scenes > 1
                                && ui
                                    .button(
                                        RichText::new(format!(
                                            "{} Delete Scene",
                                            icons::TRASH
                                        ))
                                        .color(p.rec_red),
                                    )
                                    .clicked()
                            {
                                delete_scene_target = Some(scene_id);
                                ui.close_menu();
                            }
                        });

                        row_resp.on_hover_text(format!(
                            "{}\n• Click: Open in Preview\n• Double-Click: Transition to LIVE (Program)\n• Right-Click: Scene Options",
                            scene.top_display_label(idx + 1)
                        ));
                    }
                }

                if let Some(id) = transition_scene_to_live {
                    app.studio.preview_scene_id = id;
                    app.studio.transition_preview_to_program();
                } else if let Some(id) = select_preview {
                    app.studio.preview_scene_id = id;
                    app.studio.sync_if_linked();
                }

                if let Some((id, current_name)) = rename_scene_target {
                    app.studio.preview_scene_id = id;
                    app.rename_scene_buffer = current_name;
                    app.show_rename_scene_dialog = true;
                }
                if let Some(id) = duplicate_scene_target {
                    app.studio.preview_scene_id = id;
                    app.studio.duplicate_preview_scene();
                }
                if let Some(id) = delete_scene_target {
                    app.studio.delete_scene(id);
                }
                if add_new_scene_requested {
                    app.open_add_scene_dialog();
                }
            });
    });

    regions.with_footer(ui, |ui| {
        // Only `+` is left here. Rename, Duplicate and Delete all live on the scene row's
        // right-click menu, so repeating them in the footer just stole width from the one button
        // the footer is for — and the row menu is the only one of the two that acts on the scene
        // you actually right-clicked.
        let add_scene_btn = crate::ui::button_3d::Button3D::new("")
            .icon(icons::PLUS)
            .min_size(Vec2::new(26.0, 24.0))
            .tooltip("Add Scene...");

        if ui.add(add_scene_btn).clicked() {
            app.open_add_scene_dialog();
        }
    });

    Some(card_rect)
}

fn color_tag_tint(tag: SourceColorTag) -> Option<Color32> {
    match tag {
        SourceColorTag::None => None,
        SourceColorTag::Blue => Some(Color32::from_rgba_unmultiplied(56, 139, 253, 45)),
        SourceColorTag::Teal => Some(Color32::from_rgba_unmultiplied(34, 184, 207, 45)),
        SourceColorTag::Green => Some(Color32::from_rgba_unmultiplied(34, 197, 94, 45)),
        SourceColorTag::Amber => Some(Color32::from_rgba_unmultiplied(245, 158, 11, 45)),
        SourceColorTag::Red => Some(Color32::from_rgba_unmultiplied(239, 68, 68, 45)),
        SourceColorTag::Purple => Some(Color32::from_rgba_unmultiplied(168, 85, 247, 45)),
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum SourceContextAction {
    OpenAddSourceDialog(crate::studio::source::AddSourceType),
    AddApplicationAudio,
    AddAudioInput,
    AddAudioOutput,
    AddBrowser,
    AddColor,
    AddDisplay,
    AddGameCapture,
    AddImage,
    AddImageSlideShow,
    AddMedia,
    AddPowerPoint,
    AddSceneSource,
    AddTextGdiPlus,
    AddTvTextTemplate(TvTextTemplate),
    AddCamera,
    AddWindow,
    MoveUp(Uuid),
    MoveDown(Uuid),
    MoveTop(Uuid),
    MoveBottom(Uuid),
    ResetTransform(Uuid),
    RotateCw90(Uuid),
    RotateCcw90(Uuid),
    Rotate180(Uuid),
    FlipHorizontal(Uuid),
    FlipVertical(Uuid),
    FitToScreen(Uuid),
    StretchToScreen(Uuid),
    CenterOnScreen(Uuid),
    CenterVertically(Uuid),
    CenterHorizontally(Uuid),
    PipBottomRight(Uuid),
    PipBottomLeft(Uuid),
    PipTopRight(Uuid),
    ToggleLock(Uuid),
    SetColor(Uuid, SourceColorTag),
    SetFilterKind(Uuid, crate::studio::source::KeyFilterKind),
    OpenFilters(Uuid),
    /// Adds an audio filter of the given kind to the source's chain.
    AddAudioFilter(Uuid, crate::audio::filters::AudioFilterKind),
    /// Applies the built-in Clean Mic chain (noise suppression + noise gate).
    ApplyCleanMicPreset(Uuid),
    /// Opens the per-source audio filter chain modal.
    OpenAudioFilters(Uuid),
    Copy(Uuid),
    Paste,
    Duplicate(Uuid),
    Remove(Uuid),
    Rename(Uuid),
    OpenProperties(Uuid),
}

fn populate_add_source_menu(ui: &mut egui::Ui, action_out: &mut Option<SourceContextAction>) {
    ui.set_min_width(250.0);
    ui.label(RichText::new("Add Source").small().strong());
    ui.separator();

    if ui.button(format!("{}  Open Full Add Source Dialog...", icons::PLUS)).clicked() {
        *action_out = Some(SourceContextAction::OpenAddSourceDialog(crate::studio::source::AddSourceType::DisplayCapture));
        ui.close_menu();
    }
    ui.separator();

    let entries: &[(&str, &str, SourceContextAction)] = &[
        (
            icons::DEVICE_DESKTOP,
            "Display Capture",
            SourceContextAction::OpenAddSourceDialog(crate::studio::source::AddSourceType::DisplayCapture),
        ),
        (
            icons::APP_WINDOW,
            "Window Capture",
            SourceContextAction::OpenAddSourceDialog(crate::studio::source::AddSourceType::WindowCapture),
        ),
        (
            icons::PHOTO,
            "Image",
            SourceContextAction::OpenAddSourceDialog(crate::studio::source::AddSourceType::Image),
        ),
        (
            icons::PLAYER_PLAY,
            "Media (Video / MP3 / Audio)",
            SourceContextAction::OpenAddSourceDialog(crate::studio::source::AddSourceType::MediaSource),
        ),
        (
            icons::PRESENTATION,
            "PowerPoint / PPT Source",
            SourceContextAction::OpenAddSourceDialog(crate::studio::source::AddSourceType::PowerPoint),
        ),
        (
            icons::PDF,
            "PDF Document / Presentation",
            SourceContextAction::OpenAddSourceDialog(crate::studio::source::AddSourceType::Pdf),
        ),
        (
            icons::CLOCK,
            "Clock & Timer",
            SourceContextAction::OpenAddSourceDialog(crate::studio::source::AddSourceType::TimeSource),
        ),
        (
            icons::LETTER_CASE,
            "Text (GDI+)",
            SourceContextAction::OpenAddSourceDialog(crate::studio::source::AddSourceType::TextGdiPlus),
        ),
        (
            icons::BRUSH,
            "Color",
            SourceContextAction::OpenAddSourceDialog(crate::studio::source::AddSourceType::ColorSource),
        ),
        (
            icons::WORLD,
            "Browser",
            SourceContextAction::OpenAddSourceDialog(crate::studio::source::AddSourceType::BrowserSource),
        ),
        (
            icons::CAMERA,
            "Video Capture Device",
            SourceContextAction::OpenAddSourceDialog(crate::studio::source::AddSourceType::WebcamDevice),
        ),
        (
            icons::PRESENTATION,
            "Image Slide Show",
            SourceContextAction::OpenAddSourceDialog(crate::studio::source::AddSourceType::ImageSlideShow),
        ),
        (
            icons::DEVICE_GAMEPAD,
            "Game Capture",
            SourceContextAction::AddGameCapture,
        ),
        (
            icons::LIST,
            "Scene",
            SourceContextAction::AddSceneSource,
        ),
        (
            icons::MICROPHONE,
            "Audio Input Capture",
            SourceContextAction::AddAudioInput,
        ),
        (
            icons::VOLUME,
            "Audio Output Capture",
            SourceContextAction::AddAudioOutput,
        ),
        (
            icons::VOLUME,
            "Application Audio Capture (BETA)",
            SourceContextAction::AddApplicationAudio,
        ),
    ];

    let scroll_id = ui.id().with("add_source_menu_scroll_offset");
    let max_scroll_id = ui.id().with("add_source_menu_max_scroll");
    let mut current_offset = ui.data_mut(|d| d.get_temp::<f32>(scroll_id).unwrap_or(0.0));
    let max_scroll = ui.data_mut(|d| d.get_temp::<f32>(max_scroll_id).unwrap_or(400.0));
    const MENU_MAX_H: f32 = 300.0;

    // 1. Tangkap input mouse wheel untuk scroll menu naik/turun
    let wheel_delta = ui.input(|i| {
        let s = i.smooth_scroll_delta.y;
        let r = i.raw_scroll_delta.y;
        if s.abs() > 0.1 {
            s
        } else if r.abs() > 0.1 {
            r
        } else {
            0.0
        }
    });
    if wheel_delta.abs() > 0.1 {
        current_offset = (current_offset - wheel_delta * 1.5).clamp(0.0, max_scroll);
        ui.ctx().request_repaint();
    }

    // 2. Up arrow hover/click bar jika menu sedang ter-scroll ke bawah
    if current_offset > 2.0 {
        let (up_rect, up_resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::click());
        let up_hovered = up_resp.hovered();
        let bg_fill = if up_hovered { ui.visuals().widgets.hovered.bg_fill } else { ui.visuals().widgets.inactive.bg_fill.gamma_multiply(0.25) };
        ui.painter().rect_filled(up_rect, CornerRadius::same(3), bg_fill);
        let arrow_col = if up_hovered { ui.visuals().hyperlink_color } else { ui.visuals().text_color() };
        ui.painter().text(up_rect.center(), egui::Align2::CENTER_CENTER, icons::CHEVRON_UP, egui::FontId::proportional(14.0), arrow_col);
        if up_hovered {
            current_offset = (current_offset - 2.0).max(0.0);
            ui.ctx().request_repaint();
        }
        if up_resp.clicked() {
            current_offset = (current_offset - 30.0).max(0.0);
            ui.ctx().request_repaint();
        }
    }

    let scroll_out = egui::ScrollArea::vertical()
        .id_salt("add_source_menu_scroll_area")
        .max_height(MENU_MAX_H)
        .vertical_scroll_offset(current_offset)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (icon, label, act) in entries {
                if ui.button(format!("{}  {}", icon, label)).clicked() {
                    *action_out = Some(*act);
                    ui.close_menu();
                }
            }

            ui.separator();
            ui.menu_button(
                format!("{}  TV Text Show Templates", icons::PRESENTATION),
                |ui| {
                    ui.set_min_width(240.0);
                    for tpl in TvTextTemplate::all() {
                        if ui
                            .button(format!("{}  {}", icons::LETTER_CASE, tpl.label()))
                            .clicked()
                        {
                            *action_out = Some(SourceContextAction::AddTvTextTemplate(*tpl));
                            ui.close_menu();
                        }
                    }
                },
            );
        });

    let actual_offset = scroll_out.state.offset.y;
    let actual_max = (scroll_out.content_size.y - MENU_MAX_H).max(0.0);
    current_offset = actual_offset;

    // 3. Down arrow hover/click bar jika masih ada konten tertutup di bawah
    if current_offset < actual_max - 2.0 {
        let (down_rect, down_resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), Sense::click());
        let down_hovered = down_resp.hovered();
        let bg_fill = if down_hovered { ui.visuals().widgets.hovered.bg_fill } else { ui.visuals().widgets.inactive.bg_fill.gamma_multiply(0.25) };
        ui.painter().rect_filled(down_rect, CornerRadius::same(3), bg_fill);
        let arrow_col = if down_hovered { ui.visuals().hyperlink_color } else { ui.visuals().text_color() };
        ui.painter().text(down_rect.center(), egui::Align2::CENTER_CENTER, icons::CHEVRON_DOWN, egui::FontId::proportional(14.0), arrow_col);
        if down_hovered {
            current_offset = (current_offset + 2.0).min(actual_max);
            ui.ctx().request_repaint();
        }
        if down_resp.clicked() {
            current_offset = (current_offset + 30.0).min(actual_max);
            ui.ctx().request_repaint();
        }
    }

    ui.data_mut(|d| {
        d.insert_temp(scroll_id, current_offset);
        d.insert_temp(max_scroll_id, actual_max);
    });
}

fn populate_source_context_menu(
    ui: &mut egui::Ui,
    src_id: Uuid,
    is_locked: bool,
    current_color: SourceColorTag,
    current_filter_kind: crate::studio::source::KeyFilterKind,
    // True for Audio Input / Output / Application Capture, which can host an audio chain.
    is_audio_source: bool,
    // Number of filters already configured on the source, shown as a hint in the menu.
    audio_filter_count: usize,
    copied_source_name: Option<&str>,
    action_out: &mut Option<SourceContextAction>,
) {
    ui.set_min_width(220.0);

    ui.menu_button(format!("{}  Add Source", icons::PLUS), |ui| {
        populate_add_source_menu(ui, action_out);
    });

    ui.separator();

    ui.menu_button(format!("{}  Filters", icons::FILTER), |ui| {
        ui.set_min_width(220.0);
        for fk in crate::studio::source::KeyFilterKind::all() {
            let active = current_filter_kind == *fk;
            let resp = ui.selectable_label(active, fk.label()).on_hover_text(fk.hover_explanation());
            if resp.clicked() {
                *action_out = Some(SourceContextAction::SetFilterKind(src_id, *fk));
                ui.close_menu();
            }
        }
        ui.separator();
        if ui
            .button(format!("{}  Configure Filters...", icons::ADJUSTMENTS))
            .on_hover_text("Open filter settings window for keying, target color, and smoothness")
            .clicked()
        {
            *action_out = Some(SourceContextAction::OpenFilters(src_id));
            ui.close_menu();
        }
    });

    // Audio filter chain: only the three audio capture kinds can host one, matching the

    // rule that the global mixer buses stay raw device buses.

    if is_audio_source {

        ui.menu_button(

            format!("{}  Audio Filters ({})", icons::ADJUSTMENTS, audio_filter_count),

            |ui| {

                ui.set_min_width(240.0);

                for fk in crate::audio::filters::AudioFilterKind::all() {

                    if ui

                        .button(format!("{}  Add {}", icons::PLUS, fk.short_name()))

                        .on_hover_text(fk.hover_explanation())

                        .clicked()

                    {

                        *action_out = Some(SourceContextAction::AddAudioFilter(src_id, *fk));

                        ui.close_menu();

                    }

                }

                ui.separator();

                if ui

                    .button(format!("{}  Clean Mic Preset", icons::SPARKLES))

                    .on_hover_text(

                        "Replace this source's chain with Noise Suppression + Noise Gate — the standard broadcast starting point for a microphone",

                    )

                    .clicked()

                {

                    *action_out = Some(SourceContextAction::ApplyCleanMicPreset(src_id));

                    ui.close_menu();

                }

                ui.separator();

                if ui

                    .button(format!("{}  Configure Audio Filters...", icons::ADJUSTMENTS))

                    .on_hover_text("Open the audio filter chain editor for this source")

                    .clicked()

                {

                    *action_out = Some(SourceContextAction::OpenAudioFilters(src_id));

                    ui.close_menu();

                }

            },

        );

    }
    ui.menu_button(format!("{}  Set Color", icons::PALETTE), |ui| {
        ui.set_min_width(145.0);
        for tag in SourceColorTag::all() {
            if ui
                .selectable_label(current_color == *tag, tag.label())
                .clicked()
            {
                *action_out = Some(SourceContextAction::SetColor(src_id, *tag));
                ui.close_menu();
            }
        }
    });

    ui.menu_button(format!("{}  Order", icons::ARROWS_UP_DOWN), |ui| {
        ui.set_min_width(185.0);
        if ui
            .button(format!("{}  Move Up (Bring Forward)", icons::ARROW_UP))
            .clicked()
        {
            *action_out = Some(SourceContextAction::MoveUp(src_id));
            ui.close_menu();
        }
        if ui
            .button(format!("{}  Move Down (Send Backward)", icons::ARROW_DOWN))
            .clicked()
        {
            *action_out = Some(SourceContextAction::MoveDown(src_id));
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Move to Top (Front)").clicked() {
            *action_out = Some(SourceContextAction::MoveTop(src_id));
            ui.close_menu();
        }
        if ui.button("Move to Bottom (Back)").clicked() {
            *action_out = Some(SourceContextAction::MoveBottom(src_id));
            ui.close_menu();
        }
    });

    ui.menu_button(format!("{}  Transform", icons::TRANSFORM), |ui| {
        ui.set_min_width(220.0);
        if ui.button("Rotate 90 degrees CW").clicked() {
            *action_out = Some(SourceContextAction::RotateCw90(src_id));
            ui.close_menu();
        }
        if ui.button("Rotate 90 degrees CCW").clicked() {
            *action_out = Some(SourceContextAction::RotateCcw90(src_id));
            ui.close_menu();
        }
        if ui.button("Rotate 180 degrees").clicked() {
            *action_out = Some(SourceContextAction::Rotate180(src_id));
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Flip Horizontal").clicked() {
            *action_out = Some(SourceContextAction::FlipHorizontal(src_id));
            ui.close_menu();
        }
        if ui.button("Flip Vertical").clicked() {
            *action_out = Some(SourceContextAction::FlipVertical(src_id));
            ui.close_menu();
        }
        ui.separator();
        if ui.add(egui::Button::new("Fit to screen").shortcut_text("Ctrl+F")).clicked() {
            *action_out = Some(SourceContextAction::FitToScreen(src_id));
            ui.close_menu();
        }
        if ui.add(egui::Button::new("Stretch to screen").shortcut_text("Ctrl+S")).clicked() {
            *action_out = Some(SourceContextAction::StretchToScreen(src_id));
            ui.close_menu();
        }
        if ui.add(egui::Button::new("Center to screen").shortcut_text("Ctrl+D")).clicked() {
            *action_out = Some(SourceContextAction::CenterOnScreen(src_id));
            ui.close_menu();
        }
        if ui.button("Center Vertically").clicked() {
            *action_out = Some(SourceContextAction::CenterVertically(src_id));
            ui.close_menu();
        }
        if ui.button("Center Horizontally").clicked() {
            *action_out = Some(SourceContextAction::CenterHorizontally(src_id));
            ui.close_menu();
        }
        ui.separator();
        if ui.add(egui::Button::new(format!("{}  Reset Transform", icons::REFRESH)).shortcut_text("Ctrl+R")).clicked() {
            *action_out = Some(SourceContextAction::ResetTransform(src_id));
            ui.close_menu();
        }
        ui.separator();
        if ui.button("PiP Bottom-Right").clicked() {
            *action_out = Some(SourceContextAction::PipBottomRight(src_id));
            ui.close_menu();
        }
        if ui.button("PiP Bottom-Left").clicked() {
            *action_out = Some(SourceContextAction::PipBottomLeft(src_id));
            ui.close_menu();
        }
        if ui.button("PiP Top-Right").clicked() {
            *action_out = Some(SourceContextAction::PipTopRight(src_id));
            ui.close_menu();
        }
        ui.separator();
        let lock_text = if is_locked {
            format!("{}  Unlock Transform", icons::LOCK_OPEN)
        } else {
            format!("{}  Lock Transform", icons::LOCK)
        };
        if ui.button(lock_text).clicked() {
            *action_out = Some(SourceContextAction::ToggleLock(src_id));
            ui.close_menu();
        }
    });

    ui.separator();

    // Shortcut hints go through `shortcut_text()` so egui lays out a real right-aligned column.
    // Padding the label with literal spaces (what this block used to do) can never align: the
    // required run length differs per string, and a dynamic name such as `Paste 'Clip 2'` pushes
    // the hint out of the column entirely.
    if ui
        .add(egui::Button::new(format!("{}  Copy", icons::COPY)).shortcut_text("Ctrl+C"))
        .clicked()
    {
        *action_out = Some(SourceContextAction::Copy(src_id));
        ui.close_menu();
    }

    let paste_label = if let Some(name) = copied_source_name {
        format!("{}  Paste '{}'", icons::CLIPBOARD, name)
    } else {
        format!("{}  Paste", icons::CLIPBOARD)
    };
    if ui
        .add_enabled(
            copied_source_name.is_some(),
            egui::Button::new(paste_label).shortcut_text("Ctrl+V"),
        )
        .clicked()
    {
        *action_out = Some(SourceContextAction::Paste);
        ui.close_menu();
    }

    if ui
        .add(egui::Button::new(format!("{}  Duplicate", icons::COPY)).shortcut_text("Ctrl+D"))
        .on_hover_text("Duplicate this layer in active scene")
        .clicked()
    {
        *action_out = Some(SourceContextAction::Duplicate(src_id));
        ui.close_menu();
    }

    if ui
        .add(egui::Button::new(format!("{}  Remove", icons::TRASH)).shortcut_text("Del"))
        .clicked()
    {
        *action_out = Some(SourceContextAction::Remove(src_id));
        ui.close_menu();
    }

    if ui
        .add(egui::Button::new(format!("{}  Rename...", icons::EDIT)).shortcut_text("F2"))
        .clicked()
    {
        *action_out = Some(SourceContextAction::Rename(src_id));
        ui.close_menu();
    }

    ui.separator();

    if ui
        .button(format!("{}  Filters...", icons::FILTER))
        .on_hover_text("Configure Chroma Key, Color Key, or Luma Key")
        .clicked()
    {
        *action_out = Some(SourceContextAction::OpenFilters(src_id));
        ui.close_menu();
    }

    if ui
        .button(format!("{}  Properties", icons::SETTINGS))
        .clicked()
    {
        *action_out = Some(SourceContextAction::OpenProperties(src_id));
        ui.close_menu();
    }
}

/// Returns the card's outer rect so the caller can attach a width splitter to its right edge.
fn render_sources_panel(ui: &mut egui::Ui, app: &mut NexoraApp, size: Vec2) -> Option<Rect> {
    let p = ThemePalette::for_mode(app.settings.theme);
    let mut pending_action: Option<SourceContextAction> = None;

    // Keyboard shortcuts for selected source (Delete / F2 / Left-Right slide navigation / Ctrl+C / Ctrl+V / Ctrl+D) when no modal or text input is active
    if !ui.ctx().wants_keyboard_input() && !app.is_any_modal_open() {
        let ctrl = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
        if ctrl && ui.input(|i| i.key_pressed(egui::Key::V)) {
            if app.copied_source.is_some() {
                pending_action = Some(SourceContextAction::Paste);
            }
        } else if let Some(sel_id) = app.studio.preview_scene().selected_source_id {
            if ctrl && ui.input(|i| i.key_pressed(egui::Key::C)) {
                pending_action = Some(SourceContextAction::Copy(sel_id));
            } else if ctrl && ui.input(|i| i.key_pressed(egui::Key::D)) {
                pending_action = Some(SourceContextAction::Duplicate(sel_id));
            } else
            if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                pending_action = Some(SourceContextAction::Remove(sel_id));
            } else if ui.input(|i| i.key_pressed(egui::Key::F2)) {
                pending_action = Some(SourceContextAction::Rename(sel_id));
            } else if ui.input(|i| i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::PageDown)) {
                app.advance_presentation_slide(1);
            } else if ui.input(|i| i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::PageUp)) {
                app.advance_presentation_slide(-1);
            }
        }
    }

    let (regions, card_rect) = paint_and_split_dock_card(ui, &p, size);

    regions.with_header(ui, |ui| {
        ui.label(
            RichText::new("SOURCE")
                .size(13.5)
                .strong()
                .color(p.text_primary),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("—").small())
                        .min_size(Vec2::new(22.0, 20.0)),
                )
                .on_hover_text("Minimize Source panel")
                .clicked()
            {
                app.settings.panels.show_sources = false;
            }
            paint_header_hint(ui, &p, "Right-click to edit", 11.0, false);
        });
    });

    regions.with_body(ui, |ui, body_size| {
        egui::ScrollArea::vertical()
            .id_salt("sources_scroll")
            .max_height(body_size.y)
            // Drag-to-scroll would swallow the row drag used for reordering the layer stack;
            // the wheel and the scroll bar keep the list scrollable.
            .drag_to_scroll(false)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let scene = app.studio.preview_scene_mut();
                let cur_sel = scene.selected_source_id;
                let mut new_selected = cur_sel;
                let mut vis_changed = false;
                let mut pending_slide_target: Option<(uuid::Uuid, i32)> = None;

                if scene.sources.is_empty() {
                    let empty_resp = ui.selectable_label(
                        false,
                        RichText::new("No sources. Click '+' below or right-click to Add Source")
                            .small()
                            .color(p.text_muted),
                    );
                    empty_resp.context_menu(|ui| {
                        if let Some(src) = &app.copied_source {
                            if ui
                                .add(
                                    egui::Button::new(format!("{}  Paste '{}'", icons::CLIPBOARD, src.name))
                                        .shortcut_text("Ctrl+V"),
                                )
                                .clicked()
                            {
                                pending_action = Some(SourceContextAction::Paste);
                                ui.close_menu();
                            }
                            ui.separator();
                        }
                        populate_add_source_menu(ui, &mut pending_action);
                    });
                }

                let row_w = (body_size.x - 4.0).max(150.0);
                ui.spacing_mut().item_spacing.y = 4.0;

                // Row rectangles are collected while the list is laid out so the drop gap can be
                // resolved from the pointer position even after it leaves the grabbed row.
                let mut row_rects: Vec<(uuid::Uuid, egui::Rect)> =
                    Vec::with_capacity(scene.sources.len());
                // Label of the grabbed row, reused to paint the floating ghost after the loop.
                let mut drag_label: Option<String> = None;

                // Raw pointer state drives the reorder gesture: a press arms a row and moving far
                // enough turns that press into a drag, so a plain click still only selects.
                let (pointer_pos, pressed_now) = ui.ctx().input(|i| {
                    (
                        i.pointer.interact_pos().or(i.pointer.hover_pos()),
                        i.pointer.any_pressed() && i.pointer.primary_down(),
                    )
                });
                // Moving less than this keeps the press a click, so selection stays snappy.
                const REORDER_DRAG_THRESHOLD: f32 = 4.0;

                for (src_index, src) in scene.sources.iter_mut().enumerate() {
                    let is_sel = cur_sel == Some(src.id);
                    let src_id = src.id;
                    let is_locked = src.locked;
                    let current_color = src.color_tag;
                    let current_filter_kind = src.filter.kind;
                    let is_audio_source = src.supports_audio_filters();
                    let audio_filter_count = src.audio_filters.len();
                    let is_media = matches!(src.kind, SourceKind::MediaSource { .. });
                    let is_ppt = matches!(src.kind, SourceKind::PowerPointSource { .. });
                    let is_pdf = matches!(src.kind, SourceKind::PdfSource { .. });
                    let is_presentation = is_ppt || is_pdf;

                    let row_h = 31.0;
                    let (row_rect, _) =
                        ui.allocate_exact_size(Vec2::new(row_w, row_h), Sense::hover());
                    row_rects.push((src_id, row_rect));

                    // Reserve right-hand zone for action buttons ([Play] [Mute] [Eye] [Lock] or [Prev] [Next] [Eye] [Lock])
                    let btn_count = if is_media || is_presentation { 4 } else { 2 };
                    let buttons_zone_w = (btn_count as f32) * 28.0 + 6.0;
                    let select_rect = Rect::from_min_max(
                        row_rect.min,
                        Pos2::new(
                            (row_rect.max.x - buttons_zone_w).max(row_rect.min.x + 48.0),
                            row_rect.max.y,
                        ),
                    );

                    // The row keeps a plain `click` sense, so hovering and selecting look and feel
                    // exactly as before; the reorder gesture is tracked from the raw pointer below
                    // (a `Sense::drag` row would be eaten by the enclosing scroll area).
                    let row_resp = ui.interact(
                        select_rect,
                        ui.id().with(("source_row_select", src_id)),
                        Sense::click(),
                    );
                    let hovered = row_resp.hovered() || ui.rect_contains_pointer(row_rect);
                    // Two independent hit tests: the widget response (press started on the row)
                    // and the raw rect test, so the gesture arms even if one of them misses.
                    if pressed_now
                        && (row_resp.is_pointer_button_down_on()
                            || ui.rect_contains_pointer(select_rect))
                    {
                        // Only a press that starts on the row may become a drag reorder.
                        let grip_offset_y = pointer_pos
                            .map(|p| (p.y - row_rect.top()).clamp(0.0, row_rect.height()))
                            .unwrap_or(row_rect.height() * 0.5);
                        app.source_press = Some(crate::app::SourceDragGrab {
                            source_id: src_id,
                            from_index: src_index,
                            grip_offset_y,
                        });
                        tracing::debug!(
                            target: "nexora::dnd",
                            source = %src_id,
                            index = src_index,
                            "row pressed (reorder candidate)"
                        );
                    }
                    let is_dragging =
                        app.source_reorder_drag.as_ref().map(|d| d.source_id) == Some(src_id);

                    // Card Fill & Border (matching Scenes card style + optional color tag tint)
                    let tag_tint = color_tag_tint(src.color_tag);
                    let (card_fill, border_stroke, strip_col) = if is_dragging {
                        // The grabbed row leaves a faded placeholder behind; the ghost card that
                        // follows the pointer is what the user actually sees moving.
                        (
                            Color32::from_rgba_unmultiplied(
                                p.accent_preview.r(),
                                p.accent_preview.g(),
                                p.accent_preview.b(),
                                14,
                            ),
                            Stroke::new(1.0_f32, p.border_subtle),
                            p.border_subtle.gamma_multiply(0.7),
                        )
                    } else if is_sel {
                        let base_fill = tag_tint.unwrap_or_else(|| {
                            Color32::from_rgba_unmultiplied(
                                p.accent_preview.r(),
                                p.accent_preview.g(),
                                p.accent_preview.b(),
                                if hovered { 38 } else { 26 },
                            )
                        });
                        (
                            base_fill,
                            Stroke::new(1.4_f32, p.accent_preview),
                            p.accent_preview,
                        )
                    } else if let Some(tint) = tag_tint {
                        (
                            tint,
                            Stroke::new(
                                1.0_f32,
                                if hovered {
                                    p.accent_preview
                                } else {
                                    p.border_subtle
                                },
                            ),
                            tint.gamma_multiply(1.8),
                        )
                    } else {
                        (
                            if hovered {
                                p.bg_elevated
                            } else {
                                Color32::from_rgba_unmultiplied(
                                    p.bg_elevated.r(),
                                    p.bg_elevated.g(),
                                    p.bg_elevated.b(),
                                    140,
                                )
                            },
                            Stroke::new(1.0_f32, p.border_subtle),
                            p.border_subtle,
                        )
                    };

                    ui.painter()
                        .rect_filled(row_rect, CornerRadius::same(6), card_fill);

                    // Left Accent Strip (4.5px wide)
                    let strip_rect = Rect::from_min_max(
                        row_rect.min,
                        Pos2::new(row_rect.min.x + 4.5, row_rect.max.y),
                    );
                    ui.painter().rect_filled(
                        strip_rect,
                        CornerRadius {
                            nw: 6,
                            ne: 0,
                            sw: 6,
                            se: 0,
                        },
                        strip_col,
                    );

                    ui.painter().rect_stroke(
                        row_rect,
                        CornerRadius::same(6),
                        border_stroke,
                        egui::StrokeKind::Inside,
                    );

                    // Draw Source Icon & Name (and optional Filter badge) inside `select_rect`
                    let text_col = if is_dragging {
                        // The grabbed row fades out while its floating ghost follows the pointer.
                        p.text_muted.gamma_multiply(0.65)
                    } else if !src.visible {
                        p.text_muted
                    } else if is_sel {
                        p.text_primary
                    } else {
                        p.text_secondary
                    };

                    let label_text = if src.filter.is_active() {
                        format!("{}  {} [{}]", src.kind.tabler_icon(), src.name, src.filter.kind.short_name())
                    } else {
                        format!("{}  {}", src.kind.tabler_icon(), src.name)
                    };
                    if is_dragging {
                        drag_label = Some(label_text.clone());
                    }

                    let text_clip = Rect::from_min_max(
                        Pos2::new(row_rect.min.x + 11.0, row_rect.min.y),
                        Pos2::new(select_rect.max.x - 4.0, row_rect.max.y),
                    );
                    ui.painter().with_clip_rect(text_clip).text(
                        Pos2::new(row_rect.min.x + 11.0, row_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        label_text,
                        egui::FontId::proportional(12.0),
                        text_col,
                    );

                    if row_resp.clicked() {
                        new_selected = Some(src_id);
                    }
                    if row_resp.double_clicked() {
                        new_selected = Some(src_id);
                        pending_action = Some(SourceContextAction::OpenProperties(src_id));
                    }

                    row_resp.context_menu(|ui| {
                        populate_source_context_menu(
                            ui,
                            src_id,
                            is_locked,
                            current_color,
                            current_filter_kind,
                            is_audio_source,
                            audio_filter_count,
                            app.copied_source.as_ref().map(|s| s.name.as_str()),
                            &mut pending_action,
                        );
                    });
                    row_resp.on_hover_text(format!(
                        "{}\n• Click anywhere on row: Select Source\n• Double-click: Open Properties\n• Right-click: Source Options\n• Drag row up/down: Change layer order (top = front)",
                        src.kind.summary()
                    ));

                    // Render Right-Side Action Buttons inside the right zone of `row_rect`
                    let buttons_rect = Rect::from_min_max(
                        Pos2::new(select_rect.max.x, row_rect.min.y),
                        Pos2::new(row_rect.max.x - 6.0, row_rect.max.y),
                    );
                    let mut btn_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(buttons_rect)
                            .layout(Layout::right_to_left(Align::Center)),
                    );
                    btn_ui.spacing_mut().item_spacing.x = 4.0;

                    let lock_label = if src.locked {
                        icons::LOCK
                    } else {
                        icons::LOCK_OPEN
                    };
                    let lock_col = if src.locked { Some(p.rec_red) } else { None };
                    if paint_icon_action_btn(
                        &mut btn_ui,
                        lock_label,
                        lock_col,
                        &p,
                        "Lock / Unlock position on canvas",
                    ) {
                        src.locked = !src.locked;
                        vis_changed = true;
                    }

                    let vis_label = if src.visible {
                        icons::EYE
                    } else {
                        icons::EYE_OFF
                    };
                    let vis_col = if src.visible {
                        None
                    } else {
                        Some(p.text_muted)
                    };
                    if paint_icon_action_btn(
                        &mut btn_ui,
                        vis_label,
                        vis_col,
                        &p,
                        "Show / Hide this source",
                    ) {
                        src.visible = !src.visible;
                        vis_changed = true;
                    }

                    if let SourceKind::MediaSource {
                        muted, is_playing, ..
                    } = &mut src.kind
                    {
                        let snd_icon = if *muted {
                            icons::VOLUME_OFF
                        } else {
                            icons::VOLUME
                        };
                        let snd_col = if *muted { Some(p.rec_red) } else { None };
                        if paint_icon_action_btn(
                            &mut btn_ui,
                            snd_icon,
                            snd_col,
                            &p,
                            "Mute / Unmute Media Playback Audio",
                        ) {
                            *muted = !*muted;
                            vis_changed = true;
                        }

                        let play_icon = if *is_playing {
                            icons::PLAYER_PAUSE
                        } else {
                            icons::PLAYER_PLAY
                        };
                        let play_col = if *is_playing {
                            None
                        } else {
                            Some(p.accent_preview)
                        };
                        if paint_icon_action_btn(
                            &mut btn_ui,
                            play_icon,
                            play_col,
                            &p,
                            "Pause / Play Media",
                        ) {
                            *is_playing = !*is_playing;
                            vis_changed = true;
                        }
                    }

                    if let SourceKind::PdfSource {
                        page_index,
                        page_count,
                        pages,
                        ..
                    } = &mut src.kind
                    {
                        let total = if *page_count > 0 { *page_count } else { pages.len() };
                        if total > 0 {
                            // In right_to_left layout, render Next first so on screen [ <- ] is left and [ -> ] is right
                            let tip_next = format!("Next Page (Current: {}/{})", *page_index + 1, total);
                            if paint_icon_action_btn(
                                &mut btn_ui,
                                icons::ARROW_RIGHT,
                                None,
                                &p,
                                &tip_next,
                            ) {
                                pending_slide_target = Some((src.id, 1));
                            }

                            let tip_prev = format!("Previous Page (Current: {}/{})", *page_index + 1, total);
                            if paint_icon_action_btn(
                                &mut btn_ui,
                                icons::ARROW_LEFT,
                                None,
                                &p,
                                &tip_prev,
                            ) {
                                pending_slide_target = Some((src.id, -1));
                            }
                        }
                    }

                    if let SourceKind::PowerPointSource {
                        slide_index,
                        slide_count,
                        slides,
                        ..
                    } = &mut src.kind
                    {
                        let total = if *slide_count > 0 { *slide_count } else { slides.len() };
                        if total > 0 {
                            // In right_to_left layout, render Next first so on screen [ <- ] is left and [ -> ] is right
                            let tip_next = format!("Next Slide (Current: {}/{})", *slide_index + 1, total);
                            if paint_icon_action_btn(
                                &mut btn_ui,
                                icons::ARROW_RIGHT,
                                None,
                                &p,
                                &tip_next,
                            ) {
                                pending_slide_target = Some((src.id, 1));
                            }

                            let tip_prev = format!("Previous Slide (Current: {}/{})", *slide_index + 1, total);
                            if paint_icon_action_btn(
                                &mut btn_ui,
                                icons::ARROW_LEFT,
                                None,
                                &p,
                                &tip_prev,
                            ) {
                                pending_slide_target = Some((src.id, -1));
                            }
                        }
                    }

                    if let SourceKind::TimeSource {
                        mode,
                        is_running,
                        elapsed_sec,
                        ..
                    } = &mut src.kind
                    {
                        if *mode != crate::studio::source::TimeSourceMode::RealTimeClock {
                            let (play_ic, play_tip) = if *is_running {
                                (icons::PLAYER_PAUSE, "Pause Timer")
                            } else {
                                (icons::PLAYER_PLAY, "Start / Resume Timer")
                            };
                            if paint_icon_action_btn(
                                &mut btn_ui,
                                play_ic,
                                None,
                                &p,
                                play_tip,
                            ) {
                                *is_running = !*is_running;
                                vis_changed = true;
                            }

                            if paint_icon_action_btn(
                                &mut btn_ui,
                                icons::REFRESH,
                                None,
                                &p,
                                "Reset Timer to 00:00",
                            ) {
                                *elapsed_sec = 0.0;
                                vis_changed = true;
                            }
                        }
                    }
                }
                // --- Drag & Drop Reorder (Z-Order) -------------------------------------------
                // The list order *is* the Z-order (index 0 = front-most layer), so dropping a row
                // simply moves that source to a new slot in the stack.
                let pointer_down = ui.ctx().input(|i| i.pointer.any_down());
                // A press becomes a drag only after the pointer clearly moves, so a click keeps
                // selecting the row exactly like before.
                if let Some(press) = app.source_press {
                    let press_origin = ui.ctx().input(|i| i.pointer.press_origin());
                    let moved = match (press_origin, pointer_pos) {
                        (Some(origin), Some(p)) => (p - origin).length(),
                        // Fallback when the origin is already gone: distance from the grab point.
                        (_, Some(p)) => row_rects
                            .get(press.from_index)
                            .map(|(_, r)| (p.y - (r.top() + press.grip_offset_y)).abs())
                            .unwrap_or(0.0),
                        _ => 0.0,
                    };
                    if !pointer_down {
                        app.source_press = None;
                    } else if moved > REORDER_DRAG_THRESHOLD && app.source_reorder_drag.is_none() {
                        app.source_reorder_drag = Some(press);
                        new_selected = Some(press.source_id);
                        tracing::debug!(
                            target: "nexora::dnd",
                            source = %press.source_id,
                            from = press.from_index,
                            "reorder drag started"
                        );
                    }
                }

                let mut drop_slot: Option<(uuid::Uuid, usize)> = None;
                if let Some(drag) = app.source_reorder_drag {
                    // The list can change mid-drag (source deleted, scene switched, panel
                    // re-laid out), so only honour the drag while the grabbed row is still there.
                    let grabbed_ok = row_rects
                        .get(drag.from_index)
                        .map(|(id, _)| *id == drag.source_id)
                        .unwrap_or(false);
                    if !grabbed_ok {
                        // The grabbed row vanished: cancel without touching the order.
                        app.source_reorder_drag = None;
                        app.source_press = None;
                    } else if let Some(pointer_pos) = pointer_pos {
                        // Rows in display order, without the dragged one.
                        let others: Vec<egui::Rect> = row_rects
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| *i != drag.from_index)
                            .map(|(_, (_, r))| *r)
                            .collect();
                        // The gap the source falls into: every row whose centre sits above the
                        // pointer has already been passed.
                        let slot = others
                            .iter()
                            .filter(|r| r.center().y < pointer_pos.y)
                            .count();
                        if !pointer_down {
                            // Released: commit the drop.
                            drop_slot = Some((drag.source_id, slot));
                            app.source_reorder_drag = None;
                            app.source_press = None;
                            tracing::debug!(
                                target: "nexora::dnd",
                                source = %drag.source_id,
                                from = drag.from_index,
                                to = slot,
                                "reorder drop"
                            );
                        } else {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                        // Gap indicator drawn between the two rows the source lands between.
                        let gap_y = if slot < others.len() {
                            others[slot].top()
                        } else {
                            others.last().map(|r| r.bottom()).unwrap_or(pointer_pos.y)
                        };
                        let x0 = row_rects.first().map(|(_, r)| r.left()).unwrap_or(0.0);
                        let x1 = row_rects.last().map(|(_, r)| r.right()).unwrap_or(0.0);
                        ui.painter().hline(
                            x0..=x1,
                            gap_y,
                            Stroke::new(2.0_f32, p.accent_preview),
                        );

                        // Floating ghost card: the row lifted off the list and stuck to the
                        // pointer, so the item being dragged is unmistakable.
                        if let Some(label) = drag_label.as_deref() {
                            let ghost_rect = Rect::from_min_size(
                                Pos2::new(x0, pointer_pos.y - drag.grip_offset_y),
                                Vec2::new((x1 - x0).max(120.0), 31.0),
                            );
                            let clip = ui.clip_rect().intersect(ghost_rect.expand(8.0));
                            let ghost = ui.painter().with_clip_rect(clip);
                            // Soft drop shadow underneath the card.
                            ghost.rect_filled(
                                ghost_rect.translate(Vec2::new(0.0, 3.0)),
                                CornerRadius::same(7),
                                Color32::from_black_alpha(80),
                            );
                            ghost.rect_filled(
                                ghost_rect,
                                CornerRadius::same(6),
                                Color32::from_rgba_unmultiplied(
                                    p.bg_elevated.r(),
                                    p.bg_elevated.g(),
                                    p.bg_elevated.b(),
                                    245,
                                ),
                            );
                            ghost.rect_stroke(
                                ghost_rect,
                                CornerRadius::same(6),
                                Stroke::new(1.4_f32, p.accent_preview),
                                egui::StrokeKind::Inside,
                            );
                            ghost.rect_filled(
                                Rect::from_min_max(
                                    ghost_rect.min,
                                    Pos2::new(ghost_rect.min.x + 4.5, ghost_rect.max.y),
                                ),
                                CornerRadius {
                                    nw: 6,
                                    ne: 0,
                                    sw: 6,
                                    se: 0,
                                },
                                p.accent_preview,
                            );
                            ghost.text(
                                Pos2::new(ghost_rect.min.x + 11.0, ghost_rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                label,
                                egui::FontId::proportional(12.0),
                                p.text_primary,
                            );
                            }
                        }
                    } else if !pointer_down {
                        // Pointer left the window on release: cancel instead of guessing a slot.
                        app.source_reorder_drag = None;
                        app.source_press = None;
                    }
                }
                let mut reordered = false;
                if let Some((drag_id, slot)) = drop_slot {
                    if scene.move_source_to_slot(drag_id, slot) {
                        reordered = true;
                    }
                }

                let sel_src = scene
                    .sources
                    .iter()
                    .find(|s| s.id == new_selected.unwrap_or(uuid::Uuid::nil()));
                let sel_filter_k = sel_src.map(|s| s.filter.kind).unwrap_or_default();
                let sel_is_audio = sel_src.map(|s| s.supports_audio_filters()).unwrap_or(false);
                let sel_audio_count = sel_src.map(|s| s.audio_filters.len()).unwrap_or(0);

                scene.selected_source_id = new_selected;
                if let Some((tgt_id, delta)) = pending_slide_target {
                    app.advance_source_slide(Some(tgt_id), delta);
                }
                if vis_changed {
                    app.studio.sync_media_transport_to_program();
                    app.studio.sync_if_linked();
                }
                if reordered {
                    // Layer order changed, so the linked Program has to follow the Preview.
                    app.studio.sync_if_linked();
                }

                let rem = ui.available_size();
                if rem.y > 6.0 {
                    let (_, blank_resp) = ui.allocate_exact_size(
                        Vec2::new(rem.x.max(80.0), rem.y),
                        Sense::click(),
                    );
                    blank_resp.context_menu(|ui| {
                        if let Some(src) = &app.copied_source {
                            if ui
                                .add(
                                    egui::Button::new(format!("{}  Paste '{}'", icons::CLIPBOARD, src.name))
                                        .shortcut_text("Ctrl+V"),
                                )
                                .clicked()
                            {
                                pending_action = Some(SourceContextAction::Paste);
                                ui.close_menu();
                            }
                            ui.separator();
                        }
                        if let Some(sel_id) = new_selected {
                            populate_source_context_menu(
                                ui,
                                sel_id,
                                false,
                                SourceColorTag::None,
                                sel_filter_k,
                                sel_is_audio,
                                sel_audio_count,
                                app.copied_source.as_ref().map(|s| s.name.as_str()),
                            &mut pending_action,
                            );
                        } else {
                            ui.menu_button("Add Source", |ui| {
                                populate_add_source_menu(ui, &mut pending_action);
                            });
                        }
                    });
                }
            });
    });

    regions.with_footer(ui, |ui| {
        // Eight buttons share one footer, so the labels have to give way before the row does.
        // Built-in labelled buttons are split into icon-only `+` / `-` keys, while `Properties` and
        // `Filters` keep their text at full width and shorten to `Props` / `Filter` when squeezed.
        // The footer is down to a single control: a bare `+` that opens the add dialog.
        //
        // Everything else that used to live here — Remove, Rename, Filters, Properties, the
        // layer-order arrows, and even the source-type dropdown — is reachable from the row's
        // right-click menu, and the add dialog itself lets the source type be picked. Repeating
        // them in the footer only stole width from the one button the footer is for.
        let add_btn = crate::ui::button_3d::Button3D::new("")
            .icon(icons::PLUS)
            .min_size(Vec2::new(26.0, 24.0))
            .tooltip("Add Source: Open complete layer addition dialog");

        if ui.add(add_btn).clicked() {
            pending_action = Some(SourceContextAction::OpenAddSourceDialog(crate::studio::source::AddSourceType::DisplayCapture));
        }
    });

    if let Some(action) = pending_action {
        execute_source_context_action(app, action);
    }

    Some(card_rect)
}

fn execute_source_context_action(app: &mut NexoraApp, action: SourceContextAction) {
    match action {
        SourceContextAction::OpenAddSourceDialog(st) => {
            app.open_add_source_dialog(st);
        }
        SourceContextAction::AddPowerPoint => {
            app.open_add_source_dialog(crate::studio::source::AddSourceType::PowerPoint);
        }
        SourceContextAction::AddDisplay => {
            app.capture_catalog.refresh_from_system();
            let (mon_id, mon_name, mon_res) = if let Some(d) = app.capture_catalog.displays.first()
            {
                (d.id.clone(), d.name.clone(), (d.width, d.height))
            } else {
                (
                    "MONITOR_0".to_string(),
                    "Primary Monitor (Display 1)".to_string(),
                    (1920, 1080),
                )
            };
            let item = SourceItem::new(
                "Display Capture",
                SourceKind::DisplayCapture {
                    monitor_id: mon_id,
                    monitor_name: mon_name,
                    resolution: mon_res,
                    capture_cursor: true,
                },
            );
            app.studio.preview_scene_mut().add_source(item);
            app.studio.sync_if_linked();
            app.show_source_properties_dialog = true;
        }
        SourceContextAction::AddWindow => {
            app.capture_catalog.refresh_from_system();
            let (hwnd, title, exe) = if let Some(w) = app.capture_catalog.windows.first() {
                (w.hwnd, w.title.clone(), w.executable.clone())
            } else {
                (
                    0x0001_0420,
                    "NEXORA Studio - Visual Studio Code".to_string(),
                    "Code.exe".to_string(),
                )
            };
            let item = SourceItem::new(
                "Window Capture",
                SourceKind::WindowCapture {
                    window_hwnd: hwnd,
                    window_title: title,
                    executable_name: exe,
                    capture_cursor: true,
                },
            );
            app.studio.preview_scene_mut().add_source(item);
            app.studio.sync_if_linked();
            app.show_source_properties_dialog = true;
        }
        SourceContextAction::AddCamera => {
            app.capture_catalog.refresh_from_system();
            let (cid, cname) = if let Some(c) = app.capture_catalog.cameras.iter().find(|c| c.is_online).or_else(|| app.capture_catalog.cameras.first()) {
                (c.id.clone(), c.name.clone())
            } else {
                ("CAM_0".to_string(), "Integrated Camera".to_string())
            };
            let item = SourceItem::new(
                "Video Capture Device",
                SourceKind::WebcamDevice {
                    device_id: cid,
                    device_name: cname,
                    resolution: (1280, 720),
                    corner_radius: 0.0,
                    border_thickness: 0.0,
                    border_color_rgba: [255, 255, 255, 255],
                },
            );
            app.studio.preview_scene_mut().add_source(item);
            app.studio.sync_if_linked();
            app.show_source_properties_dialog = true;
        }
        SourceContextAction::AddImage => {
            let item = SourceItem::new(
                "Image",
                SourceKind::ImageSource {
                    file_path: String::new(),
                    opacity: 1.0,
                },
            );
            app.studio.preview_scene_mut().add_source(item);
            app.studio.sync_if_linked();
            app.show_source_properties_dialog = true;
        }
        SourceContextAction::AddMedia => {
            let item = SourceItem::new(
                "Media Source",
                SourceKind::MediaSource {
                    is_local_file: true,
                    file_path: String::new(),
                    loop_playback: true,
                    restart_on_activate: true,
                    use_hardware_decoding: true,
                    hide_when_ended: false,
                    close_when_inactive: false,
                    speed_percent: 100,
                    volume: 1.0,
                    muted: false,
                    is_playing: true,
                    color_range: crate::studio::source::MediaColorRange::Auto,
                    restart_nonce: 0,
                },
            );
            app.studio.preview_scene_mut().add_source(item);
            app.studio.sync_if_linked();
            app.show_source_properties_dialog = true;
        }
        SourceContextAction::AddGameCapture => {
            app.capture_catalog.refresh_from_system();
            let (hwnd, title, exe) = if let Some(w) = app.capture_catalog.windows.first() {
                (w.hwnd, w.title.clone(), w.executable.clone())
            } else {
                (
                    0x0001_0420,
                    "Fullscreen Game Application".to_string(),
                    "game.exe".to_string(),
                )
            };
            let item = SourceItem::new(
                "Game Capture",
                SourceKind::GameCapture {
                    mode: "Capture specific window".to_string(),
                    window_hwnd: hwnd,
                    window_title: title,
                    executable_name: exe,
                    capture_cursor: true,
                    anti_cheat_hook: true,
                },
            );
            app.studio.preview_scene_mut().add_source(item);
            app.studio.sync_if_linked();
            app.show_source_properties_dialog = true;
        }
        SourceContextAction::AddApplicationAudio => {
            app.capture_catalog.refresh_from_system();
            let (title, exe) = if let Some(w) = app.capture_catalog.windows.first() {
                (w.title.clone(), w.executable.clone())
            } else {
                ("Application Window".to_string(), "app.exe".to_string())
            };
            let item = SourceItem::new(
                "Application Audio Capture",
                SourceKind::ApplicationAudioCapture {
                    window_title: title,
                    executable_name: exe,
                    volume: 1.0,
                    muted: false,
                },
            );
            app.studio.preview_scene_mut().add_source(item);
            app.studio.sync_if_linked();
            app.show_source_properties_dialog = true;
        }
        SourceContextAction::AddAudioInput => {
            app.audio_mixer.refresh_devices_from_system();
            let dev = app.audio_mixer.microphone.selected_device.clone();
            let item = SourceItem::new(
                "Audio Input Capture",
                SourceKind::AudioInputCapture {
                    device_name: dev,
                    volume: 1.0,
                    muted: false,
                },
            );
            app.studio.preview_scene_mut().add_source(item);
            app.studio.sync_if_linked();
            app.show_source_properties_dialog = true;
        }
        SourceContextAction::AddAudioOutput => {
            app.audio_mixer.refresh_devices_from_system();
            let dev = app.audio_mixer.desktop_audio.selected_device.clone();
            let item = SourceItem::new(
                "Audio Output Capture",
                SourceKind::AudioOutputCapture {
                    device_name: dev,
                    volume: 1.0,
                    muted: false,
                },
            );
            app.studio.preview_scene_mut().add_source(item);
            app.studio.sync_if_linked();
            app.show_source_properties_dialog = true;
        }
        SourceContextAction::AddColor => {
            let mut item = SourceItem::new(
                "Color Source",
                SourceKind::ColorSource {
                    color_rgba: [37, 99, 235, 255],
                    width: 1920,
                    height: 1080,
                },
            );
            item.transform.preset_pip_bottom_left();
            app.studio.preview_scene_mut().add_source(item);
            app.studio.sync_if_linked();
            app.show_source_properties_dialog = true;
        }
        SourceContextAction::AddTextGdiPlus => {
            let mut item = SourceItem::new(
                "Text (GDI+)",
                SourceKind::TextGdiPlus {
                    text: "NEXORA Studio Live".to_string(),
                    font_size: 32.0,
                    color_rgba: [255, 255, 255, 255],
                    effect: TextAnimationEffect::Static,
                    scroll_speed: 95.0,
                    loop_animation: true,
                    loop_gap_px: 80.0,
                    banner_style: TvBannerStyle::PlainText,
                    bg_color_rgba: [15, 23, 42, 225],
                    accent_color_rgba: [220, 38, 38, 255],
                    badge_text: "LIVE".to_string(),
                    text_shadow: true,
                },
            );
            item.transform.pos_x = 0.04;
            item.transform.pos_y = 0.05;
            item.transform.scale = 0.35;
            item.transform.scale_y = 0.35;
            app.studio.preview_scene_mut().add_source(item);
            app.studio.sync_if_linked();
            app.show_source_properties_dialog = true;
        }
        SourceContextAction::AddTvTextTemplate(tpl) => {
            let mut item = SourceItem::new(
                tpl.label(),
                SourceKind::TextGdiPlus {
                    text: String::new(),
                    font_size: 24.0,
                    color_rgba: [255, 255, 255, 255],
                    effect: TextAnimationEffect::ScrollLeft,
                    scroll_speed: 100.0,
                    loop_animation: true,
                    loop_gap_px: 80.0,
                    banner_style: TvBannerStyle::BreakingNewsTv,
                    bg_color_rgba: [15, 23, 42, 235],
                    accent_color_rgba: [220, 38, 38, 255],
                    badge_text: "BREAKING NEWS".to_string(),
                    text_shadow: true,
                },
            );
            item.apply_tv_text_template(tpl);
            app.studio.preview_scene_mut().add_source(item);
            app.studio.sync_if_linked();
            app.show_source_properties_dialog = true;
        }
        SourceContextAction::AddBrowser => {
            let mut item = SourceItem::new(
                "Browser",
                SourceKind::BrowserSource {
                    url: "https://obsproject.com/browser-source".to_string(),
                    width: 800,
                    height: 600,
                    fps: 30,
                },
            );
            item.transform.preset_pip_top_right();
            app.studio.preview_scene_mut().add_source(item);
            app.studio.sync_if_linked();
            app.show_source_properties_dialog = true;
        }
        SourceContextAction::AddImageSlideShow => {
            let item = SourceItem::new(
                "Image Slide Show",
                SourceKind::ImageSlideShow {
                    files: Vec::new(),
                    slide_time_ms: 4500,
                    loop_slides: true,
                    transition_effect: SlideTransitionEffect::CrossFade,
                    transition_duration_ms: 650,
                },
            );
            app.studio.preview_scene_mut().add_source(item);
            app.studio.sync_if_linked();
            app.show_source_properties_dialog = true;
        }
        SourceContextAction::AddSceneSource => {
            let first_scene_name = app
                .studio
                .scenes
                .first()
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "Scene 1".to_string());
            let item = SourceItem::new(
                "Scene",
                SourceKind::SceneSource {
                    scene_name: first_scene_name,
                },
            );
            app.studio.preview_scene_mut().add_source(item);
            app.studio.sync_if_linked();
            app.show_source_properties_dialog = true;
        }
        SourceContextAction::MoveUp(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            scene.move_selected_source_up();
            app.studio.sync_if_linked();
        }
        SourceContextAction::MoveDown(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            scene.move_selected_source_down();
            app.studio.sync_if_linked();
        }
        SourceContextAction::MoveTop(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            scene.move_selected_source_to_top();
            app.studio.sync_if_linked();
        }
        SourceContextAction::MoveBottom(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            scene.move_selected_source_to_bottom();
            app.studio.sync_if_linked();
        }
        SourceContextAction::ResetTransform(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            app.reset_source_to_original_size(id);
        }
        SourceContextAction::FitToScreen(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            app.fit_source_to_screen(id);
        }
        SourceContextAction::RotateCw90(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source_mut() {
                src.transform.rotate_cw_90();
            }
            app.studio.sync_if_linked();
        }
        SourceContextAction::RotateCcw90(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source_mut() {
                src.transform.rotate_ccw_90();
            }
            app.studio.sync_if_linked();
        }
        SourceContextAction::Rotate180(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source_mut() {
                src.transform.rotate_180();
            }
            app.studio.sync_if_linked();
        }
        SourceContextAction::FlipHorizontal(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source_mut() {
                src.transform.flip_horizontal();
            }
            app.studio.sync_if_linked();
        }
        SourceContextAction::FlipVertical(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source_mut() {
                src.transform.flip_vertical();
            }
            app.studio.sync_if_linked();
        }
        SourceContextAction::StretchToScreen(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source_mut() {
                src.transform.stretch_to_screen();
            }
            app.studio.sync_if_linked();
        }
        SourceContextAction::CenterVertically(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source_mut() {
                src.transform.center_vertically();
            }
            app.studio.sync_if_linked();
        }
        SourceContextAction::CenterHorizontally(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source_mut() {
                src.transform.center_horizontally();
            }
            app.studio.sync_if_linked();
        }
        SourceContextAction::PipBottomRight(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source_mut() {
                src.transform.preset_pip_bottom_right();
            }
            app.studio.sync_if_linked();
        }
        SourceContextAction::PipBottomLeft(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source_mut() {
                src.transform.preset_pip_bottom_left();
            }
            app.studio.sync_if_linked();
        }
        SourceContextAction::PipTopRight(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source_mut() {
                src.transform.preset_pip_top_right();
            }
            app.studio.sync_if_linked();
        }
        SourceContextAction::CenterOnScreen(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source_mut() {
                src.transform.preset_center();
            }
            app.studio.sync_if_linked();
        }
        SourceContextAction::ToggleLock(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source_mut() {
                src.locked = !src.locked;
            }
            app.studio.sync_if_linked();
        }
        SourceContextAction::SetColor(id, color) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source_mut() {
                src.color_tag = color;
            }
            app.studio.sync_if_linked();
        }
        SourceContextAction::SetFilterKind(id, filter_kind) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source_mut() {
                src.filter.kind = filter_kind;
                src.filter.enabled = filter_kind != crate::studio::source::KeyFilterKind::None;
            }
            app.studio.sync_if_linked();
        }
        SourceContextAction::OpenFilters(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            app.show_source_filters_dialog = true;
        }

        SourceContextAction::AddAudioFilter(id, kind) => {

            let scene = app.studio.preview_scene_mut();

            scene.selected_source_id = Some(id);

            if let Some(src) = scene.selected_source_mut() {

                if src.supports_audio_filters() {

                    src.audio_filters.push(crate::audio::filters::AudioFilter::new(kind));

                }

            }

            app.show_audio_filters_dialog = true;

            app.studio.sync_if_linked();

            app.save_all_state();

        }

        SourceContextAction::ApplyCleanMicPreset(id) => {

            let scene = app.studio.preview_scene_mut();

            scene.selected_source_id = Some(id);

            if let Some(src) = scene.selected_source_mut() {

                if src.supports_audio_filters() {

                    // Replaces rather than appends, so running the preset twice is idempotent.

                    src.audio_filters = crate::audio::filters::clean_mic_preset();

                }

            }

            app.show_audio_filters_dialog = true;

            app.studio.sync_if_linked();

            app.save_all_state();

        }

        SourceContextAction::OpenAudioFilters(id) => {

            let scene = app.studio.preview_scene_mut();

            scene.selected_source_id = Some(id);

            app.show_audio_filters_dialog = true;

        }        SourceContextAction::Copy(id) => {
            app.copy_source_by_id(id);
        }
        SourceContextAction::Paste => {
            app.paste_copied_source();
        }
        SourceContextAction::Duplicate(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            scene.duplicate_selected_source();
            app.studio.sync_if_linked();
        }
        SourceContextAction::Remove(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            scene.remove_selected_source();
            app.studio.sync_if_linked();
        }
        SourceContextAction::Rename(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source() {
                app.rename_source_buffer = src.name.clone();
                app.show_rename_source_dialog = true;
            }
        }
        SourceContextAction::OpenProperties(id) => {
            app.capture_catalog.refresh_from_system();
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            app.show_source_properties_dialog = true;
        }
    }
}

fn truncate_str(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        text.to_string()
    } else {
        let keep = max_chars.saturating_sub(1).max(8);
        let prefix: String = text.chars().take(keep).collect();
        format!("{}…", prefix)
    }
}

/// Rough advance width of one average glyph at the 12.5 pt button font, in px.
///
/// Only used to pick between two pre-written labels, so an estimate is enough — being off by a
/// few px just moves the switchover point by one character. Measuring the real galley per button
/// per frame would mean laying out every candidate label every frame, which is the cost this whole
/// helper exists to avoid.
const DOCK_BTN_CHAR_W: f32 = 6.6;

/// Horizontal space a dock footer button spends on things that are not its label.
///
/// `Button3D` centres `"{icon}  {text}"` in a `min_size` box, and egui adds `button_padding` on
/// both sides of the allocated rect. Icon + gap + padding is roughly this much; the rest is label.
const DOCK_BTN_CHROME_W: f32 = 34.0;

/// Picks a button label that still fits the footer it is drawn in.
///
/// The four dock panels can each be dragged from ~150 px to ~65% of the window, so a footer that
/// has room for "Properties" at one width has room for only "Props" at another. Rather than let
/// `Button3D` clip the overflow away — which is what made the buttons look broken — each label has
/// a hand-written short form that keeps its meaning, and the longest one that fits the measured
/// budget wins.
///
/// Returns the **longest** candidate that fits, falling back to the shortest so a very narrow panel
/// still shows something recognisable (and keeps its tooltip as the full explanation).
fn dock_button_label(available_w: f32, long: &'static str, short: &'static str) -> &'static str {
    if dock_label_fits(available_w, long) {
        long
    } else {
        short
    }
}

/// Whether a label plus its icon and padding fits inside `available_w`.
fn dock_label_fits(available_w: f32, label: &str) -> bool {
    let label_w = label.chars().count() as f32 * DOCK_BTN_CHAR_W;
    label_w + DOCK_BTN_CHROME_W <= available_w
}

/// Rough advance width of one average glyph at `text_size` pt, in px.
///
/// Used to decide whether a header hint still fits; like [`DOCK_BTN_CHAR_W`] this is an estimate,
/// which is fine because the decision is only "does this roughly fit".
fn dock_text_width(text: &str, text_size: f32) -> f32 {
    text.chars().count() as f32 * text_size * 0.53
}

/// Draws a right-aligned header hint (e.g. `48 kHz`, `Right-click to edit`), dropping it when the
/// panel is too narrow to show it beside the title.
///
/// The hints used to be drawn unconditionally, which in a squeezed panel made them collide with the
/// title and the minimize button. Suppressing them is the right trade: a hint is decoration, the
/// title and the controls are not.
fn paint_header_hint(
    ui: &mut egui::Ui,
    p: &ThemePalette,
    text: &str,
    text_size: f32,
    monospace: bool,
) {
    let reserved_for_title_and_controls = 150.0;
    if ui.available_width() < dock_text_width(text, text_size) + reserved_for_title_and_controls {
        return;
    }
    let mut rich = RichText::new(text).size(text_size).color(p.text_muted);
    if monospace {
        rich = rich.monospace();
    }
    ui.label(rich);
}

/// Truncates `text` so that it occupies at most `max_w` pixels.
///
/// Unlike [`truncate_str`], which limits by character count, this one limits by the space that is
/// actually available. The scene name sits next to the status badges, and how much room is left
/// depends on the panel width and on whether the badges had to shrink to their icon-only form.
fn truncate_to_width(text: &str, max_w: f32, char_w: f32) -> String {
    if max_w <= 0.0 {
        return String::new();
    }
    let max_chars = (max_w / char_w).floor().max(0.0) as usize;
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    if max_chars <= 1 {
        return "…".to_string();
    }
    let prefix: String = text.chars().take(max_chars - 1).collect();
    format!("{}…", prefix)
}

/// One channel of the audio mixer, normalised across the three kinds of entry the panel holds:
/// the two global buses, one bus per audio source, and media playback.
///
/// Normalising them is what makes the alternative layouts possible: one renderer can draw any
/// channel, so the panel does not need a separate implementation per kind of entry.
struct MixerChannel<'a> {
    /// Display name of the channel.
    name: String,
    /// Icon on the mute button while the channel is unmuted.
    icon: &'static str,
    /// Icon on the mute button while the channel is muted.
    icon_off: &'static str,
    /// Short tags drawn next to the name: the direction of the bus and whether it is live.
    ///
    /// The strip layout shows both, and so do the compact layouts — but the compact ones drop
    /// them first when the row runs out of room.
    tags: [Option<(&'static str, Color32)>; 2],
    /// Mute state, borrowed so a layout renderer can toggle it.
    muted: &'a mut bool,
    /// Volume in 0..=1, borrowed so a layout renderer can drive the fader.
    volume: &'a mut f32,
    /// Playback state of a media source, borrowed so the transport button can toggle it.
    is_playing: Option<&'a mut bool>,
    /// Live signal level in 0..=1.
    level: f32,
    /// Held peak in 0..=1.
    peak_hold: f32,
    /// Set when a renderer changed the mute state or the volume.
    changed: bool,
    /// Set when a renderer changed a media source's playback state.
    transport_changed: bool,
}

impl MixerChannel<'_> {
    /// The percentage of the channel volume, formatted the way the strip layout shows it.
    fn percent_readout(&self) -> String {
        let pct = if *self.muted {
            0.0_f32
        } else {
            (*self.volume).clamp(0.0, 1.0) * 100.0
        };
        format!("{:>3.0}%", pct)
    }

    /// The dB readout for this channel, formatted the way the strip layout shows it.
    fn db_readout(&self) -> String {
        let muted = *self.muted;
        let volume = *self.volume;
        let db = if muted || volume <= 0.0001 {
            -60.0_f32
        } else {
            (20.0 * volume.log10()).max(-60.0)
        };
        let pct = if muted {
            0
        } else {
            (volume.clamp(0.0, 1.0) * 100.0).round() as i32
        };
        if db <= -59.5 {
            format!("{:>3}% (-∞ dB)", pct)
        } else {
            format!("{:>3}% ({:+.1}dB)", pct, db)
        }
    }
}

/// Live level of every audio-capturing source in the preview and program scenes, keyed by id.
///
/// Read before the mixer is borrowed mutably: `source_level` needs the whole mixer, which cannot
/// be borrowed at the same time as the individual channels it would hand out.
fn mixer_source_levels(
    app: &NexoraApp,
) -> std::collections::HashMap<uuid::Uuid, crate::audio::SourceLevel> {
    let mut levels = std::collections::HashMap::new();
    let preview_id = app.studio.preview_scene_id;
    let program_id = app.studio.program_scene_id;
    for scene in &app.studio.scenes {
        if scene.id != program_id && scene.id != preview_id {
            continue;
        }
        for src in &scene.sources {
            let is_capture = matches!(
                src.kind,
                SourceKind::AudioInputCapture { .. }
                    | SourceKind::AudioOutputCapture { .. }
                    | SourceKind::ApplicationAudioCapture { .. }
            );
            if !is_capture {
                continue;
            }
            let bus_id = NexoraApp::source_bus_id(src.id);
            levels.insert(src.id, app.audio_mixer.source_level(&bus_id));
        }
    }
    levels
}

/// Collects every channel the mixer should show, in display order.
///
/// The order is the two global buses first, then one bus per audio-capturing source, then media
/// playback sources. All three layouts use this order, so switching between them never reshuffles
/// the channels under the user's fingers.
fn collect_mixer_channels<'a>(app: &'a mut NexoraApp) -> Vec<MixerChannel<'a>> {
    let p = ThemePalette::for_mode(app.settings.theme);

    // Both sets are read up front, before the mixer is borrowed mutably: `source_level` needs the
    // whole mixer, which cannot be borrowed at the same time as the channels themselves.
    let levels = mixer_source_levels(app);
    let program_audio_ids: std::collections::HashSet<uuid::Uuid> = app
        .studio
        .program_scene()
        .sources
        .iter()
        .filter(|s| s.visible)
        .filter(|s| {
            matches!(
                s.kind,
                SourceKind::AudioInputCapture { .. }
                    | SourceKind::AudioOutputCapture { .. }
                    | SourceKind::ApplicationAudioCapture { .. }
            )
        })
        .map(|s| s.id)
        .collect();
    let program_media_ids: std::collections::HashSet<uuid::Uuid> = app
        .studio
        .program_scene()
        .sources
        .iter()
        .filter(|s| s.visible && matches!(s.kind, SourceKind::MediaSource { .. }))
        .map(|s| s.id)
        .collect();

    let preview_id = app.studio.preview_scene_id;
    let program_id = app.studio.program_scene_id;

    let NexoraApp {
        audio_mixer,
        studio,
        ..
    } = app;

    let mut channels: Vec<MixerChannel<'a>> = Vec::new();

    // --- The two global buses ---
    // The name, level and peak are read first: taking a mutable borrow of one field of a channel
    // would otherwise collide with the immutable read of the whole channel.
    let (desk_name, desk_level, desk_hold) = {
        let ch = &audio_mixer.desktop_audio;
        (ch.label.clone(), ch.effective_level(), ch.peak_hold)
    };
    channels.push(MixerChannel {
        name: desk_name,
        icon: icons::VOLUME,
        icon_off: icons::VOLUME_OFF,
        tags: [None, None],
        muted: &mut audio_mixer.desktop_audio.muted,
        volume: &mut audio_mixer.desktop_audio.volume,
        is_playing: None,
        level: desk_level,
        peak_hold: desk_hold,
        changed: false,
        transport_changed: false,
    });

    let (mic_name, mic_level, mic_hold) = {
        let ch = &audio_mixer.microphone;
        (ch.label.clone(), ch.effective_level(), ch.peak_hold)
    };
    channels.push(MixerChannel {
        name: mic_name,
        icon: icons::MICROPHONE,
        icon_off: icons::MICROPHONE_OFF,
        tags: [None, None],
        muted: &mut audio_mixer.microphone.muted,
        volume: &mut audio_mixer.microphone.volume,
        is_playing: None,
        level: mic_level,
        peak_hold: mic_hold,
        changed: false,
        transport_changed: false,
    });

    // --- One bus per audio-capturing source, then media playback ---
    let mut seen_ids = std::collections::HashSet::new();
    for scene in &mut studio.scenes {
        if scene.id != program_id && scene.id != preview_id {
            continue;
        }
        for src in &mut scene.sources {
            let src_id = src.id;
            if !seen_ids.insert(src_id) {
                continue;
            }
            let name = src.name.clone();
            let src_level = levels.get(&src_id).copied().unwrap_or_default();
            // Whether the source is live on Program decides the colour of the `PGM` / `PRV` tag.
            let live_tag = |is_live: bool| {
                if is_live {
                    ("PGM", p.accent_program)
                } else {
                    ("PRV", p.accent_preview)
                }
            };

            // `kind` is bound to a local first: the mutable borrows of the fields below have to
            // outlive the `match` statement they are created in.
            let kind = &mut src.kind;
            match kind {
                SourceKind::AudioInputCapture { volume, muted, .. } => {
                    channels.push(MixerChannel {
                        name,
                        icon: icons::MICROPHONE,
                        icon_off: icons::MICROPHONE_OFF,
                        tags: [
                            Some(("IN", p.text_muted)),
                            Some(live_tag(program_audio_ids.contains(&src_id))),
                        ],
                        muted,
                        volume,
                        is_playing: None,
                        level: src_level.current_peak,
                        peak_hold: src_level.peak_hold,
                        changed: false,
                        transport_changed: false,
                    });
                }
                SourceKind::AudioOutputCapture { volume, muted, .. } => {
                    channels.push(MixerChannel {
                        name,
                        icon: icons::VOLUME,
                        icon_off: icons::VOLUME_OFF,
                        tags: [
                            Some(("OUT", p.text_muted)),
                            Some(live_tag(program_audio_ids.contains(&src_id))),
                        ],
                        muted,
                        volume,
                        is_playing: None,
                        level: src_level.current_peak,
                        peak_hold: src_level.peak_hold,
                        changed: false,
                        transport_changed: false,
                    });
                }
                SourceKind::ApplicationAudioCapture { volume, muted, .. } => {
                    channels.push(MixerChannel {
                        name,
                        icon: icons::VOLUME,
                        icon_off: icons::VOLUME_OFF,
                        tags: [
                            Some(("APP", p.text_muted)),
                            Some(live_tag(program_audio_ids.contains(&src_id))),
                        ],
                        muted,
                        volume,
                        is_playing: None,
                        level: src_level.current_peak,
                        peak_hold: src_level.peak_hold,
                        changed: false,
                        transport_changed: false,
                    });
                }
                SourceKind::MediaSource {
                    volume,
                    muted,
                    is_playing,
                    ..
                } => {
                    channels.push(MixerChannel {
                        name,
                        icon: icons::VOLUME,
                        icon_off: icons::VOLUME_OFF,
                        tags: [
                            None,
                            Some(live_tag(program_media_ids.contains(&src_id))),
                        ],
                        muted,
                        volume,
                        is_playing: Some(is_playing),
                        level: 0.0,
                        peak_hold: 0.0,
                        changed: false,
                        transport_changed: false,
                    });
                }
                _ => continue,
            }
        }
    }

    channels
}

/// Draws a thin live-level line under a mixer channel.
///
/// The strip layout uses a 7 px meter; in the denser layouts that would cost as much vertical
/// space as it saves, so the level is reduced to a 3 px line.
fn paint_mixer_thin_meter(
    ui: &mut egui::Ui,
    p: &ThemePalette,
    level: f32,
    peak_hold: f32,
    w: f32,
) {
    let h = 3.0_f32;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w.max(40.0), h), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(1), p.bg_canvas_well);

    if level > 0.005 {
        let level = level.clamp(0.0, 1.0);
        let fill = if level > 0.88 {
            p.meter_clip
        } else if level > 0.70 {
            p.meter_warn
        } else {
            p.meter_ok
        };
        painter.rect_filled(
            Rect::from_min_max(
                rect.min,
                Pos2::new(rect.min.x + rect.width() * level, rect.max.y),
            ),
            CornerRadius::same(1),
            fill,
        );
    }

    if peak_hold > 0.008 {
        let x = rect.min.x + rect.width() * peak_hold.clamp(0.0, 1.0);
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(x - 1.0, rect.min.y),
                Pos2::new(x + 1.0, rect.max.y),
            ),
            CornerRadius::same(1),
            Color32::WHITE,
        );
    }
}

/// The column geometry shared by every row of the compact mixer list.
///
/// The widths are measured from the text that is actually drawn and every row uses the same
/// numbers, so the faders start at the same x in every row — no matter how long the channel names
/// are, and no matter which rows happen to carry a transport button or a tag.
struct MixerRowLayout {
    /// Width of the whole row.
    row_w: f32,
    /// Width of the mute button.
    mute_w: f32,
    /// Width of the transport button slot; zero when no channel in the list can be paused.
    transport_w: f32,
    /// Width reserved for the channel name.
    name_w: f32,
    /// Width of one tag slot.
    tag_w: f32,
    /// Number of tag slots reserved in every row.
    tag_slots: usize,
    /// Width of the fader.
    fader_w: f32,
    /// Width of the percentage readout.
    readout_w: f32,
    /// Horizontal gap between two columns.
    gap: f32,
}

impl MixerRowLayout {
    /// The number of columns a row is built from.
    fn cols(&self) -> f32 {
        4.0 + self.tag_slots as f32 + if self.transport_w > 0.0 { 1.0 } else { 0.0 }
    }

    /// The width of the columns that do not change: mute, transport, tags and readout.
    ///
    /// What is left of the row after these is shared between the channel name and the fader.
    fn fixed_w(&self) -> f32 {
        self.mute_w
            + self.transport_w
            + self.tag_slots as f32 * self.tag_w
            + self.readout_w
            + (self.cols() - 1.0) * self.gap
    }

    /// The width the columns of one row take up, gaps included.
    ///
    /// This is the width the row has to fill exactly: any other value would either leave a gap or
    /// push the fader out of the panel.
    fn total_w(&self) -> f32 {
        self.fixed_w() + self.name_w + self.fader_w
    }
}

/// Works out the column widths of the compact mixer rows.
///
/// The readout and the tags are measured from the text they will show, so a number can never be
/// clipped and a tag can never overrun its slot. What is left over is split between the channel
/// name and the fader — the fader first, because a row without a usable fader is worthless while
/// a shortened name is still readable.
fn mixer_row_layout(
    ui: &egui::Ui,
    p: &ThemePalette,
    channels: &[MixerChannel<'_>],
    row_w: f32,
) -> MixerRowLayout {
    let gap = ui.spacing().item_spacing.x;

    // The readout is measured from the widest number it can show.
    let readout_w = ui
        .painter()
        .layout_no_wrap(
            "100%".to_owned(),
            egui::FontId::monospace(MIXER_READOUT_FONT_SIZE),
            p.text_secondary,
        )
        .size()
        .x
        + 4.0;

    // The tag slot is as wide as the widest tag needs it, and the same in every row, so the
    // faders line up.
    let tag_font = egui::FontId::new(MIXER_TAG_FONT_SIZE, egui::FontFamily::Proportional);
    let mut tag_w = 0.0_f32;
    let mut tag_slots = 0_usize;
    for ch in channels {
        tag_slots = tag_slots.max(ch.tags.iter().filter(|t| t.is_some()).count());
        for (text, color) in ch.tags.iter().flatten() {
            let w = ui
                .painter()
                .layout_no_wrap((*text).to_owned(), tag_font.clone(), *color)
                .size()
                .x;
            tag_w = tag_w.max(w);
        }
    }
    if tag_slots > 0 {
        tag_w += 8.0;
    }
    // In a narrow panel the tags go entirely — for every row at once, because a tag that is
    // sometimes there and sometimes not would break the alignment.
    if row_w < MIXER_TAGS_MIN_ROW_W {
        tag_slots = 0;
        tag_w = 0.0;
    }

    // The transport button only exists for media sources, but its slot is reserved in every row
    // so a media channel does not push its fader out of line with the others.
    let transport_w = if channels.iter().any(|c| c.is_playing.is_some()) {
        MIXER_TRANSPORT_W
    } else {
        0.0
    };

    // Columns: mute | [transport] | name | [tags] | fader | readout.
    //
    // The name and the fader share what is left over. The fader comes first: a row without a
    // usable fader is worthless, while a shortened name is still readable.
    let mut lay = MixerRowLayout {
        row_w,
        mute_w: MIXER_MUTE_W,
        transport_w,
        name_w: 0.0,
        tag_w,
        tag_slots,
        fader_w: 0.0,
        readout_w,
        gap,
    };

    let avail = (row_w - lay.fixed_w()).max(0.0);
    lay.name_w = (avail * 0.38).clamp(MIXER_NAME_MIN_W.min(avail), avail);
    lay.fader_w = avail - lay.name_w;
    if lay.fader_w < MIXER_FADER_MIN_W {
        lay.name_w = (lay.name_w - (MIXER_FADER_MIN_W - lay.fader_w)).max(0.0);
        lay.fader_w = avail - lay.name_w;
    }
    lay.fader_w = lay.fader_w.max(MIXER_FADER_MIN_W.min(avail));

    // The columns have to fill the row exactly. Should the arithmetic ever come out wider than
    // the panel, the name gives way — a fader that is pushed out of sight is worse than a name
    // that is shortened.
    let overflow = lay.total_w() - lay.row_w;
    if overflow > 0.0 {
        lay.name_w = (lay.name_w - overflow).max(0.0);
    }

    lay
}

/// Draws one channel as a compact horizontal line: mute, name, tag, fader and level readout.
///
/// This is the dense layout — roughly a third of the height of a full strip — for when the mixer
/// holds more channels than the panel can show comfortably.
fn render_mixer_channel_row(
    ui: &mut egui::Ui,
    p: &ThemePalette,
    ch: &mut MixerChannel<'_>,
    lay: &MixerRowLayout,
) {
    // The fader's tooltip carries the exact dB value, so the number in the row can stay short.
    let readout = ch.db_readout();
    let percent = ch.percent_readout();
    let name_font = egui::FontId::new(MIXER_NAME_FONT_SIZE, egui::FontFamily::Proportional);
    let tag_font = egui::FontId::new(MIXER_TAG_FONT_SIZE, egui::FontFamily::Proportional);
    let readout_font = egui::FontId::monospace(MIXER_READOUT_FONT_SIZE);

    ui.allocate_ui_with_layout(
        Vec2::new(lay.row_w, MIXER_ROW_H),
        Layout::left_to_right(Align::Center),
        |ui| {
            // --- Mute ---
            let icon = if *ch.muted { ch.icon_off } else { ch.icon };
            let color = if *ch.muted { p.rec_red } else { p.text_secondary };
            if paint_icon_action_btn_sized(
                ui,
                icon,
                Some(color),
                p,
                if *ch.muted {
                    "Unmute channel"
                } else {
                    "Mute channel"
                },
                Vec2::new(lay.mute_w, MIXER_ROW_H),
            ) {
                *ch.muted = !*ch.muted;
                ch.changed = true;
            }

            // --- Transport (media sources only) ---
            //
            // The slot is reserved in every row, so a media channel does not push its fader out of
            // line with the faders of the other rows.
            if lay.transport_w > 0.0 {
                if let Some(is_playing) = ch.is_playing.as_deref_mut() {
                    let play_icon = if *is_playing {
                        icons::PLAYER_PAUSE
                    } else {
                        icons::PLAYER_PLAY
                    };
                    if paint_icon_action_btn_sized(
                        ui,
                        play_icon,
                        None,
                        p,
                        "Pause / Play Media",
                        Vec2::new(lay.transport_w, MIXER_ROW_H),
                    ) {
                        *is_playing = !*is_playing;
                        ch.transport_changed = true;
                    }
                } else {
                    ui.allocate_exact_size(Vec2::new(lay.transport_w, MIXER_ROW_H), Sense::hover());
                }
            }

            // --- Name ---
            //
            // The column is reserved first and the name is painted into it, so the width of the
            // name never depends on how long the name happens to be.
            let (name_rect, _) =
                ui.allocate_exact_size(Vec2::new(lay.name_w, MIXER_ROW_H), Sense::hover());
            paint_single_line_text(
                ui,
                name_rect,
                &ch.name,
                name_font,
                p.text_primary,
                egui::Align::Min,
                true,
            );

            // --- Tags ---
            //
            // Every row reserves the same number of tag slots, so the fader always starts at the
            // same x.
            for slot in 0..lay.tag_slots {
                let (tag_rect, _) =
                    ui.allocate_exact_size(Vec2::new(lay.tag_w, MIXER_ROW_H), Sense::hover());
                if let Some((text, color)) = ch.tags.get(slot).copied().flatten() {
                    paint_single_line_text(
                        ui,
                        tag_rect,
                        text,
                        tag_font.clone(),
                        color,
                        egui::Align::Center,
                        false,
                    );
                }
            }

            // --- Fader ---
            ui.scope(|ui| {
                ui.spacing_mut().slider_width = lay.fader_w;
                let changed = ui
                    .add_enabled(
                        !*ch.muted,
                        egui::Slider::new(&mut *ch.volume, 0.0..=1.0)
                            .show_value(false)
                            .trailing_fill(true),
                    )
                    .on_hover_text(readout.clone())
                    .changed();
                if changed {
                    ch.changed = true;
                }
            });

            // --- Readout ---
            //
            // The column is reserved like the others, and the number is right aligned inside it,
            // so it ends flush with the right edge of the row.
            let (readout_rect, _) =
                ui.allocate_exact_size(Vec2::new(lay.readout_w, MIXER_ROW_H), Sense::hover());
            paint_single_line_text(
                ui,
                readout_rect,
                &percent,
                readout_font,
                p.text_secondary,
                egui::Align::Max,
                false,
            );
        },
    );

    paint_mixer_thin_meter(ui, p, ch.level, ch.peak_hold, lay.row_w);
}

/// Draws one channel as an OBS-style mixer column.
///
/// The column follows the layout of a mixer channel in OBS Studio, from top to bottom: the name of
/// the channel, the dB readout, the upright fader with a level meter on its left and a dB scale
/// printed on its right, and the mute button at the bottom.
///
/// The fader works in dB rather than in raw volume — 0 dB at the top of the travel and -60 dB at
/// the bottom — so the printed scale stays true while the handle moves.
fn render_mixer_channel_column(
    ui: &mut egui::Ui,
    p: &ThemePalette,
    ch: &mut MixerChannel<'_>,
    col_w: f32,
    col_h: f32,
) {
    let name_h = 20.0_f32;
    let db_h = 15.0_f32;
    let mute_h = 24.0_f32;
    let meter_w = 10.0_f32;
    let scale_w = 24.0_f32;
    let inner_w = (col_w - 4.0).max(40.0);

    // The dB readout sits under the name, the way OBS Studio prints it.
    let mut db = mixer_volume_to_db(*ch.volume);
    let db_text = if *ch.muted || db <= -59.5 {
        "-∞ dB".to_owned()
    } else if db >= -0.05 {
        "0.0 dB".to_owned()
    } else {
        format!("{db:.1} dB")
    };

    ui.allocate_ui_with_layout(
        Vec2::new(col_w, col_h),
        Layout::top_down(Align::Min),
        |ui| {
            // --- Name ---
            let (name_rect, _) = ui.allocate_exact_size(Vec2::new(inner_w, name_h), Sense::hover());
            paint_single_line_text(
                ui,
                name_rect,
                &ch.name,
                egui::FontId::new(10.5, egui::FontFamily::Proportional),
                p.text_primary,
                egui::Align::Min,
                true,
            );

            // --- dB readout ---
            let (db_rect, _) = ui.allocate_exact_size(Vec2::new(inner_w, db_h), Sense::hover());
            paint_single_line_text(
                ui,
                db_rect,
                &db_text,
                egui::FontId::monospace(10.0),
                p.text_secondary,
                egui::Align::Max,
                false,
            );

            // --- Fader, meter and scale ---
            //
            // The fader takes whatever room the fixed parts leave.
            //
            // For a *vertical* slider egui reads `slider_width` as its height, not its width —
            // the width is the handle thickness. So the height goes into `slider_width` here.
            let fader_h = (col_h
                - name_h
                - db_h
                - mute_h
                - ui.spacing().item_spacing.y * 3.0)
                .max(60.0);

            ui.allocate_ui_with_layout(
                Vec2::new(inner_w, fader_h),
                Layout::left_to_right(Align::Min),
                |ui| {
                    // The level meter stands to the left of the fader, like in OBS Studio.
                    let (meter_rect, _) =
                        ui.allocate_exact_size(Vec2::new(meter_w, fader_h), Sense::hover());
                    paint_mixer_vertical_meter(ui, p, meter_rect, ch.level, ch.peak_hold);

                    // The fader itself.
                    let slider = ui
                        .scope(|ui| {
                            ui.spacing_mut().slider_width = fader_h;
                            ui.add_enabled(
                                !*ch.muted,
                                egui::Slider::new(&mut db, MIXER_SCALE_MIN_DB..=0.0)
                                    .vertical()
                                    .show_value(false)
                                    .trailing_fill(true),
                            )
                        })
                        .inner;
                    if slider.changed() {
                        *ch.volume = mixer_db_to_volume(db);
                        ch.changed = true;
                    }

                    // The dB scale is printed to the right of the fader, on the travel of its
                    // handle, so the numbers stay true.
                    let (scale_rect, _) =
                        ui.allocate_exact_size(Vec2::new(scale_w, fader_h), Sense::hover());
                    paint_mixer_db_scale(ui, p, scale_rect, slider.rect);
                },
            );

            // --- Mute ---
            let icon = if *ch.muted { ch.icon_off } else { ch.icon };
            let color = if *ch.muted { p.rec_red } else { p.text_secondary };
            if paint_icon_action_btn_sized(
                ui,
                icon,
                Some(color),
                p,
                if *ch.muted {
                    "Unmute channel"
                } else {
                    "Mute channel"
                },
                Vec2::new(26.0, mute_h - 2.0),
            ) {
                *ch.muted = !*ch.muted;
                ch.changed = true;
            }
        },
    );
}

/// Paints the vertical level meter that stands beside the fader.
///
/// The meter is a vertical bar: the louder the signal, the taller the bar. It keeps the same
/// colours as the thin meter of the other layouts — green, yellow at the warning level and red
/// when the signal clips.
fn paint_mixer_vertical_meter(
    ui: &egui::Ui,
    p: &ThemePalette,
    rect: Rect,
    level: f32,
    peak_hold: f32,
) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(1), p.bg_canvas_well);
    painter.rect_stroke(
        rect,
        CornerRadius::same(1),
        Stroke::new(1.0_f32, p.border_subtle),
        egui::StrokeKind::Inside,
    );

    if level > 0.005 {
        let level = level.clamp(0.0, 1.0);
        let fill = if level > 0.88 {
            p.meter_clip
        } else if level > 0.70 {
            p.meter_warn
        } else {
            p.meter_ok
        };
        let h = rect.height() * level;
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(rect.min.x, rect.max.y - h), rect.max),
            CornerRadius::same(1),
            fill,
        );
    }

    if peak_hold > 0.008 {
        let y = rect.max.y - rect.height() * peak_hold.clamp(0.0, 1.0);
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(rect.min.x, y - 1.0),
                Pos2::new(rect.max.x, y + 1.0),
            ),
            CornerRadius::ZERO,
            Color32::WHITE,
        );
    }
}

/// Paints the dB scale printed next to the fader.
///
/// The numbers sit on the travel of the fader handle, so the value under the handle is the value
/// the scale shows. When the column is too short for a label every 6 dB the scale gets coarser
/// instead of overlapping.
fn paint_mixer_db_scale(ui: &egui::Ui, p: &ThemePalette, rect: Rect, fader_rect: Rect) {
    const STEPS: [i32; 5] = [6, 12, 20, 30, 60];

    let label_h = 11.0_f32;
    let slots = (rect.height() / label_h).floor() as i32;
    let Some(step) = STEPS
        .iter()
        .copied()
        .find(|s| (60 / s) + 1 <= slots)
    else {
        return;
    };

    // The handle of a vertical slider travels between `top + radius` and `bottom - radius`,
    // where the radius is a quarter of the fader's thickness.
    let thickness = fader_rect.width();
    let inset = thickness / 2.5;
    let travel_h = (fader_rect.height() - inset * 2.0).max(1.0);

    let font = egui::FontId::monospace(8.5);
    for tick in (0..=(60 / step)).rev() {
        let value = -tick as f32 * step as f32;
        // 0 dB sits at the top of the travel, -60 dB at the bottom.
        let norm = (value - MIXER_SCALE_MIN_DB) / -MIXER_SCALE_MIN_DB;
        let y = fader_rect.top() + inset + (1.0 - norm) * travel_h;

        let text = format!("{value:.0}");
        let galley = ui
            .painter()
            .layout_no_wrap(text, font.clone(), p.text_muted);
        let pos = Pos2::new(rect.left(), y - galley.size().y * 0.5);
        ui.painter().galley(pos, galley, p.text_muted);
    }
}

/// Renders the mixer in the dense list layout: one compact line per channel.
fn render_mixer_list(
    ui: &mut egui::Ui,
    app: &mut NexoraApp,
    p: &ThemePalette,
    body_size: Vec2,
) {
    let row_w = (body_size.x - 10.0).max(120.0);
    let mut changed = false;
    let mut transport_changed = false;

    {
        let mut channels = collect_mixer_channels(app);

        // The column geometry is worked out once and shared by every row, so the faders line up
        // down the panel.
        let layout = mixer_row_layout(ui, p, &channels, row_w);

        egui::ScrollArea::vertical()
            .id_salt("audio_mixer_list_scroll")
            .max_height(body_size.y)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for ch in &mut channels {
                    render_mixer_channel_row(ui, p, ch, &layout);
                    changed |= ch.changed;
                    transport_changed |= ch.transport_changed;
                }
            });
    }

    if transport_changed {
        app.studio.sync_media_transport_to_program();
    }
    if changed {
        app.studio.sync_if_linked();
    }
}

/// Renders the mixer in the column layout: the channels stand upright and run to the right.
fn render_mixer_columns(
    ui: &mut egui::Ui,
    app: &mut NexoraApp,
    p: &ThemePalette,
    body_size: Vec2,
) {
    let col_w = MIXER_COL_W;
    let mut changed = false;
    let mut transport_changed = false;

    {
        let mut channels = collect_mixer_channels(app);
        egui::ScrollArea::horizontal()
            .id_salt("audio_mixer_columns_scroll")
            .max_width(body_size.x)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    for ch in &mut channels {
                        render_mixer_channel_column(ui, p, ch, col_w, body_size.y);
                        changed |= ch.changed;
                        transport_changed |= ch.transport_changed;
                    }
                });
            });
    }

    if transport_changed {
        app.studio.sync_media_transport_to_program();
    }
    if changed {
        app.studio.sync_if_linked();
    }
}

/// Returns the card's outer rect so the caller can attach a width splitter to its right edge.
fn render_audio_mixer_panel(ui: &mut egui::Ui, app: &mut NexoraApp, size: Vec2) -> Option<Rect> {
    let p = ThemePalette::for_mode(app.settings.theme);
    let (regions, card_rect) = paint_and_split_dock_card(ui, &p, size);

    regions.with_header(ui, |ui| {
        ui.label(
            RichText::new("AUDIO MIXER")
                .size(13.5)
                .strong()
                .color(p.text_primary),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("—").small())
                        .min_size(Vec2::new(22.0, 20.0)),
                )
                .on_hover_text("Minimize Audio Mixer panel")
                .clicked()
            {
                app.settings.panels.show_audio_mixer = false;
            }
            ui.add_space(3.0);

            // Layout picker. The mixer holds three very different kinds of entry, and how many
            // of them fit depends on the panel width — so the arrangement is the user's call:
            // full strips, a dense list, or upright columns standing side by side.
            let mode = app.settings.audio_mixer_view_mode;
            let (mode_icon, mode_label) = match mode {
                crate::settings::AudioMixerViewMode::Strip => (icons::LAYOUT_GRID, "Strips"),
                crate::settings::AudioMixerViewMode::List => (icons::LIST, "List"),
                crate::settings::AudioMixerViewMode::Columns => {
                    (icons::LAYOUT_COLUMNS, "Columns")
                }
            };
            let mut chosen: Option<crate::settings::AudioMixerViewMode> = None;
            ui.menu_button(
                RichText::new(mode_icon).size(13.0).color(p.accent_preview),
                |ui| {
                    ui.set_min_width(150.0);
                    for (m, label, icon) in [
                        (
                            crate::settings::AudioMixerViewMode::Strip,
                            "Strips",
                            icons::LAYOUT_GRID,
                        ),
                        (crate::settings::AudioMixerViewMode::List, "List", icons::LIST),
                        (
                            crate::settings::AudioMixerViewMode::Columns,
                            "Columns",
                            icons::LAYOUT_COLUMNS,
                        ),
                    ] {
                        if ui
                            .selectable_label(mode == m, format!("{icon} {label}"))
                            .clicked()
                        {
                            chosen = Some(m);
                            ui.close_menu();
                        }
                    }
                },
            )
            .response
            .on_hover_text(format!("Mixer layout: {mode_label}"));
            if let Some(m) = chosen {
                app.settings.audio_mixer_view_mode = m;
            }

            paint_header_hint(ui, &p, "48 kHz", 11.0, true);
        });
    });

    regions.with_body(ui, |ui, body_size| {
        // `Strip` keeps the historical full-width blocks; the two denser arrangements are drawn
        // by their own functions.
        let mode = app.settings.audio_mixer_view_mode;
        match mode {
            crate::settings::AudioMixerViewMode::List => {
                render_mixer_list(ui, app, &p, body_size);
                return;
            }
            crate::settings::AudioMixerViewMode::Columns => {
                render_mixer_columns(ui, app, &p, body_size);
                return;
            }
            crate::settings::AudioMixerViewMode::Strip => {}
        }
        egui::ScrollArea::vertical()
            .id_salt("audio_mixer_scroll")
            .max_height(body_size.y)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let strip_w = (body_size.x - 10.0).max(120.0);
                render_audio_channel_strip(
                    ui,
                    &mut app.audio_mixer.desktop_audio,
                    &p,
                    "desk_bus",
                    strip_w,
                );
                ui.add_space(3.0);
                ui.separator();
                ui.add_space(2.0);
                render_audio_channel_strip(
                    ui,
                    &mut app.audio_mixer.microphone,
                    &p,
                    "mic_bus",
                    strip_w,
                );

                // OBS-style per-source audio buses: one full-width row card per audio-capable
                // source that is present in Preview or Program, with its own device picker,
                // fader, MUTE and live meter — fully independent of the two global buses above.
                let mut source_audio_changed = false;
                {
                    let available_mics = app.audio_mixer.microphone.available_devices.clone();
                    let available_outs = app.audio_mixer.desktop_audio.available_devices.clone();
                    let program_ids: std::collections::HashSet<uuid::Uuid> = app
                        .studio
                        .program_scene()
                        .sources
                        .iter()
                        .filter(|s| s.visible)
                        .map(|s| s.id)
                        .collect();
                    let preview_id = app.studio.preview_scene_id;
                    let program_id = app.studio.program_scene_id;
                    let mut seen_audio_ids = std::collections::HashSet::new();

                    for scene in &mut app.studio.scenes {
                        if scene.id != program_id && scene.id != preview_id {
                            continue;
                        }
                        for src in &mut scene.sources {
                            if !seen_audio_ids.insert(src.id) {
                                continue;
                            }

                            let is_live_on_pgm = program_ids.contains(&src.id);
                            let src_name = src.name.clone();

                            // Only touch the three audio capture kinds; everything else is
                            // rendered elsewhere in this panel (MediaSource) or not at all.
                            // `is_app` is read BEFORE the mutable `kind` borrow below, so the
                            // shared borrow has already ended by the time `device_slot` holds it.
                            let is_app = matches!(
                                src.kind,
                                SourceKind::ApplicationAudioCapture { .. }
                            );
                            let (is_input, device_slot, volume, muted) = match &mut src.kind {
                                SourceKind::AudioInputCapture {
                                    device_name,
                                    volume,
                                    muted,
                                } => (true, device_name, volume, muted),
                                SourceKind::AudioOutputCapture {
                                    device_name,
                                    volume,
                                    muted,
                                } => (false, device_name, volume, muted),
                                SourceKind::ApplicationAudioCapture {
                                    executable_name,
                                    volume,
                                    muted,
                                    ..
                                } => (false, executable_name, volume, muted),
                                _ => continue,
                            };

                            let bus_id = NexoraApp::source_bus_id(src.id);
                            let level = app.audio_mixer.source_level(&bus_id);

                            ui.add_space(3.0);
                            ui.separator();
                            ui.add_space(2.0);

                            if render_source_audio_strip(
                                ui,
                                &p,
                                &src_name,
                                &bus_id,
                                is_input,
                                is_app,
                                is_live_on_pgm,
                                volume,
                                muted,
                                device_slot,
                                level,
                                if is_input { &available_mics } else { &available_outs },
                                strip_w,
                            ) {
                                source_audio_changed = true;
                            }
                        }
                    }
                }
                if source_audio_changed {
                    app.studio.sync_if_linked();
                }

                // Also show direct Audio Playback controls for any Media Source in Program or Preview
                let mut media_changed = false;
                let program_media_ids: std::collections::HashSet<uuid::Uuid> = app
                    .studio
                    .program_scene()
                    .sources
                    .iter()
                    .filter(|s| s.visible && matches!(s.kind, SourceKind::MediaSource { .. }))
                    .map(|s| s.id)
                    .collect();

                let preview_id = app.studio.preview_scene_id;
                let program_id = app.studio.program_scene_id;
                let mut shown_ids = std::collections::HashSet::new();

                for scene in &mut app.studio.scenes {
                    if scene.id != program_id && scene.id != preview_id {
                        continue;
                    }
                    for src in &mut scene.sources {
                        if !shown_ids.insert(src.id) {
                            continue;
                        }
                        let src_name = src.name.clone();
                        let is_live_on_pgm = program_media_ids.contains(&src.id);
                        if let SourceKind::MediaSource {
                            volume,
                            muted,
                            is_playing,
                            restart_nonce,
                            ..
                        } = &mut src.kind
                        {
                            ui.add_space(3.0);
                            ui.separator();
                            ui.add_space(2.0);
                            ui.allocate_ui_with_layout(
                                Vec2::new(strip_w, 22.0),
                                Layout::left_to_right(Align::Center),
                                |ui| {
                                    let mute_label = if *muted {
                                        format!("{} MUTE", icons::VOLUME_OFF)
                                    } else {
                                        format!("{} ON", icons::VOLUME)
                                    };
                                    let mute_btn = egui::Button::new(
                                        RichText::new(mute_label)
                                            .small()
                                            .strong()
                                            .color(if *muted { p.rec_red } else { p.text_primary }),
                                    )
                                    .min_size(Vec2::new(54.0, 20.0));
                                    if ui
                                        .add(mute_btn)
                                        .on_hover_text("Mute / Unmute Media Source playback sound")
                                        .clicked()
                                    {
                                        *muted = !*muted;
                                        media_changed = true;
                                    }

                                    let play_icon = if *is_playing {
                                        icons::PLAYER_PAUSE
                                    } else {
                                        icons::PLAYER_PLAY
                                    };
                                    if paint_icon_action_btn(
                                        ui,
                                        play_icon,
                                        None,
                                        &p,
                                        "Pause / Play Media",
                                    ) {
                                        *is_playing = !*is_playing;
                                        media_changed = true;
                                    }
                                    if paint_icon_action_btn(
                                        ui,
                                        icons::REFRESH,
                                        None,
                                        &p,
                                        "Restart Media from 00:00",
                                    ) {
                                        *is_playing = true;
                                        *restart_nonce = restart_nonce.wrapping_add(1);
                                        media_changed = true;
                                    }

                                    let status_tag = if is_live_on_pgm { "PGM" } else { "PRV" };
                                    let status_col = if is_live_on_pgm {
                                        p.accent_program
                                    } else {
                                        p.accent_preview
                                    };
                                    ui.label(
                                        RichText::new(status_tag)
                                            .size(9.5)
                                            .strong()
                                            .color(status_col),
                                    )
                                    .on_hover_text(if is_live_on_pgm {
                                        "Live on Program — Sound is active"
                                    } else {
                                        "Standby in Preview — Sound will play when transitioned to Program"
                                    });

                                    ui.label(
                                        RichText::new(truncate_str(&src_name, 12))
                                            .strong()
                                            .size(11.0),
                                    );

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        let pct = (*volume * 100.0).round() as i32;
                                        ui.label(
                                            RichText::new(format!("{:>3}%", pct))
                                                .size(10.5)
                                                .monospace()
                                                .color(p.text_secondary),
                                        );
                                        let rem_w = (ui.available_width() - 6.0).clamp(42.0, 120.0);
                                        ui.scope(|ui| {
                                            ui.spacing_mut().slider_width = rem_w;
                                            if ui
                                                .add_enabled(
                                                    !*muted,
                                                    egui::Slider::new(volume, 0.0..=1.0)
                                                        .show_value(false)
                                                        .trailing_fill(true),
                                                )
                                                .changed()
                                            {
                                                media_changed = true;
                                            }
                                        });
                                    });
                                },
                            );
                        }
                    }
                }
                if media_changed {
                    app.studio.sync_media_transport_to_program();
                    app.studio.sync_if_linked();
                }
            });
    });

    regions.with_footer(ui, |ui| {
        let refresh_label = dock_button_label(ui.available_width(), "Refresh", "Sync");
        let refresh_btn = crate::ui::button_3d::Button3D::new(refresh_label)
            .icon(icons::REFRESH)
            .min_size(Vec2::new(0.0, 24.0))
            .tooltip("Rescan Windows WASAPI audio playback and microphone devices");

        if ui.add(refresh_btn).clicked() {
            app.audio_mixer.refresh_devices_from_system();
        }

        let both_muted =
            app.audio_mixer.desktop_audio.muted && app.audio_mixer.microphone.muted;
        // "Unmute All" is two characters longer than "Mute All", so in a narrow panel the label has
        // to shorten to the verb alone — the icon already says which direction it toggles.
        let mute_long = if both_muted { "Unmute All" } else { "Mute All" };
        let mute_short = if both_muted { "Unmute" } else { "Mute" };
        let mute_all_label = dock_button_label(ui.available_width(), mute_long, mute_short);
        let mute_all_icon = if both_muted { icons::VOLUME } else { icons::VOLUME_OFF };

        let mute_btn = crate::ui::button_3d::Button3D::new(mute_all_label)
            .icon(mute_all_icon)
            .min_size(Vec2::new(0.0, 24.0))
            .tooltip("Mute or unmute all audio channels");

        if ui.add(mute_btn).clicked() {
            let target = !both_muted;
            app.audio_mixer.desktop_audio.muted = target;
            app.audio_mixer.microphone.muted = target;
        }

        let reset_btn = crate::ui::button_3d::Button3D::new("Reset")
            .min_size(Vec2::new(0.0, 24.0))
            .tooltip("Reset audio channel levels to default");

        if ui.add(reset_btn).clicked() {
            app.audio_mixer.reset_levels();
        }
    });

    Some(card_rect)
}

fn render_audio_channel_strip(
    ui: &mut egui::Ui,
    channel: &mut AudioChannel,
    p: &ThemePalette,
    id_salt: &str,
    row_w: f32,
) {
    let is_mic = id_salt == "mic_bus";
    let disabled = channel.is_disabled();

    // Row 1: Mute Button + Label on left | Volume Slider + dB Readout on right
    ui.allocate_ui_with_layout(
        Vec2::new(row_w, 20.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            let mute_label = if channel.muted || disabled {
                if is_mic {
                    format!("{} MUTE", icons::MICROPHONE_OFF)
                } else {
                    format!("{} MUTE", icons::VOLUME_OFF)
                }
            } else if is_mic {
                format!("{} ON", icons::MICROPHONE)
            } else {
                format!("{} ON", icons::VOLUME)
            };
            let mute_btn = egui::Button::new(
                RichText::new(mute_label)
                    .small()
                    .strong()
                    .color(if channel.muted || disabled {
                        p.rec_red
                    } else {
                        p.text_primary
                    }),
            )
            .min_size(Vec2::new(54.0, 20.0));

            if ui
                .add_enabled(!disabled, mute_btn)
                .on_hover_text("Toggle channel mute")
                .clicked()
            {
                channel.muted = !channel.muted;
            }

            ui.label(RichText::new(&channel.label).strong().size(11.0));

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let db = channel.volume_db();
                let pct = channel.volume_percent();
                let db_str = if db <= -59.5 {
                    format!("{:>3}% (-∞ dB)", pct)
                } else {
                    format!("{:>3}% ({:+.1}dB)", pct, db)
                };
                // The numeric readout is the first thing to go when the strip is squeezed: the
                // fader's own tooltip carries the same number, and a clipped readout would only
                // push the fader into an unusable width.
                let show_db = ui.available_width() >= MIXER_DB_READOUT_MIN_W;
                if show_db {
                    ui.label(
                        RichText::new(db_str)
                            .size(10.5)
                            .monospace()
                            .color(p.text_secondary),
                    );
                }

                let rem_w = (ui.available_width() - 6.0).clamp(48.0, 140.0);
                ui.scope(|ui| {
                    ui.spacing_mut().slider_width = rem_w;
                    ui.add_enabled(
                        !channel.muted && !disabled,
                        egui::Slider::new(&mut channel.volume, 0.0..=1.0)
                            .show_value(false)
                            .trailing_fill(true),
                    );
                });
            });
        },
    );

    // Row 2: Full-Width Device Selector (Cleanly truncated so long driver names never overflow)
    let max_chars = ((row_w - 34.0) / 6.6).clamp(18.0, 58.0) as usize;
    let display_dev = truncate_str(&channel.selected_device, max_chars);

    ui.allocate_ui_with_layout(
        Vec2::new(row_w, 20.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            let combo_resp = egui::ComboBox::from_id_salt(id_salt)
                .selected_text(display_dev)
                .width(row_w)
                .show_ui(ui, |ui| {
                    for dev in &channel.available_devices {
                        ui.selectable_value(&mut channel.selected_device, dev.clone(), dev);
                    }
                });
            combo_resp
                .response
                .on_hover_text(format!("Active Device: {}", channel.selected_device));
        },
    );

    // Row 3: Live WASAPI Peak + Hold Level Meter Bar (shared painter, see `paint_level_meter`)
    let level = channel.effective_level();
    let hold = if channel.muted || disabled { 0.0 } else { channel.peak_hold };
    paint_level_meter(ui, p, level, hold, row_w).on_hover_text(format!(
        "Live Signal: {:.0}% | Peak Hold: {:.0}%",
        level * 100.0,
        hold * 100.0
    ));
}

/// Draws the 7 px 3-zone level meter shared by the global channel strips and the per-source
/// audio bus rows: green up to `-9 dB` (70%), amber to `-3 dB` (88%), red above that, with a
/// 2.5 px illuminated peak-hold pill.
fn paint_level_meter(
    ui: &mut egui::Ui,
    p: &ThemePalette,
    level: f32,
    peak_hold: f32,
    meter_w: f32,
) -> egui::Response {
    let meter_h = 7.0_f32;
    let (meter_rect, meter_resp) = ui.allocate_exact_size(Vec2::new(meter_w.max(60.0), meter_h), Sense::hover());
    let painter = ui.painter_at(meter_rect);

    // Track background well with smooth rounded corners and subtle border
    painter.rect_filled(meter_rect, CornerRadius::same(3), p.bg_canvas_well);
    painter.rect_stroke(
        meter_rect,
        CornerRadius::same(3),
        Stroke::new(0.5_f32, p.border_subtle),
        egui::StrokeKind::Inside,
    );

    // Studio calibration tick marks at -9 dB (70%) and -3 dB (88%)
    let tick_stroke = Stroke::new(1.0_f32, p.border_subtle.gamma_multiply(0.7));
    for frac in [0.70_f32, 0.88_f32] {
        let x = meter_rect.min.x + meter_rect.width() * frac;
        painter.line_segment(
            [
                Pos2::new(x, meter_rect.min.y + 1.0),
                Pos2::new(x, meter_rect.max.y - 1.0),
            ],
            tick_stroke,
        );
    }

    if level > 0.005 {
        // Safe green zone (0.0 .. 0.70)
        let green_end = level.min(0.70);
        let green_right_radius = if level <= 0.70 { 3 } else { 0 };
        painter.rect_filled(
            Rect::from_min_max(
                meter_rect.min,
                Pos2::new(meter_rect.min.x + meter_rect.width() * green_end, meter_rect.max.y),
            ),
            CornerRadius {
                nw: 3,
                sw: 3,
                ne: green_right_radius,
                se: green_right_radius,
            },
            p.meter_ok,
        );

        // Caution amber zone (0.70 .. 0.88)
        if level > 0.70 {
            let warn_end = level.min(0.88);
            let warn_right_radius = if level <= 0.88 { 3 } else { 0 };
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(meter_rect.min.x + meter_rect.width() * 0.70, meter_rect.min.y),
                    Pos2::new(meter_rect.min.x + meter_rect.width() * warn_end, meter_rect.max.y),
                ),
                CornerRadius {
                    nw: 0,
                    sw: 0,
                    ne: warn_right_radius,
                    se: warn_right_radius,
                },
                p.meter_warn,
            );
        }

        // Clip red zone (0.88 .. 1.00)
        if level > 0.88 {
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(meter_rect.min.x + meter_rect.width() * 0.88, meter_rect.min.y),
                    Pos2::new(
                        meter_rect.min.x + meter_rect.width() * level.min(1.0),
                        meter_rect.max.y,
                    ),
                ),
                CornerRadius { nw: 0, sw: 0, ne: 3, se: 3 },
                p.meter_clip,
            );
        }
    }

    // Professional peak-hold indicator: a 2.5 px pill matching signal intensity.
    if peak_hold > 0.008 {
        let hold_norm = peak_hold.clamp(0.0, 1.0);
        let hold_center_x = (meter_rect.min.x + meter_rect.width() * hold_norm)
            .clamp(meter_rect.min.x + 1.5, meter_rect.max.x - 1.5);
        let pill_half_w = 1.25_f32;
        let hold_rect = Rect::from_min_max(
            Pos2::new(hold_center_x - pill_half_w, meter_rect.min.y),
            Pos2::new(hold_center_x + pill_half_w, meter_rect.max.y),
        );
        let peak_color = if hold_norm > 0.88 {
            p.meter_clip
        } else if hold_norm > 0.70 {
            p.meter_warn
        } else {
            p.meter_ok
        };
        painter.rect_filled(hold_rect, CornerRadius::same(1), peak_color);
        painter.rect_stroke(
            hold_rect,
            CornerRadius::same(1),
            Stroke::new(0.5_f32, egui::Color32::from_white_alpha(150)),
            egui::StrokeKind::Inside,
        );
    }

    meter_resp
}

/// One full-width row card for a per-source audio bus (OBS-style).
///
/// Layout matches the global channel strips so the panel reads as a single consistent mixer:
/// row 1 = MUTE toggle + source name + `PGM`/`PRV` badge + volume slider + dB readout,
/// row 2 = full-width device picker (or the resolved application name), row 3 = live meter.
///
/// Returns `true` when the user changed something that needs a scene re-sync.
#[allow(clippy::too_many_arguments)]
fn render_source_audio_strip(
    ui: &mut egui::Ui,
    p: &ThemePalette,
    src_name: &str,
    bus_id: &str,
    is_input: bool,
    is_app: bool,
    is_live_on_pgm: bool,
    volume: &mut f32,
    muted: &mut bool,
    device_name: &mut String,
    level: crate::audio::SourceLevel,
    available_devices: &[String],
    row_w: f32,
) -> bool {
    let mut changed = false;

    // Row 1: MUTE toggle + name + PGM/PRV badge | volume slider + dB readout
    ui.allocate_ui_with_layout(
        Vec2::new(row_w, 20.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            let mute_label = if *muted {
                if is_input {
                    format!("{} MUTE", icons::MICROPHONE_OFF)
                } else {
                    format!("{} MUTE", icons::VOLUME_OFF)
                }
            } else if is_input {
                format!("{} ON", icons::MICROPHONE)
            } else {
                format!("{} ON", icons::VOLUME)
            };
            let mute_btn = egui::Button::new(
                RichText::new(mute_label)
                    .small()
                    .strong()
                    .color(if *muted { p.rec_red } else { p.text_primary }),
            )
            .min_size(Vec2::new(54.0, 20.0));

            if ui
                .add(mute_btn)
                .on_hover_text("Mute / unmute this source's audio bus")
                .clicked()
            {
                *muted = !*muted;
                changed = true;
            }

            // Source bus badge: distinguishes Direction (IN/OUT/APP) from PGM/PRV liveness.
            let bus_tag = if is_app {
                "APP"
            } else if is_input {
                "IN"
            } else {
                "OUT"
            };
            ui.label(
                RichText::new(bus_tag)
                    .size(9.5)
                    .strong()
                    .color(p.text_muted),
            )
            .on_hover_text(if is_app {
                "Application Audio Capture bus"
            } else if is_input {
                "Audio Input Capture bus"
            } else {
                "Audio Output Capture bus"
            });

            let status_tag = if is_live_on_pgm { "PGM" } else { "PRV" };
            let status_col = if is_live_on_pgm {
                p.accent_program
            } else {
                p.accent_preview
            };
            ui.label(
                RichText::new(status_tag)
                    .size(9.5)
                    .strong()
                    .color(status_col),
            )
            .on_hover_text(if is_live_on_pgm {
                "Live on Program — this audio is recorded"
            } else {
                "Standby in Preview — audio will be recorded once transitioned to Program"
            });

            ui.label(
                RichText::new(truncate_str(src_name, 12))
                    .strong()
                    .size(11.0),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let db = if *muted || *volume <= 0.0001 {
                    -60.0_f32
                } else {
                    (20.0 * volume.log10()).max(-60.0)
                };
                let pct = if *muted {
                    0
                } else {
                    (volume.clamp(0.0, 1.0) * 100.0).round() as i32
                };
                let db_str = if db <= -59.5 {
                    format!("{:>3}% (-∞ dB)", pct)
                } else {
                    format!("{:>3}% ({:+.1}dB)", pct, db)
                };
                // Same rule as the global bus strips: the readout yields before the fader does.
                let show_db = ui.available_width() >= MIXER_DB_READOUT_MIN_W;
                if show_db {
                    ui.label(
                        RichText::new(db_str)
                            .size(10.5)
                            .monospace()
                            .color(p.text_secondary),
                    );
                }

                let rem_w = (ui.available_width() - 6.0).clamp(42.0, 120.0);
                ui.scope(|ui| {
                    ui.spacing_mut().slider_width = rem_w;
                    if ui
                        .add_enabled(
                            !*muted,
                            egui::Slider::new(volume, 0.0..=1.0)
                                .show_value(false)
                                .trailing_fill(true),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                });
            });
        },
    );

    // Row 2: full-width device picker. Application Audio Capture is bound to the chosen
    // application, so it shows the executable instead of a device list.
    ui.allocate_ui_with_layout(
        Vec2::new(row_w, 20.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            if is_app {
                let max_chars = ((row_w - 34.0) / 6.6).clamp(18.0, 58.0) as usize;
                ui.add(
                    egui::Label::new(
                        RichText::new(truncate_str(device_name, max_chars))
                            .size(10.5)
                            .color(p.text_primary),
                    )
                    .truncate(),
                )
                .on_hover_text(format!(
                    "Application: {}\nChange it from the source's Properties dialog.",
                    device_name
                ));
            } else {
                let max_chars = ((row_w - 34.0) / 6.6).clamp(18.0, 58.0) as usize;
                let display_dev = truncate_str(device_name, max_chars);
                egui::ComboBox::from_id_salt(bus_id)
                    .selected_text(display_dev)
                    .width(row_w)
                    .show_ui(ui, |ui| {
                        for dev in available_devices {
                            if ui
                                .selectable_value(device_name, dev.clone(), dev)
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    })
                    .response
                    .on_hover_text(format!("Active Device: {}", device_name));
            }
        },
    );

    // Row 3: live meter (muted / disabled buses are drawn empty)
    let shown_level = if *muted { 0.0 } else { level.current_peak };
    let shown_hold = if *muted { 0.0 } else { level.peak_hold };
    paint_level_meter(ui, p, shown_level, shown_hold, row_w).on_hover_text(format!(
        "Live Signal: {:.0}% | Peak Hold: {:.0}%",
        shown_level * 100.0,
        shown_hold * 100.0
    ));

    changed
}

/// Returns the card's outer rect so the caller can attach a width splitter to its right edge.
fn render_recording_controls_panel(ui: &mut egui::Ui, app: &mut NexoraApp, size: Vec2) -> Option<Rect> {
    let p = ThemePalette::for_mode(app.settings.theme);
    let (regions, card_rect) = paint_and_split_dock_card(ui, &p, size);

    regions.with_header(ui, |ui| {
        ui.label(
            RichText::new("CONTROLS")
                .size(13.5)
                .strong()
                .color(p.text_primary),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("—").small())
                        .min_size(Vec2::new(22.0, 20.0)),
                )
                .on_hover_text("Minimize Controls panel")
                .clicked()
            {
                app.settings.panels.show_recording_controls = false;
            }
        });
    });

    regions.with_body(ui, |ui, body_size| {
        ui.spacing_mut().item_spacing.y = 2.5;

        // Digital Studio Timer Readout
        egui::Frame::new()
            .fill(p.bg_canvas_well)
            .stroke(Stroke::new(1.0_f32, p.border_subtle))
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.set_min_width((body_size.x - 16.0).max(60.0));
                ui.horizontal(|ui| {
                    let dot_color = match app.recording.status {
                        RecordingStatus::Idle => p.text_muted,
                        RecordingStatus::Recording => p.rec_red,
                        RecordingStatus::Paused => p.pause_amber,
                    };
                    ui.label(
                        RichText::new(format!(
                            "{} {}",
                            icons::PLAYER_RECORD,
                            app.recording.status.label()
                        ))
                        .size(11.0)
                        .color(dot_color),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(app.recording.formatted_timer())
                                .monospace()
                                .size(13.5)
                                .color(p.text_primary),
                        );
                    });
                });
            });

        ui.add_space(1.5);

        // Start / Pause / Stop Recording Button Row (Full-width, No Icon)
        let is_idle = app.recording.status == RecordingStatus::Idle;
        let btn_full_w = ui.available_width();

        if is_idle {
            let start_btn = crate::ui::button_3d::Button3D::danger("Start Recording")
                .min_size(Vec2::new(btn_full_w, 26.0))
                .tooltip("Start local MP4 recording session");

            if ui.add(start_btn).clicked() {
                let _ = app.start_recording_session();
            }
        } else {
            let pause_label = if app.recording.status == RecordingStatus::Paused {
                "Resume"
            } else {
                "Pause"
            };
            let half_w = ((btn_full_w - 4.0) * 0.5).max(40.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        crate::ui::button_3d::Button3D::warning(pause_label)
                            .min_size(Vec2::new(half_w, 26.0)),
                    )
                    .clicked()
                {
                    app.toggle_recording_pause();
                }

                let stop_btn = crate::ui::button_3d::Button3D::danger("Stop")
                    .min_size(Vec2::new(half_w, 26.0));

                if ui.add(stop_btn).clicked() {
                    app.stop_recording_session();
                }
            });
        }

        ui.add_space(2.0);

        // Virtual Camera Button Row (Full-width, Identical Size to Recording, No Icon)
        let vcam_active = app.virtual_camera_active;
        let vcam_btn = if vcam_active {
            crate::ui::button_3d::Button3D::primary("Stop Virtual Camera")
                .min_size(Vec2::new(btn_full_w, 26.0))
                .selected(true)
                .tooltip("Stop Virtual Camera (Right-click for Settings)")
        } else {
            crate::ui::button_3d::Button3D::new("Start Virtual Camera")
                .min_size(Vec2::new(btn_full_w, 26.0))
                .tooltip("Start Virtual Camera (Right-click for Settings)")
        };

        let vcam_resp = ui.add(vcam_btn);

        if vcam_resp.clicked() {
            app.toggle_virtual_camera();
        }

            vcam_resp.context_menu(|ui| {
                if ui.button(format!("{}  Virtual Camera Settings...", icons::SETTINGS)).clicked() {
                    app.open_virtual_camera_dialog();
                    ui.close_menu();
                }
            });

        // Output Destination Path (Diposisikan di paling bawah dengan teks kecil rapi)
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.label(
                    RichText::new("Out:")
                        .size(10.0)
                        .color(p.text_muted),
                );
                let dir_str = app.settings.output_directory.to_string_lossy();
                ui.add(
                    egui::Label::new(
                        RichText::new(dir_str.as_ref())
                            .size(10.0)
                            .monospace()
                            .color(p.text_secondary),
                    )
                    .truncate(),
                )
                .on_hover_text(format!("Output Directory: {}", dir_str));
            });
        });
    });

    regions.with_footer(ui, |ui| {
        // Three buttons in the narrowest default panel, so the labels shorten early rather than
        // letting the last button clip. Every short form keeps the icon that identifies it.
        let open_label = dock_button_label(ui.available_width(), "Open Folder", "Folder");
        let open_btn = crate::ui::button_3d::Button3D::new(open_label)
            .icon(icons::FOLDER_OPEN)
            .min_size(Vec2::new(0.0, 24.0))
            .tooltip("Open output directory in Windows Explorer");

        if ui.add(open_btn).clicked() {
            let _ = crate::platform::open_folder_in_explorer(&app.settings.output_directory);
        }

        let lib_label = dock_button_label(ui.available_width(), "Library", "Lib");
        let lib_btn = crate::ui::button_3d::Button3D::new(lib_label)
            .icon(icons::MOVIE)
            .min_size(Vec2::new(0.0, 24.0))
            .selected(app.settings.panels.show_library_drawer)
            .tooltip("Open Recording Library drawer");

        if ui.add(lib_btn).clicked() {
            app.settings.panels.show_library_drawer =
                !app.settings.panels.show_library_drawer;
            if app.settings.panels.show_library_drawer {
                app.media_library
                    .scan_directory(&app.settings.output_directory);
            }
        }

        let vcam_btn = crate::ui::button_3d::Button3D::new("VCam")
            .icon(icons::SETTINGS)
            .min_size(Vec2::new(0.0, 24.0))
            .tooltip("Virtual Camera Settings (Output Type: Program, Preview, Scene)");

        if ui.add(vcam_btn).clicked() {
            app.open_virtual_camera_dialog();
        }
    });

    Some(card_rect)
}

#[cfg(test)]
mod tests {
    use super::{
        dock_button_label, dock_panel_min_w, icons, mixer_row_layout, resolve_dock_column_widths,
        truncate_to_width, MixerChannel, ThemePalette, DOCK_PANEL_MIN_W, SCENE_NAME_CHAR_W,
    };
    use crate::settings::ThemeMode;
    use egui::Color32;

    fn widths(weights: [f32; 4], visible: [bool; 4], usable_w: f32) -> Vec<f32> {
        let mut w = weights;
        resolve_dock_column_widths(&mut w, visible, usable_w)
    }

    /// The default weights must reproduce the old equal split, so upgrading users see no change.
    #[test]
    fn equal_weights_split_the_dock_evenly() {
        let w = widths([1.0; 4], [true; 4], 1200.0);
        assert_eq!(w.len(), 4);
        for col in &w {
            assert!((col - 300.0).abs() < 0.01);
        }
    }

    /// Weights scale proportionally: a panel with twice the weight of another gets twice the width.
    ///
    /// The budget is wide enough for every panel's floor, so nothing has to give way and the split
    /// is purely proportional.
    #[test]
    fn weights_are_normalized_proportionally() {
        let w = widths([2.0, 1.0, 1.0, 1.0], [true; 4], 2000.0);
        assert!((w[0] - 800.0).abs() < 0.01);
        assert!((w[1] - 400.0).abs() < 0.01);
    }

    /// A panel's floor outranks its proportional share.
    ///
    /// With a 1000 px dock the weights `[2, 1, 1, 1]` would hand the Audio Mixer only 200 px,
    /// which is less than its 240 px floor. The floor wins: the mixer keeps what it needs and the
    /// surplus is taken from the panel that has the most room to spare.
    #[test]
    fn a_panel_floor_wins_over_a_proportional_share() {
        let w = widths([2.0, 1.0, 1.0, 1.0], [true; 4], 1000.0);
        let mixer_floor = dock_panel_min_w(2);
        assert!(
            w[2] >= mixer_floor - 0.01,
            "Audio Mixer got {} px, below its floor of {mixer_floor} px",
            w[2]
        );
        for (idx, col) in w.iter().enumerate() {
            assert!(
                *col >= dock_panel_min_w(idx) - 0.01,
                "panel {idx} is {col} px, below its floor"
            );
        }
        assert!((w.iter().sum::<f32>() - 1000.0).abs() < 0.01);
    }

    /// Hiding a panel hands its share to the remaining ones instead of leaving a gap in the dock.
    #[test]
    fn hidden_panels_redistribute_their_share() {
        let w = widths([1.0; 4], [true, true, false, false], 1000.0);
        assert_eq!(w.len(), 2);
        assert!((w.iter().sum::<f32>() - 1000.0).abs() < 0.01);
        assert!((w[0] - 500.0).abs() < 0.01);
    }

    /// The columns never exceed the space the dock was given. When the per-panel floors cannot all
    /// fit (four panels on a narrow window), the row is scaled down as a whole rather than
    /// overflowing and pushing the last card off-screen where it cannot be reached.
    #[test]
    fn clamped_columns_still_fit_the_available_width() {
        let w = widths([1.0; 4], [true; 4], 400.0);
        let total: f32 = w.iter().sum();
        assert!(
            total <= 400.01,
            "columns overflowed the dock: {}",
            total
        );
        assert!(w.iter().all(|c| *c > 0.0));
    }

    /// A dragged boundary must be preserved: the weights passed in are never rewritten by the
    /// resolver, so the user's split survives every frame it is recomputed on.
    #[test]
    fn resolver_does_not_mutate_valid_weights() {
        let mut weights = [2.0, 1.0, 1.0, 1.0];
        resolve_dock_column_widths(&mut weights, [true; 4], 1000.0);
        assert_eq!(weights, [2.0, 1.0, 1.0, 1.0]);
    }

    /// When four minimum-width panels cannot all fit, they are scaled down together instead of
    /// overflowing: an off-screen card would be unreachable, a cramped one still is not.
    #[test]
    fn columns_are_scaled_down_when_the_minimums_do_not_fit() {
        let w = widths([1.0; 4], [true; 4], 200.0);
        let total: f32 = w.iter().sum();
        assert!(
            (total - 200.0).abs() < 0.01,
            "columns did not fit the dock: {}",
            total
        );
        for col in &w {
            assert!(*col > 0.0);
        }
    }

    /// A zero or negative weight (from a hand-edited config file) is repaired rather than allowed to
    /// collapse a panel to zero width, which would make its splitter impossible to grab again.
    #[test]
    fn invalid_weights_are_repaired() {
        let w = widths([0.0, -5.0, f32::NAN, 1.0], [true; 4], 800.0);
        for (idx, col) in w.iter().enumerate() {
            assert!(
                *col >= dock_panel_min_w(idx) - 0.01,
                "panel {idx} was repaired below its own floor: {col}"
            );
            assert!(col.is_finite());
        }
    }

    /// Each panel is held to its own floor, not to one shared number: a card whose contents need
    /// more room must not be squeezed down to the same width as a card holding two small buttons.
    #[test]
    fn each_panel_respects_its_own_floor() {
        // A budget wide enough for the floors, but with weights that would crush every panel.
        let w = widths([1.0; 4], [true; 4], 800.0);
        let floors = [
            dock_panel_min_w(0),
            dock_panel_min_w(1),
            dock_panel_min_w(2),
            dock_panel_min_w(3),
        ];
        for (idx, col) in w.iter().enumerate() {
            assert!(
                *col >= floors[idx] - 0.01,
                "panel {idx} is {col} px, below its floor of {} px",
                floors[idx]
            );
        }
    }

    /// Hiding a panel must not let the remaining ones inherit the hidden one's floor, and the
    /// visible panels still have to fit their budget.
    #[test]
    fn hidden_panels_do_not_leak_their_floor() {
        let w = widths([1.0; 4], [true, false, false, false], 400.0);
        assert_eq!(w.len(), 1);
        // The Scenes panel keeps its own floor even when it is the only one left.
        assert!(w[0] >= dock_panel_min_w(0) - 0.01);
        assert!((w.iter().sum::<f32>() - 400.0).abs() < 0.01);
    }

    /// A wide panel has room for the full label; nothing is shortened needlessly.
    #[test]
    fn full_label_is_kept_when_there_is_room() {
        assert_eq!(dock_button_label(400.0, "Properties", "Props"), "Properties");
        assert_eq!(dock_button_label(400.0, "Add Source", "Add"), "Add Source");
    }

    /// A narrow panel gets the short form instead of a label that would be clipped away.
    #[test]
    fn label_shortens_in_a_narrow_panel() {
        assert_eq!(dock_button_label(70.0, "Properties", "Props"), "Props");
        assert_eq!(dock_button_label(60.0, "Open Folder", "Folder"), "Folder");
    }

    /// Even a panel squeezed to the minimum still shows a usable label rather than an empty string.
    #[test]
    fn label_never_disappears_at_the_minimum_width() {
        for (long, short) in [
            ("Add Source", "Add"),
            ("Properties", "Props"),
            ("Filters", "Filter"),
            ("Duplicate", "Dupe"),
            ("Open Folder", "Folder"),
            ("Library", "Lib"),
            ("Refresh", "Sync"),
        ] {
            assert!(
                !dock_button_label(DOCK_PANEL_MIN_W, long, short).is_empty(),
                "label for {long} vanished at the minimum panel width"
            );
        }
    }

    /// The short form must actually save room, otherwise shortening would not help the panel fit.
    #[test]
    fn short_labels_are_strictly_shorter_than_long_ones() {
        for (long, short) in [
            ("Add Source", "Add"),
            ("Properties", "Props"),
            ("Filters", "Filter"),
            ("Duplicate", "Dupe"),
            ("Open Folder", "Folder"),
            ("Library", "Lib"),
            ("Refresh", "Sync"),
        ] {
            assert!(
                short.chars().count() < long.chars().count(),
                "\"{short}\" is not shorter than \"{long}\""
            );
        }
    }

    /// A scene name is truncated by the space that is actually left, not by a character count.
    #[test]
    fn scene_name_is_truncated_by_available_width() {
        // Wide enough for the whole name: nothing is cut.
        assert_eq!(truncate_to_width("Opening Titles", 200.0, 6.4), "Opening Titles");
        // Too narrow: the name is shortened and marked as truncated.
        let short = truncate_to_width("Opening Titles", 50.0, 6.4);
        assert!(short.ends_with('…'), "truncated name lost its ellipsis: {short}");
        assert!(short.chars().count() < "Opening Titles".chars().count());
        // No room at all: nothing is drawn rather than a broken one-character name.
        assert_eq!(truncate_to_width("Opening Titles", 0.0, 6.4), "");
    }

    /// The truncated name must never be wider than the space it was given.
    #[test]
    fn truncated_name_never_exceeds_its_budget() {
        let name = "A Very Long Scene Name That Will Not Fit";
        for max_w in [12.0, 30.0, 64.0, 128.0] {
            let t = truncate_to_width(name, max_w, SCENE_NAME_CHAR_W);
            let est_w = t.chars().count() as f32 * SCENE_NAME_CHAR_W;
            assert!(
                est_w <= max_w + SCENE_NAME_CHAR_W,
                "\"{t}\" is ~{est_w} px wide, over its {max_w} px budget"
            );
        }
    }

    /// The compact mixer rows have to fill their width exactly.
    ///
    /// Every row shares one geometry, so the faders of all rows start at the same x. The geometry
    /// has to add up to the width of the row: any less and the columns drift apart, any more and
    /// the fader is pushed out of the panel.
    #[test]
    fn mixer_rows_fill_their_width_exactly() {
        let mut muted_a = false;
        let mut vol_a = 0.5_f32;
        let mut muted_b = false;
        let mut vol_b = 0.8_f32;
        let mut muted_c = false;
        let mut vol_c = 0.2_f32;
        let mut playing = true;
        let channels = vec![
            MixerChannel {
                name: "Desktop Audio".to_owned(),
                icon: icons::VOLUME,
                icon_off: icons::VOLUME_OFF,
                tags: [None, None],
                muted: &mut muted_a,
                volume: &mut vol_a,
                is_playing: None,
                level: 0.0,
                peak_hold: 0.0,
                changed: false,
                transport_changed: false,
            },
            MixerChannel {
                name: "A Very Long Microphone Name".to_owned(),
                icon: icons::MICROPHONE,
                icon_off: icons::MICROPHONE_OFF,
                tags: [Some(("IN", Color32::GRAY)), Some(("PGM", Color32::GREEN))],
                muted: &mut muted_b,
                volume: &mut vol_b,
                is_playing: None,
                level: 0.0,
                peak_hold: 0.0,
                changed: false,
                transport_changed: false,
            },
            MixerChannel {
                name: "Media".to_owned(),
                icon: icons::VOLUME,
                icon_off: icons::VOLUME_OFF,
                tags: [None, None],
                muted: &mut muted_c,
                volume: &mut vol_c,
                is_playing: Some(&mut playing),
                level: 0.0,
                peak_hold: 0.0,
                changed: false,
                transport_changed: false,
            },
        ];

        egui::__run_test_ui(|ui| {
            let p = ThemePalette::for_mode(ThemeMode::Dark);
            for row_w in [240.0_f32, 330.0, 400.0, 720.0] {
                let lay = mixer_row_layout(ui, &p, &channels, row_w);
                let total = lay.total_w();
                assert!(
                    (total - row_w).abs() < 0.5,
                    "a {row_w} px row was filled with {total} px of columns"
                );
                assert!(lay.name_w >= 0.0 && lay.fader_w >= 0.0);
            }
        });
    }

    /// The transport button and the tags get a slot in every row, even in the rows that do not
    /// use them — otherwise a media channel would push its fader out of line with the others.
    #[test]
    fn mixer_rows_reserve_a_slot_for_every_column() {
        let mut muted_a = false;
        let mut vol_a = 0.5_f32;
        let mut muted_b = false;
        let mut vol_b = 0.8_f32;
        let mut playing = true;
        let channels = vec![
            MixerChannel {
                name: "Desktop Audio".to_owned(),
                icon: icons::VOLUME,
                icon_off: icons::VOLUME_OFF,
                tags: [None, None],
                muted: &mut muted_a,
                volume: &mut vol_a,
                is_playing: None,
                level: 0.0,
                peak_hold: 0.0,
                changed: false,
                transport_changed: false,
            },
            MixerChannel {
                name: "Media".to_owned(),
                icon: icons::VOLUME,
                icon_off: icons::VOLUME_OFF,
                tags: [Some(("APP", Color32::GRAY)), Some(("PRV", Color32::BLUE))],
                muted: &mut muted_b,
                volume: &mut vol_b,
                is_playing: Some(&mut playing),
                level: 0.0,
                peak_hold: 0.0,
                changed: false,
                transport_changed: false,
            },
        ];

        egui::__run_test_ui(|ui| {
            let p = ThemePalette::for_mode(ThemeMode::Dark);
            let lay = mixer_row_layout(ui, &p, &channels, 480.0);
            assert!(lay.transport_w > 0.0, "the media channel has no transport slot");
            assert_eq!(lay.tag_slots, 2, "the two tags of the media channel are not reserved");
            assert!(lay.tag_w > 0.0);
        });
    }
}
