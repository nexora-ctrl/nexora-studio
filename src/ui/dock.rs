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

            let active_count = [
                app.settings.panels.show_scenes,
                app.settings.panels.show_sources,
                app.settings.panels.show_audio_mixer,
                app.settings.panels.show_recording_controls,
            ]
            .into_iter()
            .filter(|b| *b)
            .count()
            .max(1);

            let usable_w = (total_w - gap * (active_count.saturating_sub(1) as f32)).max(240.0);
            let col_w = (usable_w / (active_count as f32)).floor();
            let card_h = total_h.floor().max(160.0);
            let card_size = Vec2::new(col_w, card_h);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;

                if app.settings.panels.show_scenes {
                    render_scenes_panel(ui, app, card_size);
                }

                if app.settings.panels.show_sources {
                    render_sources_panel(ui, app, card_size);
                }

                if app.settings.panels.show_audio_mixer {
                    render_audio_mixer_panel(ui, app, card_size);
                }

                if app.settings.panels.show_recording_controls {
                    render_recording_controls_panel(ui, app, card_size);
                }
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

fn paint_and_split_dock_card(
    ui: &mut egui::Ui,
    p: &ThemePalette,
    card_size: Vec2,
) -> DockCardRegions {
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

    DockCardRegions {
        header,
        body,
        footer,
    }
}

fn paint_icon_action_btn(
    ui: &mut egui::Ui,
    icon: &str,
    accent_color: Option<Color32>,
    p: &ThemePalette,
    tooltip: &str,
) -> bool {
    let btn_size = Vec2::new(24.0, 22.0);
    let (rect, resp) = ui.allocate_exact_size(btn_size, Sense::click());
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

fn render_scenes_panel(ui: &mut egui::Ui, app: &mut NexoraApp, size: Vec2) {
    let p = ThemePalette::for_mode(app.settings.theme);
    let regions = paint_and_split_dock_card(ui, &p, size);

    regions.with_header(ui, |ui| {
        ui.label(
            RichText::new("Scenes")
                .size(12.0)
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

                        let name_text = truncate_str(&scene.name, 18);
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
                        let text_x = row_rect.min.x + 33.0;

                        // Draw clean, regular weight scene name (not bold)
                        ui.painter().galley(
                            Pos2::new(text_x, text_top_y),
                            name_galley,
                            p.text_primary,
                        );

                        // Draw subtitle with comfortable breathing room below scene name
                        ui.painter().galley(
                            Pos2::new(text_x, text_top_y + name_h + gap_y),
                            desc_galley,
                            p.text_secondary,
                        );

                        // Right-side visual status badges
                        let badges_rect = Rect::from_min_max(
                            Pos2::new(text_x + 80.0, row_rect.min.y + 2.0),
                            Pos2::new(row_rect.max.x - 7.0, row_rect.max.y - 2.0),
                        );
                        let mut badges_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(badges_rect)
                                .layout(Layout::right_to_left(Align::Center)),
                        );
                        badges_ui.spacing_mut().item_spacing.x = 4.0;
                        if is_pgm {
                            paint_status_pill(
                                &mut badges_ui,
                                &format!("{} LIVE", icons::PLAYER_RECORD),
                                p.accent_program,
                                p.accent_program,
                                Color32::WHITE,
                                "This scene is currently LIVE on Program output",
                            );
                        }
                        if is_prv {
                            paint_status_pill(
                                &mut badges_ui,
                                &format!("{} PREVIEW", icons::EYE),
                                Color32::from_rgba_unmultiplied(
                                    p.accent_preview.r(),
                                    p.accent_preview.g(),
                                    p.accent_preview.b(),
                                    45,
                                ),
                                p.accent_preview,
                                p.accent_preview,
                                "This scene is currently open in Preview editor",
                            );
                        }
                        if !is_pgm && !is_prv {
                            paint_status_pill(
                                &mut badges_ui,
                                "STANDBY",
                                Color32::TRANSPARENT,
                                p.border_subtle,
                                p.text_secondary,
                                "Click to open in Preview • Double-click to Transition to LIVE",
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
        let add_scene_btn = crate::ui::button_3d::Button3D::new("")
            .icon(icons::PLUS)
            .min_size(Vec2::new(26.0, 24.0))
            .tooltip("Add Scene...");

        if ui.add(add_scene_btn).clicked() {
            app.open_add_scene_dialog();
        }

        let can_delete = app.studio.scenes.len() > 1;
        let del_scene_btn = crate::ui::button_3d::Button3D::new("")
            .icon(icons::MINUS)
            .min_size(Vec2::new(26.0, 24.0))
            .enabled(can_delete)
            .tooltip("Remove selected Scene");

        if ui.add(del_scene_btn).clicked() {
            let id = app.studio.preview_scene_id;
            app.studio.delete_scene(id);
        }

        let dup_scene_btn = crate::ui::button_3d::Button3D::new("Duplicate")
            .icon(icons::COPY)
            .min_size(Vec2::new(72.0, 24.0))
            .tooltip("Duplicate active Scene");

        if ui.add(dup_scene_btn).clicked() {
            app.studio.duplicate_preview_scene();
        }

        let ren_scene_btn = crate::ui::button_3d::Button3D::new("Rename")
            .icon(icons::EDIT)
            .min_size(Vec2::new(66.0, 24.0))
            .tooltip("Rename active Scene");

        if ui.add(ren_scene_btn).clicked() {
            app.rename_scene_buffer = app.studio.preview_scene().name.clone();
            app.show_rename_scene_dialog = true;
        }
    });
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

fn render_sources_panel(ui: &mut egui::Ui, app: &mut NexoraApp, size: Vec2) {
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

    let selected_id = app.studio.preview_scene().selected_source_id;
    let regions = paint_and_split_dock_card(ui, &p, size);

    regions.with_header(ui, |ui| {
        ui.label(
            RichText::new("Source")
                .size(12.0)
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
            ui.label(
                RichText::new("Right-click to edit")
                    .small()
                    .color(p.text_muted),
            );
        });
    });

    regions.with_body(ui, |ui, body_size| {
        egui::ScrollArea::vertical()
            .id_salt("sources_scroll")
            .max_height(body_size.y)
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

                for src in &mut scene.sources {
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

                    let row_resp = ui.interact(
                        select_rect,
                        ui.id().with(("source_row_select", src_id)),
                        Sense::click(),
                    );
                    let hovered = row_resp.hovered() || ui.rect_contains_pointer(row_rect);

                    // Card Fill & Border (matching Scenes card style + optional color tag tint)
                    let tag_tint = color_tag_tint(src.color_tag);
                    let (card_fill, border_stroke, strip_col) = if is_sel {
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
                    let text_col = if !src.visible {
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
                        "{}\n• Click anywhere on row: Select Source\n• Double-click: Open Properties\n• Right-click: Source Options",
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
        let add_btn = crate::ui::button_3d::Button3D::new("Add Source")
            .icon(icons::PLUS)
            .min_size(Vec2::new(96.0, 24.0))
            .tooltip("Add Source: Open complete layer addition dialog");

        if ui.add(add_btn).clicked() {
            pending_action = Some(SourceContextAction::OpenAddSourceDialog(crate::studio::source::AddSourceType::DisplayCapture));
        }

        let add_menu_resp = ui.menu_button(
            RichText::new(icons::CHEVRON_DOWN).small(),
            |ui| {
                populate_add_source_menu(ui, &mut pending_action);
            },
        );
        add_menu_resp
            .response
            .on_hover_text("Select source type directly from pop-up menu");

        let has_sel = selected_id.is_some();

        let rem_btn = crate::ui::button_3d::Button3D::new("")
            .icon(icons::MINUS)
            .min_size(Vec2::new(26.0, 24.0))
            .enabled(has_sel)
            .tooltip("Remove selected Source");

        if ui.add(rem_btn).clicked() {
            if let Some(id) = selected_id {
                pending_action = Some(SourceContextAction::Remove(id));
            }
        }

        let prop_btn = crate::ui::button_3d::Button3D::new("Properties")
            .icon(icons::SETTINGS)
            .min_size(Vec2::new(80.0, 24.0))
            .enabled(has_sel)
            .tooltip("Open Source Properties (Configure Monitor, Window, Camera, etc.)");

        if ui.add(prop_btn).clicked() {
            if let Some(id) = selected_id {
                pending_action = Some(SourceContextAction::OpenProperties(id));
            }
        }

        let flt_btn = crate::ui::button_3d::Button3D::new("Filters")
            .icon(icons::FILTER)
            .min_size(Vec2::new(68.0, 24.0))
            .enabled(has_sel)
            .tooltip("Open Filters (Chroma Key, Color Key, Luma Key)");

        if ui.add(flt_btn).clicked() {
            if let Some(id) = selected_id {
                pending_action = Some(SourceContextAction::OpenFilters(id));
            }
        }

        let up_btn = crate::ui::button_3d::Button3D::new("")
            .icon(icons::ARROW_UP)
            .min_size(Vec2::new(26.0, 24.0))
            .enabled(has_sel)
            .tooltip("Move Source Up (Bring Forward — Closer to Front of Screen)");

        if ui.add(up_btn).clicked() {
            if let Some(id) = selected_id {
                pending_action = Some(SourceContextAction::MoveUp(id));
            }
        }

        let down_btn = crate::ui::button_3d::Button3D::new("")
            .icon(icons::ARROW_DOWN)
            .min_size(Vec2::new(26.0, 24.0))
            .enabled(has_sel)
            .tooltip("Move Source Down (Send Backward — Behind Other Layers)");

        if ui.add(down_btn).clicked() {
            if let Some(id) = selected_id {
                pending_action = Some(SourceContextAction::MoveDown(id));
            }
        }
    });

    if let Some(action) = pending_action {
        execute_source_context_action(app, action);
    }
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

fn render_audio_mixer_panel(ui: &mut egui::Ui, app: &mut NexoraApp, size: Vec2) {
    let p = ThemePalette::for_mode(app.settings.theme);
    let regions = paint_and_split_dock_card(ui, &p, size);

    regions.with_header(ui, |ui| {
        ui.label(
            RichText::new("Audio Mixer")
                .size(12.0)
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
            ui.label(
                RichText::new("48 kHz")
                    .small()
                    .monospace()
                    .color(p.text_muted),
            );
        });
    });

    regions.with_body(ui, |ui, body_size| {
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
        let refresh_btn = crate::ui::button_3d::Button3D::new("Refresh")
            .icon(icons::REFRESH)
            .min_size(Vec2::new(72.0, 24.0))
            .tooltip("Rescan Windows WASAPI audio playback and microphone devices");

        if ui.add(refresh_btn).clicked() {
            app.audio_mixer.refresh_devices_from_system();
        }

        let both_muted =
            app.audio_mixer.desktop_audio.muted && app.audio_mixer.microphone.muted;
        let mute_all_label = if both_muted { "Unmute All" } else { "Mute All" };
        let mute_all_icon = if both_muted { icons::VOLUME } else { icons::VOLUME_OFF };

        let mute_btn = crate::ui::button_3d::Button3D::new(mute_all_label)
            .icon(mute_all_icon)
            .min_size(Vec2::new(82.0, 24.0))
            .tooltip("Mute or unmute all audio channels");

        if ui.add(mute_btn).clicked() {
            let target = !both_muted;
            app.audio_mixer.desktop_audio.muted = target;
            app.audio_mixer.microphone.muted = target;
        }

        let reset_btn = crate::ui::button_3d::Button3D::new("Reset")
            .min_size(Vec2::new(52.0, 24.0))
            .tooltip("Reset audio channel levels to default");

        if ui.add(reset_btn).clicked() {
            app.audio_mixer.reset_levels();
        }
    });
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
                ui.label(
                    RichText::new(db_str)
                        .size(10.5)
                        .monospace()
                        .color(p.text_secondary),
                );

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
                ui.label(
                    RichText::new(db_str)
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

fn render_recording_controls_panel(ui: &mut egui::Ui, app: &mut NexoraApp, size: Vec2) {
    let p = ThemePalette::for_mode(app.settings.theme);
    let regions = paint_and_split_dock_card(ui, &p, size);

    regions.with_header(ui, |ui| {
        ui.label(
            RichText::new("Controls")
                .size(12.0)
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
        let open_btn = crate::ui::button_3d::Button3D::new("Open Folder")
            .icon(icons::FOLDER_OPEN)
            .min_size(Vec2::new(90.0, 24.0))
            .tooltip("Open output directory in Windows Explorer");

        if ui.add(open_btn).clicked() {
            let _ = crate::platform::open_folder_in_explorer(&app.settings.output_directory);
        }

        let lib_btn = crate::ui::button_3d::Button3D::new("Library")
            .icon(icons::MOVIE)
            .min_size(Vec2::new(72.0, 24.0))
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
            .min_size(Vec2::new(65.0, 24.0))
            .tooltip("Virtual Camera Settings (Output Type: Program, Preview, Scene)");

        if ui.add(vcam_btn).clicked() {
            app.open_virtual_camera_dialog();
        }
    });
}
