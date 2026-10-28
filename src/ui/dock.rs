use crate::app::NexoraApp;
use crate::audio::AudioChannel;
use crate::recording::RecordingStatus;
use crate::studio::source::{SourceColorTag, SourceItem, SourceKind};
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
        });
    });

    regions.with_body(ui, |ui, body_size| {
        egui::ScrollArea::vertical()
            .id_salt("scenes_scroll")
            .max_height(body_size.y)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut select_preview = None;
                let preview_id = app.studio.preview_scene_id;
                let program_id = app.studio.program_scene_id;

                for (idx, scene) in app.studio.scenes.iter().enumerate() {
                    let is_prv = scene.id == preview_id;
                    let is_pgm = scene.id == program_id;

                    ui.horizontal(|ui| {
                        if is_prv {
                            ui.label(
                                RichText::new("PRV")
                                    .small()
                                    .strong()
                                    .color(p.accent_preview),
                            );
                        }
                        if is_pgm {
                            ui.label(
                                RichText::new("PGM")
                                    .small()
                                    .strong()
                                    .color(p.accent_program),
                            );
                        }
                        if ui
                            .selectable_label(is_prv, &scene.name)
                            .on_hover_text(scene.top_display_label(idx + 1))
                            .clicked()
                        {
                            select_preview = Some(scene.id);
                        }
                    });
                }

                if let Some(id) = select_preview {
                    app.studio.preview_scene_id = id;
                    app.studio.sync_if_linked();
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
    AddDisplay,
    AddWindow,
    AddCamera,
    AddImage,
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
    Duplicate(Uuid),
    Remove(Uuid),
    Rename(Uuid),
    OpenProperties(Uuid),
}

fn populate_add_source_menu(ui: &mut egui::Ui, action_out: &mut Option<SourceContextAction>) {
    ui.set_min_width(225.0);
    if ui
        .button(format!("{}  Display Capture", icons::DEVICE_DESKTOP))
        .clicked()
    {
        *action_out = Some(SourceContextAction::AddDisplay);
        ui.close_menu();
    }
    if ui
        .button(format!("{}  Window Capture", icons::APP_WINDOW))
        .clicked()
    {
        *action_out = Some(SourceContextAction::AddWindow);
        ui.close_menu();
    }
    if ui
        .button(format!(
            "{}  Video Capture Device (Camera)",
            icons::CAMERA
        ))
        .clicked()
    {
        *action_out = Some(SourceContextAction::AddCamera);
        ui.close_menu();
    }
    if ui
        .button(format!("{}  Image Source", icons::PHOTO))
        .clicked()
    {
        *action_out = Some(SourceContextAction::AddImage);
        ui.close_menu();
    }
}

fn populate_source_context_menu(
    ui: &mut egui::Ui,
    src_id: Uuid,
    is_locked: bool,
    current_color: SourceColorTag,
    action_out: &mut Option<SourceContextAction>,
) {
    ui.set_min_width(220.0);

    ui.menu_button(format!("{}  Add Source", icons::PLUS), |ui| {
        populate_add_source_menu(ui, action_out);
    });

    ui.separator();

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
        ui.set_min_width(155.0);
        if ui
            .button(format!("{}  Move Up", icons::ARROW_UP))
            .clicked()
        {
            *action_out = Some(SourceContextAction::MoveUp(src_id));
            ui.close_menu();
        }
        if ui
            .button(format!("{}  Move Down", icons::ARROW_DOWN))
            .clicked()
        {
            *action_out = Some(SourceContextAction::MoveDown(src_id));
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Move to Top").clicked() {
            *action_out = Some(SourceContextAction::MoveTop(src_id));
            ui.close_menu();
        }
        if ui.button("Move to Bottom").clicked() {
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

                for src in &mut scene.sources {
                    let is_sel = cur_sel == Some(src.id);
                    let src_id = src.id;
                    let is_locked = src.locked;
                    let current_color = src.color_tag;

                    let row_frame = if let Some(tint) = color_tag_tint(src.color_tag) {
                        egui::Frame::new()
                            .fill(tint)
                            .corner_radius(CornerRadius::same(3))
                            .inner_margin(egui::Margin::symmetric(3, 1))
                    } else {
                        egui::Frame::new().inner_margin(egui::Margin::symmetric(3, 1))
                    };

                    row_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let label_text =
                                format!("{}  {}", src.kind.tabler_icon(), src.name);
                            let row_resp = ui
                                .selectable_label(is_sel, label_text)
                                .on_hover_text(format!(
                                    "{}\n(Right-click for options • Double-click for Properties)",
                                    src.kind.summary()
                                ));

                            if row_resp.clicked() {
                                new_selected = Some(src_id);
                            }
                            if row_resp.double_clicked() {
                                new_selected = Some(src_id);
                                pending_action =
                                    Some(SourceContextAction::OpenProperties(src_id));
                            }

                            row_resp.context_menu(|ui| {
                                populate_source_context_menu(
                                    ui,
                                    src_id,
                                    is_locked,
                                    current_color,
                                    &mut pending_action,
                                );
                            });

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let lock_label = if src.locked {
                                    icons::LOCK
                                } else {
                                    icons::LOCK_OPEN
                                };
                                if ui
                                    .small_button(lock_label)
                                    .on_hover_text("Lock / Unlock position on canvas")
                                    .clicked()
                                {
                                    src.locked = !src.locked;
                                    vis_changed = true;
                                }

                                let vis_label = if src.visible {
                                    icons::EYE
                                } else {
                                    icons::EYE_OFF
                                };
                                if ui
                                    .small_button(vis_label)
                                    .on_hover_text("Show / Hide this source")
                                    .clicked()
                                {
                                    src.visible = !src.visible;
                                    vis_changed = true;
                                }
                            });
                        });
                    });
                }
                scene.selected_source_id = new_selected;
                if vis_changed {
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
                egui::Button::new(RichText::new(format!(" {} ", icons::ARROW_UP)).small())
                    .min_size(Vec2::new(26.0, 24.0)),
            )
            .on_hover_text("Move Source Up")
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
            .on_hover_text("Move Source Down")
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
                "Image Source",
                SourceKind::ImageSource {
                    file_path: "assets/watermark_overlay.png".to_string(),
                    opacity: 0.90,
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
        let strip_w = (body_size.x - 4.0).max(120.0);
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
