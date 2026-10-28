use crate::app::NexoraApp;
use crate::audio::AudioChannel;
use crate::recording::RecordingStatus;
use crate::studio::source::{
    SlideTransitionEffect, SourceColorTag, SourceItem, SourceKind, TextAnimationEffect,
    TvBannerStyle, TvTextTemplate,
};
use crate::ui::icons;
use crate::ui::theme::ThemePalette;
use egui::{Align, Color32, CornerRadius, Layout, Pos2, Rect, RichText, Sense, Stroke, Vec2};
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
    let dock_resp = egui::TopBottomPanel::bottom("nexora_studio_dock")
        .resizable(true)
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
    let active = resp.is_pointer_button_down_on();

    let bg_fill = if active {
        p.border_subtle
    } else if hovered {
        p.bg_elevated
    } else {
        Color32::from_rgba_unmultiplied(
            p.bg_elevated.r(),
            p.bg_elevated.g(),
            p.bg_elevated.b(),
            175,
        )
    };
    let border_col = if hovered {
        p.text_secondary
    } else {
        p.border_subtle
    };
    let icon_col = accent_color.unwrap_or(if hovered {
        p.text_primary
    } else {
        p.text_secondary
    });

    ui.painter()
        .rect_filled(rect, CornerRadius::same(5), bg_fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(5),
        Stroke::new(1.0_f32, border_col),
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
                .strong()
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

                let preview_id = app.studio.preview_scene_id;
                let program_id = app.studio.program_scene_id;
                let total_scenes = app.studio.scenes.len();
                let row_w = (body_size.x - 4.0).max(150.0);

                ui.spacing_mut().item_spacing.y = 5.0;

                for (idx, scene) in app.studio.scenes.iter().enumerate() {
                    let is_prv = scene.id == preview_id;
                    let is_pgm = scene.id == program_id;
                    let src_count = scene.sources.len();
                    let active_src_count = scene.sources.iter().filter(|s| s.visible).count();

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

                    // Left Status Accent Bar (4.5px wide)
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

                    // Content inside the Scene Card
                    let inner_rect = Rect::from_min_max(
                        Pos2::new(row_rect.min.x + 11.0, row_rect.min.y + 3.0),
                        Pos2::new(row_rect.max.x - 7.0, row_rect.max.y - 3.0),
                    );
                    let mut row_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(inner_rect)
                            .layout(Layout::left_to_right(Align::Center)),
                    );

                    // Scene Icon + Name + Layer Count subtitle
                    let icon_col = if is_pgm {
                        p.accent_program
                    } else if is_prv {
                        p.accent_preview
                    } else {
                        p.text_secondary
                    };
                    row_ui.label(
                        RichText::new(icons::LAYOUT_COLUMNS)
                            .size(14.0)
                            .color(icon_col),
                    );

                    row_ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(
                            RichText::new(truncate_str(&scene.name, 18))
                                .strong()
                                .size(12.0)
                                .color(p.text_primary),
                        );
                        let state_desc = match (is_pgm, is_prv) {
                            (true, true) => format!("Live & Preview • {}/{} layers", active_src_count, src_count),
                            (true, false) => format!("On Air (Live) • {}/{} layers", active_src_count, src_count),
                            (false, true) => format!("Editing in Preview • {}/{} layers", active_src_count, src_count),
                            (false, false) => format!("Standby • {}/{} layers", active_src_count, src_count),
                        };
                        ui.label(
                            RichText::new(state_desc)
                                .size(9.5)
                                .color(p.text_secondary),
                        );
                    });

                    // Right-side visual status badges
                    row_ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        if is_pgm {
                            paint_status_pill(
                                ui,
                                &format!("{} LIVE", icons::PLAYER_RECORD),
                                p.accent_program,
                                p.accent_program,
                                Color32::WHITE,
                                "This scene is currently LIVE on Program output",
                            );
                        }
                        if is_prv {
                            paint_status_pill(
                                ui,
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
                                ui,
                                "STANDBY",
                                Color32::TRANSPARENT,
                                p.border_subtle,
                                p.text_secondary,
                                "Click to open in Preview • Double-click to Transition to LIVE",
                            );
                        }
                    });

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
                            .button(format!("{} Transition to LIVE (Program)", icons::ARROW_RIGHT))
                            .clicked()
                        {
                            transition_scene_to_live = Some(scene_id);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .button(format!("{} Rename Scene...", icons::EDIT))
                            .clicked()
                        {
                            rename_scene_target = Some((scene_id, scene_name_for_menu));
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
                                    RichText::new(format!("{} Delete Scene", icons::TRASH))
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
            });
    });

    regions.with_footer(ui, |ui| {
        if ui
            .add(
                egui::Button::new(RichText::new(format!(" {} ", icons::PLUS)).small())
                    .min_size(Vec2::new(26.0, 24.0)),
            )
            .on_hover_text("Add Scene")
            .clicked()
        {
            let next_idx = app.studio.scenes.len() + 1;
            app.studio.add_scene(format!("Scene {}", next_idx));
        }

        let can_delete = app.studio.scenes.len() > 1;
        if ui
            .add_enabled(
                can_delete,
                egui::Button::new(RichText::new(format!(" {} ", icons::MINUS)).small())
                    .min_size(Vec2::new(26.0, 24.0)),
            )
            .on_hover_text("Remove selected Scene")
            .clicked()
        {
            let id = app.studio.preview_scene_id;
            app.studio.delete_scene(id);
        }

        if ui
            .add(
                egui::Button::new(RichText::new(format!("{} Duplicate", icons::COPY)).small())
                    .min_size(Vec2::new(72.0, 24.0)),
            )
            .on_hover_text("Duplicate active Scene")
            .clicked()
        {
            app.studio.duplicate_preview_scene();
        }

        if ui
            .add(
                egui::Button::new(RichText::new(format!("{} Rename", icons::EDIT)).small())
                    .min_size(Vec2::new(66.0, 24.0)),
            )
            .on_hover_text("Rename active Scene")
            .clicked()
        {
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
enum SourceContextAction {
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
    FitToScreen(Uuid),
    PipBottomRight(Uuid),
    PipBottomLeft(Uuid),
    PipTopRight(Uuid),
    CenterOnScreen(Uuid),
    ToggleLock(Uuid),
    SetColor(Uuid, SourceColorTag),
    SetFilterKind(Uuid, crate::studio::source::KeyFilterKind),
    OpenFilters(Uuid),
    Duplicate(Uuid),
    Remove(Uuid),
    Rename(Uuid),
    OpenProperties(Uuid),
}

fn populate_add_source_menu(ui: &mut egui::Ui, action_out: &mut Option<SourceContextAction>) {
    ui.set_min_width(245.0);
    ui.label(RichText::new("Add Source").small().strong());
    ui.separator();

    let entries: &[(&str, &str, SourceContextAction)] = &[
        (
            icons::VOLUME,
            "Application Audio Capture (BETA)",
            SourceContextAction::AddApplicationAudio,
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
        (icons::WORLD, "Browser", SourceContextAction::AddBrowser),
        (icons::BRUSH, "Color", SourceContextAction::AddColor),
        (
            icons::DEVICE_DESKTOP,
            "Display Capture",
            SourceContextAction::AddDisplay,
        ),
        (
            icons::DEVICE_GAMEPAD,
            "Game Capture",
            SourceContextAction::AddGameCapture,
        ),
        (icons::PHOTO, "Image", SourceContextAction::AddImage),
        (
            icons::PRESENTATION,
            "Image Slide Show",
            SourceContextAction::AddImageSlideShow,
        ),
        (
            icons::PLAYER_PLAY,
            "Media (Video / MP3 / Audio)",
            SourceContextAction::AddMedia,
        ),
        (icons::LIST, "Scene", SourceContextAction::AddSceneSource),
        (
            icons::LETTER_CASE,
            "Text (GDI+)",
            SourceContextAction::AddTextGdiPlus,
        ),
        (
            icons::CAMERA,
            "Video Capture Device",
            SourceContextAction::AddCamera,
        ),
        (
            icons::APP_WINDOW,
            "Window Capture",
            SourceContextAction::AddWindow,
        ),
    ];

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
}

fn populate_source_context_menu(
    ui: &mut egui::Ui,
    src_id: Uuid,
    is_locked: bool,
    current_color: SourceColorTag,
    current_filter_kind: crate::studio::source::KeyFilterKind,
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
            .on_hover_text("Buka jendela pengaturan intensitas, warna target, dan kehalusan filter")
            .clicked()
        {
            *action_out = Some(SourceContextAction::OpenFilters(src_id));
            ui.close_menu();
        }
    });

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
        ui.set_min_width(180.0);
        if ui
            .button(format!("{}  Reset Transform", icons::REFRESH))
            .clicked()
        {
            *action_out = Some(SourceContextAction::ResetTransform(src_id));
            ui.close_menu();
        }
        if ui
            .button(format!("{}  Fit to Screen (16:9)", icons::MAXIMIZE))
            .clicked()
        {
            *action_out = Some(SourceContextAction::FitToScreen(src_id));
            ui.close_menu();
        }
        if ui.button("Center on Screen").clicked() {
            *action_out = Some(SourceContextAction::CenterOnScreen(src_id));
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

    if ui
        .button(format!("{}  Copy / Duplicate", icons::COPY))
        .clicked()
    {
        *action_out = Some(SourceContextAction::Duplicate(src_id));
        ui.close_menu();
    }

    if ui
        .button(format!("{}  Remove                      Del", icons::TRASH))
        .clicked()
    {
        *action_out = Some(SourceContextAction::Remove(src_id));
        ui.close_menu();
    }

    if ui
        .button(format!("{}  Rename...                   F2", icons::EDIT))
        .clicked()
    {
        *action_out = Some(SourceContextAction::Rename(src_id));
        ui.close_menu();
    }

    ui.separator();

    if ui
        .button(format!("{}  Filters...", icons::FILTER))
        .on_hover_text("Atur Chroma Key, Color Key, atau Luma Key")
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

    // Keyboard shortcuts for selected source (Delete / F2) when no text input is focused
    if !ui.ctx().wants_keyboard_input() {
        if let Some(sel_id) = app.studio.preview_scene().selected_source_id {
            if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                pending_action = Some(SourceContextAction::Remove(sel_id));
            } else if ui.input(|i| i.key_pressed(egui::Key::F2)) {
                pending_action = Some(SourceContextAction::Rename(sel_id));
            }
        }
    }

    let selected_id = app.studio.preview_scene().selected_source_id;
    let regions = paint_and_split_dock_card(ui, &p, size);

    regions.with_header(ui, |ui| {
        ui.label(
            RichText::new("Source")
                .strong()
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

                if scene.sources.is_empty() {
                    let empty_resp = ui.selectable_label(
                        false,
                        RichText::new("No sources. Click '+' below or right-click to Add Source")
                            .small()
                            .color(p.text_muted),
                    );
                    empty_resp.context_menu(|ui| {
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
                    let is_media = matches!(src.kind, SourceKind::MediaSource { .. });

                    let row_h = 31.0;
                    let (row_rect, _) =
                        ui.allocate_exact_size(Vec2::new(row_w, row_h), Sense::hover());

                    // Reserve right-hand zone for action buttons ([Play] [Mute] [Eye] [Lock])
                    let btn_count = if is_media { 4 } else { 2 };
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
                }
                let sel_filter_k = scene
                    .sources
                    .iter()
                    .find(|s| s.id == new_selected.unwrap_or(uuid::Uuid::nil()))
                    .map(|s| s.filter.kind)
                    .unwrap_or_default();

                scene.selected_source_id = new_selected;
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
                        if let Some(sel_id) = new_selected {
                            populate_source_context_menu(
                                ui,
                                sel_id,
                                false,
                                SourceColorTag::None,
                                sel_filter_k,
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
        let add_resp = ui.menu_button(
            RichText::new(format!(" {} ", icons::PLUS)).small(),
            |ui| {
                populate_add_source_menu(ui, &mut pending_action);
            },
        );
        add_resp
            .response
            .on_hover_text("Add Source (Display, Window, Camera, Image)");

        if ui
            .add_enabled(
                selected_id.is_some(),
                egui::Button::new(RichText::new(format!(" {} ", icons::MINUS)).small())
                    .min_size(Vec2::new(26.0, 24.0)),
            )
            .on_hover_text("Remove selected Source")
            .clicked()
        {
            if let Some(id) = selected_id {
                pending_action = Some(SourceContextAction::Remove(id));
            }
        }

        if ui
            .add_enabled(
                selected_id.is_some(),
                egui::Button::new(
                    RichText::new(format!("{} Properties", icons::SETTINGS)).small(),
                )
                .min_size(Vec2::new(80.0, 24.0)),
            )
            .on_hover_text("Open Source Properties (Configure Monitor, Window, Camera, etc.)")
            .clicked()
        {
            if let Some(id) = selected_id {
                pending_action = Some(SourceContextAction::OpenProperties(id));
            }
        }

        if ui
            .add_enabled(
                selected_id.is_some(),
                egui::Button::new(
                    RichText::new(format!("{} Filters", icons::FILTER)).small(),
                )
                .min_size(Vec2::new(68.0, 24.0)),
            )
            .on_hover_text("Open Filters (Chroma Key, Color Key, Luma Key)")
            .clicked()
        {
            if let Some(id) = selected_id {
                pending_action = Some(SourceContextAction::OpenFilters(id));
            }
        }

        if ui
            .add_enabled(
                selected_id.is_some(),
                egui::Button::new(RichText::new(format!(" {} ", icons::ARROW_UP)).small())
                    .min_size(Vec2::new(26.0, 24.0)),
            )
            .on_hover_text("Move Source Up (Bring Forward — Closer to Front of Screen)")
            .clicked()
        {
            if let Some(id) = selected_id {
                pending_action = Some(SourceContextAction::MoveUp(id));
            }
        }

        if ui
            .add_enabled(
                selected_id.is_some(),
                egui::Button::new(RichText::new(format!(" {} ", icons::ARROW_DOWN)).small())
                    .min_size(Vec2::new(26.0, 24.0)),
            )
            .on_hover_text("Move Source Down (Send Backward — Behind Other Layers)")
            .clicked()
        {
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
        SourceContextAction::ResetTransform(id) | SourceContextAction::FitToScreen(id) => {
            let scene = app.studio.preview_scene_mut();
            scene.selected_source_id = Some(id);
            if let Some(src) = scene.selected_source_mut() {
                src.transform.reset_full_canvas();
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
                .strong()
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
        if ui
            .add(
                egui::Button::new(
                    RichText::new(format!("{} Refresh", icons::REFRESH)).small(),
                )
                .min_size(Vec2::new(72.0, 24.0)),
            )
            .on_hover_text("Rescan Windows WASAPI audio playback and microphone devices")
            .clicked()
        {
            app.audio_mixer.refresh_devices_from_system();
        }

        let both_muted =
            app.audio_mixer.desktop_audio.muted && app.audio_mixer.microphone.muted;
        let mute_all_label = if both_muted {
            format!("{} Unmute All", icons::VOLUME)
        } else {
            format!("{} Mute All", icons::VOLUME_OFF)
        };
        if ui
            .add(
                egui::Button::new(RichText::new(mute_all_label).small())
                    .min_size(Vec2::new(82.0, 24.0)),
            )
            .on_hover_text("Mute or unmute all audio channels")
            .clicked()
        {
            let target = !both_muted;
            app.audio_mixer.desktop_audio.muted = target;
            app.audio_mixer.microphone.muted = target;
        }

        if ui
            .add(
                egui::Button::new(RichText::new("Reset").small())
                    .min_size(Vec2::new(52.0, 24.0)),
            )
            .on_hover_text("Reset audio channel levels to default")
            .clicked()
        {
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

    // Row 3: Live WASAPI Peak + Hold Level Meter Bar
    let meter_w = row_w.max(60.0);
    let (meter_rect, _) = ui.allocate_exact_size(Vec2::new(meter_w, 6.0), Sense::hover());
    let painter = ui.painter_at(meter_rect);

    painter.rect_filled(meter_rect, CornerRadius::same(2), p.bg_canvas_well);

    let level = channel.effective_level();
    if level > 0.005 {
        let green_end = level.min(0.70);
        let green_rect = Rect::from_min_size(
            meter_rect.min,
            Vec2::new(meter_rect.width() * green_end, meter_rect.height()),
        );
        painter.rect_filled(green_rect, CornerRadius::same(2), p.meter_ok);

        if level > 0.70 {
            let warn_end = level.min(0.90);
            let warn_rect = Rect::from_min_max(
                Pos2::new(meter_rect.min.x + meter_rect.width() * 0.70, meter_rect.min.y),
                Pos2::new(
                    meter_rect.min.x + meter_rect.width() * warn_end,
                    meter_rect.max.y,
                ),
            );
            painter.rect_filled(warn_rect, CornerRadius::ZERO, p.meter_warn);
        }

        if level > 0.90 {
            let clip_rect = Rect::from_min_max(
                Pos2::new(meter_rect.min.x + meter_rect.width() * 0.90, meter_rect.min.y),
                Pos2::new(
                    meter_rect.min.x + meter_rect.width() * level,
                    meter_rect.max.y,
                ),
            );
            painter.rect_filled(clip_rect, CornerRadius::ZERO, p.meter_clip);
        }
    }

    if channel.peak_hold > 0.005 {
        let hold_x = meter_rect.min.x + meter_rect.width() * channel.peak_hold.clamp(0.0, 1.0);
        painter.line_segment(
            [
                Pos2::new(hold_x, meter_rect.min.y),
                Pos2::new(hold_x, meter_rect.max.y),
            ],
            Stroke::new(1.5_f32, p.text_primary),
        );
    }
}

fn render_recording_controls_panel(ui: &mut egui::Ui, app: &mut NexoraApp, size: Vec2) {
    let p = ThemePalette::for_mode(app.settings.theme);
    let regions = paint_and_split_dock_card(ui, &p, size);

    regions.with_header(ui, |ui| {
        ui.label(
            RichText::new("Controls")
                .strong()
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
        // Digital Studio Timer Readout
        egui::Frame::new()
            .fill(p.bg_canvas_well)
            .stroke(Stroke::new(1.0_f32, p.border_subtle))
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(8, 5))
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
                        .strong()
                        .size(11.5)
                        .color(dot_color),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(app.recording.formatted_timer())
                                .monospace()
                                .strong()
                                .size(14.5)
                                .color(p.text_primary),
                        );
                    });
                });
            });

        ui.add_space(6.0);

        // Start / Pause / Stop Buttons
        ui.horizontal(|ui| {
            let is_idle = app.recording.status == RecordingStatus::Idle;

            if is_idle {
                let start_btn = egui::Button::new(
                    RichText::new(format!("{} Start Recording", icons::PLAYER_RECORD))
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(p.rec_red)
                .min_size(Vec2::new(body_size.x, 28.0));

                if ui
                    .add(start_btn)
                    .on_hover_text("Start local MP4 recording session")
                    .clicked()
                {
                    let _ = app.recording.start_session(
                        &app.settings.output_directory,
                        &app.settings.container_format,
                    );
                }
            } else {
                let pause_label = if app.recording.status == RecordingStatus::Paused {
                    format!("{} Resume", icons::PLAYER_PLAY)
                } else {
                    format!("{} Pause", icons::PLAYER_PAUSE)
                };
                if ui
                    .add(
                        egui::Button::new(RichText::new(pause_label).strong())
                            .min_size(Vec2::new(78.0, 28.0)),
                    )
                    .clicked()
                {
                    app.recording.toggle_pause();
                }

                let stop_btn = egui::Button::new(
                    RichText::new(format!("{} Stop", icons::PLAYER_STOP))
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(p.accent_program)
                .min_size(Vec2::new((body_size.x - 84.0).max(50.0), 28.0));

                if ui.add(stop_btn).clicked() {
                    let _ = app.recording.stop_session();
                }
            }
        });

        ui.add_space(6.0);

        // Output Destination Path
        ui.horizontal(|ui| {
            ui.label(RichText::new("Out:").small().color(p.text_muted));
            let dir_str = app.settings.output_directory.to_string_lossy();
            ui.add(
                egui::Label::new(
                    RichText::new(dir_str.as_ref())
                        .small()
                        .monospace()
                        .color(p.text_secondary),
                )
                .truncate(),
            );
        });
    });

    regions.with_footer(ui, |ui| {
        if ui
            .add(
                egui::Button::new(
                    RichText::new(format!("{} Open Folder", icons::FOLDER_OPEN)).small(),
                )
                .min_size(Vec2::new(95.0, 24.0)),
            )
            .on_hover_text("Open output directory in Windows Explorer")
            .clicked()
        {
            let _ = crate::platform::open_folder_in_explorer(&app.settings.output_directory);
        }
        if ui
            .add(
                egui::Button::new(
                    RichText::new(format!("{} Library", icons::MOVIE)).small(),
                )
                .min_size(Vec2::new(78.0, 24.0)),
            )
            .on_hover_text("Open Recording Library drawer")
            .clicked()
        {
            app.settings.panels.show_library_drawer =
                !app.settings.panels.show_library_drawer;
            if app.settings.panels.show_library_drawer {
                app.media_library
                    .scan_directory(&app.settings.output_directory);
            }
        }
    });
}
