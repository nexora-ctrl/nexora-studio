use crate::app::{NexoraApp, PreviewDragMode};
use crate::recording::RecordingStatus;
use crate::studio::scene::Scene;
use crate::studio::source::{
    SlideTransitionEffect, SourceItem, SourceKind, SourceTransform, TextAnimationEffect,
    TimeBadgePosition, TimeFontFamily, TimeSourceMode, TimeVisualEffect, TvBannerStyle,
};
use crate::ui::icons;
use crate::ui::theme::ThemePalette;
use egui::{
    Align2, Color32, CornerRadius, CursorIcon, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2,
};

pub fn render_preview_workspace(ui: &mut egui::Ui, app: &mut NexoraApp, time_sec: f64) {
    let p = ThemePalette::for_mode(app.settings.theme);

    // Compact Workspace Header Toolbar (Zoom, Fit-to-Window, Safe Margins, Quick Transform)
    egui::Frame::new()
        .fill(p.bg_panel)
        .stroke(Stroke::new(1.0_f32, p.border_subtle))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("WORKSPACE")
                        .small()
                        .strong()
                        .color(p.text_muted),
                );
                ui.separator();

                if ui
                    .selectable_label(app.settings.preview_fit_to_window, "Fit to Window")
                    .on_hover_text("Automatically scale 16:9 canvas while preserving aspect ratio")
                    .clicked()
                {
                    app.settings.preview_fit_to_window = true;
                }

                for (zoom_val, label) in [(0.75, "75%"), (1.0, "100%"), (1.15, "115%")] {
                    let active = !app.settings.preview_fit_to_window
                        && (app.settings.preview_zoom - zoom_val).abs() < 0.02;
                    if ui.selectable_label(active, label).clicked() {
                        app.settings.preview_fit_to_window = false;
                        app.settings.preview_zoom = zoom_val;
                    }
                }

                ui.separator();
                ui.checkbox(&mut app.settings.show_safe_margins, "Safe Guides")
                    .on_hover_text("Show broadcast 90% action-safe guide overlay on Preview");

                ui.separator();

                // Quick Transform & Crop Controls for Selected Source in Preview
                let has_selected = app.studio.preview_scene().selected_source().is_some();
                ui.add_enabled_ui(has_selected, |ui| {
                    ui.label(RichText::new("Transform:").small().color(p.text_secondary));
                    if ui
                        .button("Fit Fullscreen")
                        .on_hover_text("Reset selected source to fill 16:9 canvas and clear crop")
                        .clicked()
                    {
                        if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                            src.transform.reset_full_canvas();
                        }
                        app.studio.sync_if_linked();
                    }
                    if ui
                        .button("PiP Corner")
                        .on_hover_text("Place selected source as Picture-in-Picture in bottom-right")
                        .clicked()
                    {
                        if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                            src.transform.preset_pip_bottom_right();
                        }
                        app.studio.sync_if_linked();
                    }

                    let crop_btn_active = app.preview_crop_mode || ui.input(|i| i.modifiers.alt);
                    let crop_btn_text = if crop_btn_active {
                        "✂ Crop Mode: ON (Drag Edges)"
                    } else {
                        "✂ Crop (Hold Alt / Click)"
                    };
                    if ui
                        .selectable_label(crop_btn_active, crop_btn_text)
                        .on_hover_text(
                            "OBS-Style Crop: Hold ALT while dragging any handle in Preview, or toggle this button to crop edges directly on canvas",
                        )
                        .clicked()
                    {
                        app.preview_crop_mode = !app.preview_crop_mode;
                    }

                    let has_crop = app
                        .studio
                        .preview_scene()
                        .selected_source()
                        .map(|s| s.transform.has_crop())
                        .unwrap_or(false);
                    if has_crop
                        && ui
                            .button("Reset Crop")
                            .on_hover_text("Remove all crop margins on selected source")
                            .clicked()
                    {
                        if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                            src.transform.reset_crop();
                        }
                        app.studio.sync_if_linked();
                    }

                    if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                        ui.label(RichText::new("Scale").small().color(p.text_muted));
                        if ui
                            .add(
                                egui::Slider::new(&mut src.transform.scale, 0.10..=1.50)
                                    .show_value(true)
                                    .fixed_decimals(2),
                            )
                            .changed()
                        {
                            app.studio.sync_if_linked();
                        }
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (w, h) = app.settings.video_preset.dimensions();
                    let fps = app.settings.video_preset.fps();
                    ui.label(
                        RichText::new(format!("Canvas: {}×{} ({} FPS) • 16:9", w, h, fps))
                            .small()
                            .monospace()
                            .color(p.text_secondary),
                    );
                });
            });
        });

    ui.add_space(4.0);

    let avail = ui.available_size();

    if app.settings.panels.studio_mode {
        // Studio Mode: Dual Monitors (Preview | Transition Bridge | Program)
        let center_bridge_w = 88.0;
        let side_w = ((avail.x - center_bridge_w - 14.0) * 0.5).max(180.0);
        let panel_h = avail.y.max(160.0);

        ui.horizontal(|ui| {
            // 1. Left Viewport: PREVIEW (Editable Scene)
            ui.allocate_ui_with_layout(
                Vec2::new(side_w, panel_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_max_size(Vec2::new(side_w, panel_h));
                    render_single_viewport_card(
                        ui,
                        app,
                        ViewportRole::Preview,
                        time_sec,
                        Vec2::new(side_w, panel_h),
                    );
                },
            );

            // 2. Center Studio Transition Column
            ui.allocate_ui_with_layout(
                Vec2::new(center_bridge_w, panel_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_max_size(Vec2::new(center_bridge_w, panel_h));
                    render_transition_bridge(ui, app, panel_h);
                },
            );

            // 3. Right Viewport: PROGRAM (Final Recorded Output)
            ui.allocate_ui_with_layout(
                Vec2::new(side_w, panel_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_max_size(Vec2::new(side_w, panel_h));
                    render_single_viewport_card(
                        ui,
                        app,
                        ViewportRole::Program,
                        time_sec,
                        Vec2::new(side_w, panel_h),
                    );
                },
            );
        });
    } else {
        // Single Viewport Mode (Program / Preview Combined)
        render_single_viewport_card(ui, app, ViewportRole::Preview, time_sec, avail);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewportRole {
    Preview,
    Program,
}

fn render_transition_bridge(ui: &mut egui::Ui, app: &mut NexoraApp, height: f32) {
    use crate::studio::TransitionEffect;
    let p = ThemePalette::for_mode(app.settings.theme);
    let pending = app.studio.has_pending_preview_changes();

    egui::Frame::new()
        .fill(p.bg_panel)
        .stroke(Stroke::new(1.0_f32, p.border_subtle))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(6, 8))
        .show(ui, |ui| {
            ui.set_min_height((height - 16.0).max(120.0));
            ui.vertical_centered(|ui| {
                ui.add_space((height * 0.16).max(6.0));

                ui.label(
                    RichText::new("TRANSITION")
                        .small()
                        .strong()
                        .color(p.text_muted),
                );

                ui.add_space(6.0);

                let action_label = match app.studio.transition_effect {
                    TransitionEffect::Cut => format!("CUT {}", icons::ARROW_RIGHT),
                    TransitionEffect::Fade => format!("FADE {}", icons::ARROW_RIGHT),
                };

                let trans_btn = egui::Button::new(
                    RichText::new(action_label)
                        .strong()
                        .size(11.5)
                        .color(Color32::WHITE),
                )
                .fill(if pending {
                    p.accent_primary
                } else {
                    p.border_strong
                })
                .min_size(Vec2::new(74.0, 30.0));

                if ui
                    .add(trans_btn)
                    .on_hover_text(
                        "Send current Preview scene & layout to Program Output using the selected transition effect",
                    )
                    .clicked()
                {
                    app.studio.transition_preview_to_program();
                }

                ui.add_space(6.0);
                ui.label(
                    RichText::new("Effect:")
                        .size(10.0)
                        .color(p.text_secondary),
                );
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    let is_cut = app.studio.transition_effect == TransitionEffect::Cut;
                    if ui
                        .selectable_label(is_cut, RichText::new("Cut").size(10.5))
                        .on_hover_text("Instant Cut transition (0 ms)")
                        .clicked()
                    {
                        app.studio.transition_effect = TransitionEffect::Cut;
                    }
                    let is_fade = app.studio.transition_effect == TransitionEffect::Fade;
                    if ui
                        .selectable_label(is_fade, RichText::new("Fade").size(10.5))
                        .on_hover_text("Smooth Crossfade transition")
                        .clicked()
                    {
                        app.studio.transition_effect = TransitionEffect::Fade;
                    }
                });

                if app.studio.transition_effect == TransitionEffect::Fade {
                    ui.add_space(2.0);
                    ui.add(
                        egui::DragValue::new(&mut app.studio.transition_duration_ms)
                            .range(100..=1500)
                            .speed(10)
                            .suffix(" ms"),
                    )
                    .on_hover_text("Fade transition duration in milliseconds");
                }

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);

                if ui
                    .checkbox(&mut app.studio.auto_sync_program, "Link")
                    .on_hover_text(
                        "When enabled, Preview edits immediately apply to Program",
                    )
                    .changed()
                {
                    app.studio.sync_if_linked();
                }

                ui.add_space(4.0);
                if pending {
                    ui.label(
                        RichText::new("● Unapplied")
                            .small()
                            .color(p.pause_amber),
                    );
                } else {
                    ui.label(
                        RichText::new("✓ Synced")
                            .small()
                            .color(p.meter_ok),
                    );
                }
            });
        });
}

fn render_single_viewport_card(
    ui: &mut egui::Ui,
    app: &mut NexoraApp,
    role: ViewportRole,
    time_sec: f64,
    alloc: Vec2,
) {
    let p = ThemePalette::for_mode(app.settings.theme);
    let is_program = role == ViewportRole::Program;

    let header_accent = if is_program {
        if app.recording.status == RecordingStatus::Recording {
            p.rec_red
        } else {
            p.accent_program
        }
    } else {
        p.accent_preview
    };

    egui::Frame::new()
        .fill(p.bg_panel)
        .stroke(Stroke::new(
            if is_program && app.recording.status == RecordingStatus::Recording {
                1.5
            } else {
                1.0_f32
            },
            if is_program && app.recording.status == RecordingStatus::Recording {
                p.rec_red
            } else {
                p.border_subtle
            },
        ))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(egui::Margin::same(6))
        .show(ui, |ui| {
            let inner = Vec2::new((alloc.x - 14.0).max(120.0), (alloc.y - 14.0).max(120.0));
            ui.set_min_size(inner);
            ui.set_max_size(inner);

            // Viewport Header Bar
            ui.horizontal(|ui| {
                egui::Frame::new()
                    .fill(header_accent.gamma_multiply(0.20))
                    .stroke(Stroke::new(1.0_f32, header_accent))
                    .corner_radius(CornerRadius::same(3))
                    .inner_margin(egui::Margin::symmetric(6, 1))
                    .show(ui, |ui| {
                        let tag = match role {
                            ViewportRole::Preview => "PREVIEW • EDIT",
                            ViewportRole::Program => {
                                if app.recording.status == RecordingStatus::Recording {
                                    "PROGRAM • ON AIR (REC)"
                                } else {
                                    "PROGRAM • OUTPUT"
                                }
                            }
                        };
                        ui.label(RichText::new(tag).small().color(header_accent));
                    });

                let scene_top_label = match role {
                    ViewportRole::Preview => app.studio.preview_top_label(),
                    ViewportRole::Program => app.studio.program_top_label(),
                };
                let source_count = match role {
                    ViewportRole::Preview => app.studio.preview_scene().sources.len(),
                    ViewportRole::Program => app.studio.program_scene().sources.len(),
                };

                ui.label(
                    RichText::new(&scene_top_label)
                        .size(12.0)
                        .color(p.text_primary),
                );

                let maybe_pres = {
                    let mut res = None;
                    let target_scene = match role {
                        ViewportRole::Preview => app.studio.preview_scene(),
                        ViewportRole::Program => app.studio.program_scene(),
                    };
                    for s in &target_scene.sources {
                        if !s.visible {
                            continue;
                        }
                        match &s.kind {
                            SourceKind::PowerPointSource { slide_index, slide_count, slides, .. } => {
                                let total = if *slide_count > 0 { *slide_count } else { slides.len() };
                                if total > 0 {
                                    res = Some((*slide_index, total, "Slide"));
                                    break;
                                }
                            }
                            SourceKind::PdfSource { page_index, page_count, pages, .. } => {
                                let total = if *page_count > 0 { *page_count } else { pages.len() };
                                if total > 0 {
                                    res = Some((*page_index, total, "Halaman"));
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    if res.is_none() {
                        for sc in &app.studio.scenes {
                            for s in &sc.sources {
                                if !s.visible {
                                    continue;
                                }
                                match &s.kind {
                                    SourceKind::PowerPointSource { slide_index, slide_count, slides, .. } => {
                                        let total = if *slide_count > 0 { *slide_count } else { slides.len() };
                                        if total > 0 {
                                            res = Some((*slide_index, total, "Slide"));
                                            break;
                                        }
                                    }
                                    SourceKind::PdfSource { page_index, page_count, pages, .. } => {
                                        let total = if *page_count > 0 { *page_count } else { pages.len() };
                                        if total > 0 {
                                            res = Some((*page_index, total, "Halaman"));
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            if res.is_some() {
                                break;
                            }
                        }
                    }
                    res
                };

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{} layer(s)", source_count))
                            .small()
                            .color(p.text_muted),
                    );

                    if let Some((cur_idx, total_count, pres_label)) = maybe_pres {
                        ui.add_space(8.0);
                        let next_btn = egui::Button::new(RichText::new("Next ▶").size(11.0))
                            .min_size(Vec2::new(48.0, 19.0));
                        if ui.add(next_btn).clicked() {
                            app.advance_presentation_slide(1);
                        }

                        ui.label(
                            RichText::new(format!("{} {}/{}", pres_label, cur_idx + 1, total_count))
                                .size(11.0)
                                .color(p.accent_preview)
                                .strong(),
                        );

                        let prev_btn = egui::Button::new(RichText::new("◀ Prev").size(11.0))
                            .min_size(Vec2::new(48.0, 19.0));
                        if ui.add(prev_btn).clicked() {
                            app.advance_presentation_slide(-1);
                        }
                    }
                });
            });

            ui.add_space(4.0);

            // Aspect-Ratio Preserved 16:9 Viewport Area
            let well_size = ui.available_size();
            let (well_rect, response) =
                ui.allocate_exact_size(well_size, Sense::click_and_drag());
            let painter = ui.painter_at(well_rect);

            // Fill outer letterbox well
            painter.rect_filled(well_rect, CornerRadius::same(4), p.bg_canvas_well);

            // Calculate 16:9 aspect-locked canvas rectangle
            let aspect = 16.0 / 9.0;
            let zoom = if app.settings.preview_fit_to_window {
                1.0
            } else {
                app.settings.preview_zoom
            };
            let pad = 8.0;
            let max_w = (well_rect.width() - pad * 2.0).max(64.0) * zoom;
            let max_h = (well_rect.height() - pad * 2.0).max(36.0) * zoom;

            let (canvas_w, canvas_h) = if max_w / max_h > aspect {
                (max_h * aspect, max_h)
            } else {
                (max_w, max_w / aspect)
            };

            let canvas_rect = Rect::from_center_size(well_rect.center(), Vec2::new(canvas_w, canvas_h));

            match role {
                ViewportRole::Preview => app.preview_canvas_rect = Some(canvas_rect),
                ViewportRole::Program => app.program_canvas_rect = Some(canvas_rect),
            }

            // Base stage surface
            painter.rect_filled(
                canvas_rect,
                CornerRadius::same(2),
                Color32::from_rgb(16, 20, 28),
            );

            // Subtle technical studio grid on canvas
            draw_canvas_grid(&painter, canvas_rect);

            // Clone scene snapshot to draw layers cleanly
            let scene_clone: Scene = match role {
                ViewportRole::Preview => app.studio.preview_scene().clone(),
                ViewportRole::Program => app.studio.program_scene().clone(),
            };

            // Interactive OBS-Style Selection, Move, 8-Handle Resize, and Alt-Crop in Preview
            if role == ViewportRole::Preview
                && canvas_rect.width() > 10.0
                && canvas_rect.height() > 10.0
                && !app.is_any_modal_open()
                && !app.eyedropper_active
            {
                let alt_held = ui.input(|i| i.modifiers.alt) || app.preview_crop_mode;
                let shift_held = ui.input(|i| i.modifiers.shift);
                let pointer_pos = response.interact_pointer_pos().or_else(|| ui.input(|i| i.pointer.hover_pos()));

                // Smooth real-time redraw saat mouse bergerak di canvas Preview
                if response.hovered() && ui.input(|i| i.pointer.is_moving()) {
                    ui.ctx().request_repaint();
                }

                // Compute current selected source's screen rect for handle hit-testing
                let selected_screen_rect = scene_clone
                    .selected_source()
                    .filter(|s| s.visible)
                    .map(|s| compute_layer_screen_rect(canvas_rect, &s.transform));

                // Click, right-click, or drag start: hit-test handles of selected source first, then top-most visible layer
                if ui.input(|i| i.pointer.primary_pressed()) && response.hovered() {
                    if let Some(ptr) = pointer_pos {
                        if let Some(sel_rect) = selected_screen_rect {
                            let matched = hit_test_handles(sel_rect, ptr, alt_held);
                            if matched != PreviewDragMode::None && matched != PreviewDragMode::Move {
                                app.preview_drag_mode = matched;
                            }
                        }
                    }
                }

                if response.clicked()
                    || response.drag_started()
                    || (response.dragged() && app.preview_drag_mode == PreviewDragMode::None)
                    || response.secondary_clicked()
                {
                    let start_pos = ui.input(|i| i.pointer.press_origin()).or(pointer_pos);
                    if let Some(ptr) = start_pos {
                        let mut matched_handle = PreviewDragMode::None;
                        if !response.secondary_clicked() {
                            if let Some(sel_rect) = selected_screen_rect {
                                matched_handle = hit_test_handles(sel_rect, ptr, alt_held);
                            }
                        }

                        if matched_handle != PreviewDragMode::None && matched_handle != PreviewDragMode::Move {
                            app.preview_drag_mode = matched_handle;
                        } else {
                            // Hit-test visible sources from top-most (index 0, top of Source list) to bottom-most (back)
                            let mut hit_id = None;
                            for src in scene_clone.sources.iter() {
                                if !src.visible {
                                    continue;
                                }
                                match &src.kind {
                                    SourceKind::AudioInputCapture { .. }
                                    | SourceKind::AudioOutputCapture { .. }
                                    | SourceKind::ApplicationAudioCapture { .. } => continue,
                                    SourceKind::MediaSource { file_path, .. }
                                        if SourceKind::is_audio_only_media(file_path) =>
                                    {
                                        continue;
                                    }
                                    _ => {}
                                }
                                let r = compute_layer_screen_rect(canvas_rect, &src.transform);
                                if r.contains(ptr) {
                                    hit_id = Some(src.id);
                                    break;
                                }
                            }
                            if let Some(id) = hit_id {
                                app.studio.preview_scene_mut().selected_source_id = Some(id);
                                if !response.secondary_clicked() {
                                    app.preview_drag_mode = PreviewDragMode::Move;
                                } else {
                                    app.preview_drag_mode = PreviewDragMode::None;
                                }
                            } else if !response.secondary_clicked() {
                                app.preview_drag_mode = PreviewDragMode::None;
                            }
                        }
                    }
                }

                // Mouse wheel over selected source in Preview scales smoothly
                if response.hovered() {
                    let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
                    if scroll_y.abs() > 0.5 {
                        if let Some(ptr) = pointer_pos {
                            if let Some(sel_rect) = selected_screen_rect {
                                if sel_rect.contains(ptr) {
                                    if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                                        if !src.locked {
                                            let factor = (scroll_y * 0.0015).clamp(-0.08, 0.08);
                                            src.transform.scale = (src.transform.scale + factor).clamp(0.08, 2.0);
                                            app.studio.sync_if_linked();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Apply active drag delta (Move, 8-Handle Resize, or Alt-Crop)
                if response.dragged() && app.preview_drag_mode != PreviewDragMode::None {
                    let delta = response.drag_delta();
                    if delta.x.abs() > 0.0001 || delta.y.abs() > 0.0001 {
                        let mode = app.preview_drag_mode;
                        if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                            if !src.locked {
                                apply_transform_drag(
                                    &mut src.transform,
                                    mode,
                                    delta,
                                    canvas_rect.size(),
                                    shift_held,
                                );
                            }
                        }
                        app.studio.sync_if_linked();
                    }
                }

                if response.drag_stopped()
                    || !ui.input(|i| i.pointer.primary_down() || i.pointer.secondary_down())
                {
                    app.preview_drag_mode = PreviewDragMode::None;
                }

                // Update cursor icon secara instan (zero latency) berdasarkan hover / active drag state terkini
                if let Some(ptr) = pointer_pos {
                    if app.preview_drag_mode == PreviewDragMode::None {
                        if response.hovered() && well_rect.contains(ptr) {
                            if let Some(sel_src) = scene_clone.selected_source() {
                                if !sel_src.locked {
                                    if let Some(sel_rect) = selected_screen_rect {
                                        let hover_mode = hit_test_handles(sel_rect, ptr, alt_held);
                                        match hover_mode {
                                            PreviewDragMode::ResizeTopLeft | PreviewDragMode::ResizeBottomRight => {
                                                ui.ctx().set_cursor_icon(CursorIcon::ResizeNwSe);
                                            }
                                            PreviewDragMode::ResizeTopRight | PreviewDragMode::ResizeBottomLeft => {
                                                ui.ctx().set_cursor_icon(CursorIcon::ResizeNeSw);
                                            }
                                            PreviewDragMode::ResizeTop
                                            | PreviewDragMode::ResizeBottom
                                            | PreviewDragMode::CropTop
                                            | PreviewDragMode::CropBottom => {
                                                ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
                                            }
                                            PreviewDragMode::ResizeLeft
                                            | PreviewDragMode::ResizeRight
                                            | PreviewDragMode::CropLeft
                                            | PreviewDragMode::CropRight => {
                                                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                                            }
                                            PreviewDragMode::Move => {
                                                ui.ctx().set_cursor_icon(CursorIcon::Default);
                                            }
                                            PreviewDragMode::None => {
                                                ui.ctx().set_cursor_icon(CursorIcon::Default);
                                            }
                                        }
                                    }
                                } else {
                                    ui.ctx().set_cursor_icon(CursorIcon::Default);
                                }
                            } else {
                                ui.ctx().set_cursor_icon(CursorIcon::Default);
                            }
                        }
                    } else if app.preview_drag_mode == PreviewDragMode::Move {
                        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                    } else {
                        match app.preview_drag_mode {
                            PreviewDragMode::ResizeTopLeft | PreviewDragMode::ResizeBottomRight => {
                                ui.ctx().set_cursor_icon(CursorIcon::ResizeNwSe);
                            }
                            PreviewDragMode::ResizeTopRight | PreviewDragMode::ResizeBottomLeft => {
                                ui.ctx().set_cursor_icon(CursorIcon::ResizeNeSw);
                            }
                            PreviewDragMode::ResizeTop
                            | PreviewDragMode::ResizeBottom
                            | PreviewDragMode::CropTop
                            | PreviewDragMode::CropBottom => {
                                ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
                            }
                            PreviewDragMode::ResizeLeft
                            | PreviewDragMode::ResizeRight
                            | PreviewDragMode::CropLeft
                            | PreviewDragMode::CropRight => {
                                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                            }
                            _ => {}
                        }
                    }
                }

                // Keyboard hotkeys for transforming selected source in Preview canvas
                if role == ViewportRole::Preview
                    && !ui.ctx().wants_keyboard_input()
                    && !app.is_any_modal_open()
                {
                    let ctrl = ui.input(|i| i.modifiers.command || i.modifiers.ctrl);
                    if ctrl && ui.input(|i| i.key_pressed(egui::Key::V)) {
                        app.paste_copied_source();
                    } else if ctrl && ui.input(|i| i.key_pressed(egui::Key::C)) {
                        app.copy_selected_source();
                    } else if ctrl {
                        if ui.input(|i| i.key_pressed(egui::Key::F)) {
                            if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                                src.transform.fit_to_screen();
                                app.studio.sync_if_linked();
                            }
                        } else if ui.input(|i| i.key_pressed(egui::Key::S)) {
                            if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                                src.transform.stretch_to_screen();
                                app.studio.sync_if_linked();
                            }
                        } else if ui.input(|i| i.key_pressed(egui::Key::D)) {
                            if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                                src.transform.center_to_screen();
                                app.studio.sync_if_linked();
                            }
                        } else if ui.input(|i| i.key_pressed(egui::Key::R)) {
                            if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                                src.transform.reset_transform();
                                app.studio.sync_if_linked();
                            }
                        }
                    }
                }
            }

            // Presenter pointer click navigation in Program (Output) Viewport
            if role == ViewportRole::Program
                && canvas_rect.width() > 10.0
                && canvas_rect.height() > 10.0
                && !app.is_any_modal_open()
                && !app.eyedropper_active
            {
                let has_presentation = scene_clone.sources.iter().any(|s| {
                    s.visible
                        && matches!(
                            s.kind,
                            SourceKind::PowerPointSource { .. } | SourceKind::PdfSource { .. }
                        )
                });
                if has_presentation {
                    if response.clicked() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            let rel_x = (pos.x - canvas_rect.min.x) / canvas_rect.width();
                            if rel_x < 0.20 {
                                app.advance_presentation_slide(-1);
                            } else {
                                app.advance_presentation_slide(1);
                            }
                        }
                    }

                    if response.hovered() {
                        let scroll_y = ui.input(|i| i.raw_scroll_delta.y);
                        if scroll_y.abs() > 15.0 {
                            if scroll_y < 0.0 {
                                app.advance_presentation_slide(1);
                            } else {
                                app.advance_presentation_slide(-1);
                            }
                        }
                    }
                }
            }

            // Re-fetch updated scene snapshot after interactive drag
            let scene_clone: Scene = match role {
                ViewportRole::Preview => app.studio.preview_scene().clone(),
                ViewportRole::Program => app.studio.program_scene().clone(),
            };

            // Render visible source layers clipped cleanly to the 16:9 canvas
            let visible_count = scene_clone.sources.iter().filter(|s| s.visible).count();
            if visible_count == 0 {
                painter.text(
                    canvas_rect.center(),
                    Align2::CENTER_CENTER,
                    "No visible sources in scene\nUse the Sources panel below to add or enable capture layers",
                    FontId::proportional(12.0),
                    Color32::from_rgb(135, 145, 165),
                );
            } else {
                let egui_ctx = ui.ctx().clone();
                let canvas_painter = painter.with_clip_rect(canvas_rect);

                let fade_progress = if role == ViewportRole::Program {
                    app.studio.active_fade_progress()
                } else {
                    None
                };

                if let Some(prog_alpha) = fade_progress {
                    if let Some(from_scene) = app.studio.transition_from_snapshot.clone() {
                        let out_alpha = (1.0 - prog_alpha).clamp(0.0, 1.0);
                        // Draw from bottom of Source list (back layer) to top of Source list (front layer)
                        for src in from_scene.sources.iter().rev() {
                            if !src.visible {
                                continue;
                            }
                            let raw_layer_rect =
                                compute_layer_screen_rect(canvas_rect, &src.transform);
                            let clipped_rect = raw_layer_rect.intersect(canvas_rect);
                            if clipped_rect.width() <= 2.0 || clipped_rect.height() <= 2.0 {
                                continue;
                            }
                            render_source_item_on_canvas(
                                &canvas_painter,
                                &egui_ctx,
                                app,
                                src,
                                raw_layer_rect,
                                time_sec,
                                out_alpha,
                            );
                        }
                    }
                }

                let active_alpha = fade_progress.unwrap_or(1.0);

                // Draw from bottom of Source list (back layer) to top of Source list (front layer)
                for src in scene_clone.sources.iter().rev() {
                    if !src.visible {
                        continue;
                    }
                    // Audio-only sources do not draw visual blocks over the video canvas
                    match &src.kind {
                        SourceKind::AudioInputCapture { .. }
                        | SourceKind::AudioOutputCapture { .. }
                        | SourceKind::ApplicationAudioCapture { .. } => continue,
                        SourceKind::MediaSource { file_path, .. }
                            if SourceKind::is_audio_only_media(file_path) =>
                        {
                            continue;
                        }
                        _ => {}
                    }

                    let raw_layer_rect = compute_layer_screen_rect(canvas_rect, &src.transform);
                    let clipped_rect = raw_layer_rect.intersect(canvas_rect);

                    if clipped_rect.width() <= 2.0 || clipped_rect.height() <= 2.0 {
                        continue;
                    }

                    render_source_item_on_canvas(
                        &canvas_painter,
                        &egui_ctx,
                        app,
                        src,
                        raw_layer_rect,
                        time_sec,
                        active_alpha,
                    );
                }

                // Highlight selected source bounding box + 8 OBS resize/crop handles on top in Preview
                if role == ViewportRole::Preview {
                    if let Some(src) = scene_clone.selected_source().filter(|s| s.visible) {
                        let is_audio_only = matches!(
                            &src.kind,
                            SourceKind::AudioInputCapture { .. }
                                | SourceKind::AudioOutputCapture { .. }
                                | SourceKind::ApplicationAudioCapture { .. }
                        ) || matches!(
                            &src.kind,
                            SourceKind::MediaSource { file_path, .. }
                                if SourceKind::is_audio_only_media(file_path)
                        );
                        if !is_audio_only {
                            let raw_layer_rect =
                                compute_layer_screen_rect(canvas_rect, &src.transform);
                            let crop_active =
                                ui.input(|i| i.modifiers.alt) || app.preview_crop_mode;
                            let crop_green = Color32::from_rgb(34, 197, 94);
                            let base_box_color = if src.locked {
                                p.pause_amber
                            } else if crop_active {
                                crop_green
                            } else {
                                p.accent_preview
                            };

                            // Draw base bounding box
                            canvas_painter.rect_stroke(
                                raw_layer_rect,
                                CornerRadius::same(1),
                                Stroke::new(1.5_f32, base_box_color),
                                egui::StrokeKind::Inside,
                            );

                            // Highlight individual cropped edges in OBS Green
                            let t = &src.transform;
                            if t.crop_left > 0.002 {
                                canvas_painter.line_segment(
                                    [raw_layer_rect.left_top(), raw_layer_rect.left_bottom()],
                                    Stroke::new(2.5_f32, crop_green),
                                );
                            }
                            if t.crop_right > 0.002 {
                                canvas_painter.line_segment(
                                    [raw_layer_rect.right_top(), raw_layer_rect.right_bottom()],
                                    Stroke::new(2.5_f32, crop_green),
                                );
                            }
                            if t.crop_top > 0.002 {
                                canvas_painter.line_segment(
                                    [raw_layer_rect.left_top(), raw_layer_rect.right_top()],
                                    Stroke::new(2.5_f32, crop_green),
                                );
                            }
                            if t.crop_bottom > 0.002 {
                                canvas_painter.line_segment(
                                    [
                                        raw_layer_rect.left_bottom(),
                                        raw_layer_rect.right_bottom(),
                                    ],
                                    Stroke::new(2.5_f32, crop_green),
                                );
                            }

                            if !src.locked {
                                // 4 Corner Handles (Resize / Aspect Scale)
                                for corner in [
                                    raw_layer_rect.left_top(),
                                    raw_layer_rect.right_top(),
                                    raw_layer_rect.left_bottom(),
                                    raw_layer_rect.right_bottom(),
                                ] {
                                    let handle = Rect::from_center_size(corner, Vec2::splat(7.5));
                                    canvas_painter.rect_filled(
                                        handle,
                                        CornerRadius::same(1),
                                        base_box_color,
                                    );
                                    canvas_painter.rect_stroke(
                                        handle,
                                        CornerRadius::same(1),
                                        Stroke::new(1.0_f32, Color32::WHITE),
                                        egui::StrokeKind::Inside,
                                    );
                                }

                                // 4 Edge Midpoint Handles (Stretch or Alt-Crop like OBS Studio)
                                let edge_points = [
                                    (
                                        Pos2::new(
                                            raw_layer_rect.center().x,
                                            raw_layer_rect.min.y,
                                        ),
                                        t.crop_top > 0.002 || crop_active,
                                    ),
                                    (
                                        Pos2::new(
                                            raw_layer_rect.center().x,
                                            raw_layer_rect.max.y,
                                        ),
                                        t.crop_bottom > 0.002 || crop_active,
                                    ),
                                    (
                                        Pos2::new(
                                            raw_layer_rect.min.x,
                                            raw_layer_rect.center().y,
                                        ),
                                        t.crop_left > 0.002 || crop_active,
                                    ),
                                    (
                                        Pos2::new(
                                            raw_layer_rect.max.x,
                                            raw_layer_rect.center().y,
                                        ),
                                        t.crop_right > 0.002 || crop_active,
                                    ),
                                ];
                                for (pt, is_crop_edge) in edge_points {
                                    let h_color = if is_crop_edge {
                                        crop_green
                                    } else {
                                        base_box_color
                                    };
                                    let handle = Rect::from_center_size(pt, Vec2::splat(6.5));
                                    canvas_painter.rect_filled(
                                        handle,
                                        CornerRadius::same(1),
                                        h_color,
                                    );
                                    canvas_painter.rect_stroke(
                                        handle,
                                        CornerRadius::same(1),
                                        Stroke::new(1.0_f32, Color32::WHITE),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // Broadcast Safe Margins Guide (90% action safe) in Preview
            if role == ViewportRole::Preview && app.settings.show_safe_margins {
                let safe_rect = canvas_rect.shrink2(Vec2::new(
                    canvas_rect.width() * 0.05,
                    canvas_rect.height() * 0.05,
                ));
                painter.rect_stroke(
                    safe_rect,
                    CornerRadius::ZERO,
                    Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 28)),
                    egui::StrokeKind::Inside,
                );
            }

            // Outer canvas border
            painter.rect_stroke(
                canvas_rect,
                CornerRadius::same(2),
                Stroke::new(1.2_f32, header_accent.gamma_multiply(0.8)),
                egui::StrokeKind::Outside,
            );

            // Right-click context menu directly on Output Screen (OBS Style)
            let displays = app.capture_catalog.displays.clone();
            let (out_w, out_h) = app.settings.video_preset.dimensions();
            let scene_name = scene_clone.name.clone();

            response.context_menu(|ui| {
                ui.set_min_width(220.0);

                if role == ViewportRole::Preview {
                    let maybe_sel_src = app.studio.preview_scene().selected_source().cloned();
                    if let Some(sel_src) = maybe_sel_src {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(sel_src.kind.tabler_icon()).size(14.0).color(p.accent_primary));
                            ui.label(RichText::new(&sel_src.name).strong());
                        });
                        ui.separator();

                        if ui
                            .add(egui::Button::new(format!("{}  Copy", icons::COPY)).shortcut_text("Ctrl+C"))
                            .clicked()
                        {
                            app.copy_selected_source();
                            ui.close_menu();
                        }

                        let paste_label = if let Some(cs) = &app.copied_source {
                            format!("{}  Paste '{}'", icons::CLIPBOARD, cs.name)
                        } else {
                            format!("{}  Paste", icons::CLIPBOARD)
                        };
                        if ui
                            .add_enabled(
                                app.copied_source.is_some(),
                                egui::Button::new(paste_label).shortcut_text("Ctrl+V"),
                            )
                            .clicked()
                        {
                            app.paste_copied_source();
                            ui.close_menu();
                        }

                        if ui
                            .button(format!("{}  Duplicate", icons::COPY))
                            .clicked()
                        {
                            app.duplicate_selected_source();
                            ui.close_menu();
                        }

                        ui.separator();

                        ui.menu_button(format!("{}  Transform", icons::TRANSFORM), |ui| {
                            ui.set_min_width(220.0);
                            if ui.button("Rotate 90 degrees CW").clicked() {
                                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                                    src.transform.rotate_cw_90();
                                }
                                app.studio.sync_if_linked();
                                ui.close_menu();
                            }
                            if ui.button("Rotate 90 degrees CCW").clicked() {
                                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                                    src.transform.rotate_ccw_90();
                                }
                                app.studio.sync_if_linked();
                                ui.close_menu();
                            }
                            if ui.button("Rotate 180 degrees").clicked() {
                                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                                    src.transform.rotate_180();
                                }
                                app.studio.sync_if_linked();
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button("Flip Horizontal").clicked() {
                                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                                    src.transform.flip_horizontal();
                                }
                                app.studio.sync_if_linked();
                                ui.close_menu();
                            }
                            if ui.button("Flip Vertical").clicked() {
                                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                                    src.transform.flip_vertical();
                                }
                                app.studio.sync_if_linked();
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.add(egui::Button::new("Fit to screen").shortcut_text("Ctrl+F")).clicked() {
                                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                                    src.transform.fit_to_screen();
                                }
                                app.studio.sync_if_linked();
                                ui.close_menu();
                            }
                            if ui.add(egui::Button::new("Stretch to screen").shortcut_text("Ctrl+S")).clicked() {
                                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                                    src.transform.stretch_to_screen();
                                }
                                app.studio.sync_if_linked();
                                ui.close_menu();
                            }
                            if ui.add(egui::Button::new("Center to screen").shortcut_text("Ctrl+D")).clicked() {
                                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                                    src.transform.center_to_screen();
                                }
                                app.studio.sync_if_linked();
                                ui.close_menu();
                            }
                            if ui.button("Center Vertically").clicked() {
                                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                                    src.transform.center_vertically();
                                }
                                app.studio.sync_if_linked();
                                ui.close_menu();
                            }
                            if ui.button("Center Horizontally").clicked() {
                                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                                    src.transform.center_horizontally();
                                }
                                app.studio.sync_if_linked();
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.add(egui::Button::new(format!("{}  Reset Transform", icons::REFRESH)).shortcut_text("Ctrl+R")).clicked() {
                                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                                    src.transform.reset_transform();
                                }
                                app.studio.sync_if_linked();
                                ui.close_menu();
                            }
                        });

                        ui.menu_button(format!("{}  Order", icons::ARROWS_UP_DOWN), |ui| {
                            ui.set_min_width(180.0);
                            if ui.button(format!("{}  Move Up", icons::ARROW_UP)).clicked() {
                                app.studio.preview_scene_mut().move_selected_source_up();
                                app.studio.sync_if_linked();
                                ui.close_menu();
                            }
                            if ui.button(format!("{}  Move Down", icons::ARROW_DOWN)).clicked() {
                                app.studio.preview_scene_mut().move_selected_source_down();
                                app.studio.sync_if_linked();
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button("Move to Top (Front)").clicked() {
                                app.studio.preview_scene_mut().move_selected_source_to_top();
                                app.studio.sync_if_linked();
                                ui.close_menu();
                            }
                            if ui.button("Move to Bottom (Back)").clicked() {
                                app.studio.preview_scene_mut().move_selected_source_to_bottom();
                                app.studio.sync_if_linked();
                                ui.close_menu();
                            }
                        });

                        if ui.button(format!("{}  Filters...", icons::FILTER)).clicked() {
                            app.show_source_filters_dialog = true;
                            ui.close_menu();
                        }

                        if ui.button(format!("{}  Properties", icons::SETTINGS)).clicked() {
                            app.show_source_properties_dialog = true;
                            ui.close_menu();
                        }

                        ui.separator();
                    } else if let Some(cs) = &app.copied_source {
                        if ui
                            .add(egui::Button::new(format!("{}  Paste '{}'", icons::CLIPBOARD, cs.name)).shortcut_text("Ctrl+V"))
                            .clicked()
                        {
                            app.paste_copied_source();
                            ui.close_menu();
                        }
                        ui.separator();
                    }
                }

                let projector_label = match role {
                    ViewportRole::Preview => "Open Preview Projector",
                    ViewportRole::Program => "Open Program Projector",
                };

                ui.menu_button(projector_label, |ui| {
                    ui.set_min_width(260.0);
                    for d in &displays {
                        let disp_entry = format!(
                            r"\\.\DISPLAY{}: {}x{} @ {},{}",
                            d.id.replace("MONITOR_", ""),
                            d.width,
                            d.height,
                            d.left,
                            d.top
                        );
                        if ui.button(&disp_entry).clicked() {
                            app.projector_target_label = disp_entry.clone();
                            app.projector_target = Some(crate::app::ProjectorTarget {
                                is_program: role == ViewportRole::Program,
                                is_fullscreen: true,
                                label: disp_entry,
                                left: d.left,
                                top: d.top,
                                width: d.width,
                                height: d.height,
                                snap_frames_remaining: 6,
                            });
                            app.show_projector_window = true;
                            ui.close_menu();
                        }
                    }
                    if ui.button("New window").clicked() {
                        app.projector_target_label = "New Window (Resizable)".to_string();
                        app.projector_target = Some(crate::app::ProjectorTarget {
                            is_program: role == ViewportRole::Program,
                            is_fullscreen: false,
                            label: "New Window (Resizable)".to_string(),
                            left: 120,
                            top: 120,
                            width: 960,
                            height: 540,
                            snap_frames_remaining: 0,
                        });
                        app.show_projector_window = true;
                        ui.close_menu();
                    }
                });

                if app.show_projector_window {
                    if ui.button("Close Projector (Esc)").clicked() {
                        app.show_projector_window = false;
                        app.projector_target = None;
                        ui.close_menu();
                    }
                }

                let screenshot_label = match role {
                    ViewportRole::Preview => "Save Preview Screenshot",
                    ViewportRole::Program => "Save Program Screenshot",
                };

                if ui.button(screenshot_label).clicked() {
                    match crate::platform::save_program_screenshot(
                        &app.settings.output_directory,
                        &scene_name,
                        out_w,
                        out_h,
                    ) {
                        Ok(p) => {
                            app.screenshot_notify = Some((
                                format!("Screenshot saved to {}", p.file_name().unwrap_or_default().to_string_lossy()),
                                std::time::Instant::now(),
                            ));
                            app.media_library.scan_directory(&app.settings.output_directory);
                        }
                        Err(e) => {
                            app.screenshot_notify = Some((format!("Failed to save screenshot: {}", e), std::time::Instant::now()));
                        }
                    }
                    ui.close_menu();
                }
            });
        });
}

fn draw_canvas_grid(painter: &egui::Painter, rect: Rect) {
    let grid_color = Color32::from_rgba_unmultiplied(255, 255, 255, 10);
    let cols = 8;
    let rows = 4;
    for c in 1..cols {
        let x = rect.min.x + rect.width() * (c as f32 / cols as f32);
        painter.line_segment(
            [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
            Stroke::new(1.0_f32, grid_color),
        );
    }
    for r in 1..rows {
        let y = rect.min.y + rect.height() * (r as f32 / rows as f32);
        painter.line_segment(
            [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
            Stroke::new(1.0_f32, grid_color),
        );
    }
}

pub(crate) fn compute_layer_screen_rect(canvas_rect: Rect, transform: &SourceTransform) -> Rect {
    let (vis_x, vis_y, vis_w, vis_h) = transform.visible_norm_rect();
    let min = Pos2::new(
        canvas_rect.min.x + canvas_rect.width() * vis_x,
        canvas_rect.min.y + canvas_rect.height() * vis_y,
    );
    let size = Vec2::new(canvas_rect.width() * vis_w, canvas_rect.height() * vis_h);
    Rect::from_min_size(min, size)
}

fn hit_test_handles(layer_rect: Rect, ptr: Pos2, alt_crop_mode: bool) -> PreviewDragMode {
    let corner_margin_x = 12.0_f32.min(layer_rect.width() * 0.35);
    let corner_margin_y = 12.0_f32.min(layer_rect.height() * 0.35);
    let edge_margin = 7.0_f32;

    let tl = layer_rect.left_top();
    let tr = layer_rect.right_top();
    let bl = layer_rect.left_bottom();
    let br = layer_rect.right_bottom();

    // 1. Prioritas Utama: 4 Sudut (Corner Handles) selalu diuji pertama kali
    if (ptr.x - tl.x).abs() <= corner_margin_x && (ptr.y - tl.y).abs() <= corner_margin_y {
        return PreviewDragMode::ResizeTopLeft;
    }
    if (ptr.x - tr.x).abs() <= corner_margin_x && (ptr.y - tr.y).abs() <= corner_margin_y {
        return PreviewDragMode::ResizeTopRight;
    }
    if (ptr.x - bl.x).abs() <= corner_margin_x && (ptr.y - bl.y).abs() <= corner_margin_y {
        return PreviewDragMode::ResizeBottomLeft;
    }
    if (ptr.x - br.x).abs() <= corner_margin_x && (ptr.y - br.y).abs() <= corner_margin_y {
        return PreviewDragMode::ResizeBottomRight;
    }

    // 2. Prioritas Kedua: 4 Sisi / Edge Handles di antara sudut-sudut
    let edge_x_min = layer_rect.min.x + corner_margin_x;
    let edge_x_max = layer_rect.max.x - corner_margin_x;
    let edge_y_min = layer_rect.min.y + corner_margin_y;
    let edge_y_max = layer_rect.max.y - corner_margin_y;

    // Sisi atas & bawah
    if ptr.x >= edge_x_min && ptr.x <= edge_x_max {
        if (ptr.y - layer_rect.min.y).abs() <= edge_margin {
            return if alt_crop_mode { PreviewDragMode::CropTop } else { PreviewDragMode::ResizeTop };
        }
        if (ptr.y - layer_rect.max.y).abs() <= edge_margin {
            return if alt_crop_mode { PreviewDragMode::CropBottom } else { PreviewDragMode::ResizeBottom };
        }
    }

    // Sisi kiri & kanan
    if ptr.y >= edge_y_min && ptr.y <= edge_y_max {
        if (ptr.x - layer_rect.min.x).abs() <= edge_margin {
            return if alt_crop_mode { PreviewDragMode::CropLeft } else { PreviewDragMode::ResizeLeft };
        }
        if (ptr.x - layer_rect.max.x).abs() <= edge_margin {
            return if alt_crop_mode { PreviewDragMode::CropRight } else { PreviewDragMode::ResizeRight };
        }
    }

    // Titik handle tengah langsung (midpoint handles)
    let top_mid = Pos2::new(layer_rect.center().x, layer_rect.min.y);
    let bot_mid = Pos2::new(layer_rect.center().x, layer_rect.max.y);
    let left_mid = Pos2::new(layer_rect.min.x, layer_rect.center().y);
    let right_mid = Pos2::new(layer_rect.max.x, layer_rect.center().y);

    if (ptr.x - top_mid.x).abs() <= 10.0 && (ptr.y - top_mid.y).abs() <= edge_margin {
        return if alt_crop_mode { PreviewDragMode::CropTop } else { PreviewDragMode::ResizeTop };
    }
    if (ptr.x - bot_mid.x).abs() <= 10.0 && (ptr.y - bot_mid.y).abs() <= edge_margin {
        return if alt_crop_mode { PreviewDragMode::CropBottom } else { PreviewDragMode::ResizeBottom };
    }
    if (ptr.x - left_mid.x).abs() <= edge_margin && (ptr.y - left_mid.y).abs() <= 10.0 {
        return if alt_crop_mode { PreviewDragMode::CropLeft } else { PreviewDragMode::ResizeLeft };
    }
    if (ptr.x - right_mid.x).abs() <= edge_margin && (ptr.y - right_mid.y).abs() <= 10.0 {
        return if alt_crop_mode { PreviewDragMode::CropRight } else { PreviewDragMode::ResizeRight };
    }

    // 3. Prioritas Ketiga: Badan layer dalam (Move / Drag)
    if layer_rect.contains(ptr) {
        return PreviewDragMode::Move;
    }

    PreviewDragMode::None
}

fn apply_transform_drag(
    t: &mut SourceTransform,
    mode: PreviewDragMode,
    delta_px: Vec2,
    canvas_size: Vec2,
    shift_held: bool,
) {
    let (old_vx, old_vy, old_vw, old_vh) = t.visible_norm_rect();
    let (u0, v0, u1, v1) = t.uv_bounds();
    let uv_w = (u1 - u0).max(0.05);
    let uv_h = (v1 - v0).max(0.05);
    let cur_vw = old_vw.max(0.01);
    let cur_vh = old_vh.max(0.01);

    let canvas_w = canvas_size.x.max(1.0);
    let canvas_h = canvas_size.y.max(1.0);
    let dx = delta_px.x / canvas_w;
    let dy = delta_px.y / canvas_h;

    let layer_w_px = (cur_vw * canvas_w).max(8.0);
    let layer_h_px = (cur_vh * canvas_h).max(8.0);
    let diag_sq = layer_w_px * layer_w_px + layer_h_px * layer_h_px;

    match mode {
        PreviewDragMode::Move => {
            t.pos_x = (t.pos_x + dx).clamp(-0.90, 0.96);
            t.pos_y = (t.pos_y + dy).clamp(-0.90, 0.96);
        }
        PreviewDragMode::ResizeBottomRight => {
            if t.lock_aspect && !shift_held {
                let d_scale = (delta_px.x * layer_w_px + delta_px.y * layer_h_px) / diag_sq;
                t.scale = (t.scale * (1.0 + d_scale)).clamp(0.08, 3.0);
            } else {
                let new_vw = (old_vw + dx).clamp(0.04, 2.5);
                let new_vh = (old_vh + dy).clamp(0.04, 2.5);
                t.scale_x = (new_vw / (t.scale * uv_w).max(0.01)).clamp(0.05, 4.0);
                t.scale_y = (new_vh / (t.scale * uv_h).max(0.01)).clamp(0.05, 4.0);
            }
        }
        PreviewDragMode::ResizeTopLeft => {
            let br_x = old_vx + old_vw;
            let br_y = old_vy + old_vh;
            if t.lock_aspect && !shift_held {
                let d_scale = (-delta_px.x * layer_w_px - delta_px.y * layer_h_px) / diag_sq;
                t.scale = (t.scale * (1.0 + d_scale)).clamp(0.08, 3.0);
            } else {
                let new_vw = (old_vw - dx).clamp(0.04, 2.5);
                let new_vh = (old_vh - dy).clamp(0.04, 2.5);
                t.scale_x = (new_vw / (t.scale * uv_w).max(0.01)).clamp(0.05, 4.0);
                t.scale_y = (new_vh / (t.scale * uv_h).max(0.01)).clamp(0.05, 4.0);
            }
            let (_, _, new_vw, new_vh) = t.visible_norm_rect();
            t.pos_x = (br_x - new_vw) - t.effective_width_norm() * u0;
            t.pos_y = (br_y - new_vh) - t.effective_height_norm() * v0;
        }
        PreviewDragMode::ResizeTopRight => {
            let bl_x = old_vx;
            let bl_y = old_vy + old_vh;
            if t.lock_aspect && !shift_held {
                let d_scale = (delta_px.x * layer_w_px - delta_px.y * layer_h_px) / diag_sq;
                t.scale = (t.scale * (1.0 + d_scale)).clamp(0.08, 3.0);
            } else {
                let new_vw = (old_vw + dx).clamp(0.04, 2.5);
                let new_vh = (old_vh - dy).clamp(0.04, 2.5);
                t.scale_x = (new_vw / (t.scale * uv_w).max(0.01)).clamp(0.05, 4.0);
                t.scale_y = (new_vh / (t.scale * uv_h).max(0.01)).clamp(0.05, 4.0);
            }
            let (_, _, _, new_vh) = t.visible_norm_rect();
            t.pos_x = bl_x - t.effective_width_norm() * u0;
            t.pos_y = (bl_y - new_vh) - t.effective_height_norm() * v0;
        }
        PreviewDragMode::ResizeBottomLeft => {
            let tr_x = old_vx + old_vw;
            let tr_y = old_vy;
            if t.lock_aspect && !shift_held {
                let d_scale = (-delta_px.x * layer_w_px + delta_px.y * layer_h_px) / diag_sq;
                t.scale = (t.scale * (1.0 + d_scale)).clamp(0.08, 3.0);
            } else {
                let new_vw = (old_vw - dx).clamp(0.04, 2.5);
                let new_vh = (old_vh + dy).clamp(0.04, 2.5);
                t.scale_x = (new_vw / (t.scale * uv_w).max(0.01)).clamp(0.05, 4.0);
                t.scale_y = (new_vh / (t.scale * uv_h).max(0.01)).clamp(0.05, 4.0);
            }
            let (_, _, new_vw, _) = t.visible_norm_rect();
            t.pos_x = (tr_x - new_vw) - t.effective_width_norm() * u0;
            t.pos_y = tr_y - t.effective_height_norm() * v0;
        }
        PreviewDragMode::ResizeRight => {
            let new_vw = (old_vw + dx).clamp(0.04, 2.5);
            t.scale_x = (new_vw / (t.scale * uv_w).max(0.01)).clamp(0.05, 4.0);
        }
        PreviewDragMode::ResizeLeft => {
            let right_edge = old_vx + old_vw;
            let new_vw = (old_vw - dx).clamp(0.04, 2.5);
            t.scale_x = (new_vw / (t.scale * uv_w).max(0.01)).clamp(0.05, 4.0);
            let (_, _, new_vw, _) = t.visible_norm_rect();
            t.pos_x = (right_edge - new_vw) - t.effective_width_norm() * u0;
        }
        PreviewDragMode::ResizeBottom => {
            let new_vh = (old_vh + dy).clamp(0.04, 2.5);
            t.scale_y = (new_vh / (t.scale * uv_h).max(0.01)).clamp(0.05, 4.0);
        }
        PreviewDragMode::ResizeTop => {
            let bottom_edge = old_vy + old_vh;
            let new_vh = (old_vh - dy).clamp(0.04, 2.5);
            t.scale_y = (new_vh / (t.scale * uv_h).max(0.01)).clamp(0.05, 4.0);
            let (_, _, _, new_vh) = t.visible_norm_rect();
            t.pos_y = (bottom_edge - new_vh) - t.effective_height_norm() * v0;
        }
        PreviewDragMode::CropLeft => {
            let full_w = t.effective_width_norm().max(0.05);
            let max_c = (0.90 - t.crop_right).max(0.0);
            t.crop_left = (t.crop_left + dx / full_w).clamp(0.0, max_c);
        }
        PreviewDragMode::CropRight => {
            let full_w = t.effective_width_norm().max(0.05);
            let max_c = (0.90 - t.crop_left).max(0.0);
            t.crop_right = (t.crop_right - dx / full_w).clamp(0.0, max_c);
        }
        PreviewDragMode::CropTop => {
            let full_h = t.effective_height_norm().max(0.05);
            let max_c = (0.90 - t.crop_bottom).max(0.0);
            t.crop_top = (t.crop_top + dy / full_h).clamp(0.0, max_c);
        }
        PreviewDragMode::CropBottom => {
            let full_h = t.effective_height_norm().max(0.05);
            let max_c = (0.90 - t.crop_top).max(0.0);
            t.crop_bottom = (t.crop_bottom - dy / full_h).clamp(0.0, max_c);
        }
        PreviewDragMode::None => {}
    }
}

pub(crate) fn render_scene_thumbnail(
    painter: &egui::Painter,
    egui_ctx: &egui::Context,
    app: &mut NexoraApp,
    scene: &Scene,
    thumb_rect: Rect,
    time_sec: f64,
) {
    if thumb_rect.width() <= 4.0 || thumb_rect.height() <= 4.0 {
        return;
    }

    let thumb_painter = painter.with_clip_rect(thumb_rect);
    thumb_painter.rect_filled(thumb_rect, CornerRadius::same(4), Color32::from_rgb(10, 14, 20));

    let visible_sources: Vec<_> = scene.sources.iter().filter(|s| s.visible).collect();
    if visible_sources.is_empty() {
        thumb_painter.text(
            thumb_rect.center(),
            Align2::CENTER_CENTER,
            "(Kosong)",
            FontId::proportional(10.0),
            Color32::from_rgb(120, 130, 145),
        );
    } else {
        // Render layers from bottom (index N-1) to top (index 0)
        for src in scene.sources.iter().rev() {
            if !src.visible {
                continue;
            }
            match &src.kind {
                SourceKind::AudioInputCapture { .. }
                | SourceKind::AudioOutputCapture { .. }
                | SourceKind::ApplicationAudioCapture { .. } => continue,
                SourceKind::MediaSource { file_path, .. }
                    if SourceKind::is_audio_only_media(file_path) =>
                {
                    continue;
                }
                _ => {}
            }

            let raw_layer_rect = compute_layer_screen_rect(thumb_rect, &src.transform);
            let clipped_rect = raw_layer_rect.intersect(thumb_rect);
            if clipped_rect.width() <= 1.0 || clipped_rect.height() <= 1.0 {
                continue;
            }

            render_source_item_on_canvas(
                &thumb_painter,
                egui_ctx,
                app,
                src,
                raw_layer_rect,
                time_sec,
                1.0,
            );
        }
    }

    thumb_painter.rect_stroke(
        thumb_rect,
        CornerRadius::same(4),
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 25)),
        egui::StrokeKind::Inside,
    );
}

pub(crate) fn render_source_item_on_canvas(
    painter: &egui::Painter,
    egui_ctx: &egui::Context,
    app: &mut NexoraApp,
    src: &SourceItem,
    raw_layer_rect: Rect,
    time_sec: f64,
    alpha_mul: f32,
) {
    if let SourceKind::ImageSlideShow {
        files,
        slide_time_ms,
        loop_slides,
        transition_effect,
        transition_duration_ms,
    } = &src.kind
    {
        let valid_files: Vec<&String> = files.iter().filter(|f| !f.trim().is_empty()).collect();
        if !valid_files.is_empty() {
            let n = valid_files.len();
            let slide_ms = (*slide_time_ms).clamp(800, 60_000) as f64;
            let trans_ms = (*transition_duration_ms)
                .clamp(120, ((*slide_time_ms) / 2).max(150)) as f64;
            let total_ms = time_sec * 1000.0;
            let raw_idx = (total_ms / slide_ms).floor() as usize;
            let cur_idx = if *loop_slides {
                raw_idx % n
            } else {
                raw_idx.min(n - 1)
            };
            let next_idx = if *loop_slides {
                (cur_idx + 1) % n
            } else {
                (cur_idx + 1).min(n - 1)
            };

            let cur_key = format!("IMG:{}", valid_files[cur_idx]);
            let cur_tex = app.capture_engine.get_or_update_filtered_texture(egui_ctx, &cur_key, &src.filter);
            let next_tex = if next_idx != cur_idx {
                let next_key = format!("IMG:{}", valid_files[next_idx]);
                app.capture_engine.get_or_update_filtered_texture(egui_ctx, &next_key, &src.filter)
            } else {
                None
            };

            if let Some(c_tex) = cur_tex {
                let elapsed_in_slide = total_ms % slide_ms;
                let slide_progress = (elapsed_in_slide / slide_ms).clamp(0.0, 1.0) as f32;
                let in_transition = n > 1
                    && cur_idx != next_idx
                    && *transition_effect != SlideTransitionEffect::Cut
                    && elapsed_in_slide >= (slide_ms - trans_ms);
                let trans_t = if in_transition {
                    let raw_t = ((elapsed_in_slide - (slide_ms - trans_ms)) / trans_ms)
                        .clamp(0.0, 1.0) as f32;
                    raw_t * raw_t * (3.0 - 2.0 * raw_t)
                } else {
                    0.0
                };

                draw_slideshow_layer(
                    painter,
                    raw_layer_rect,
                    &src.transform,
                    &c_tex,
                    next_tex.as_ref(),
                    *transition_effect,
                    slide_progress,
                    trans_t,
                    alpha_mul,
                );
                return;
            }
        }
    }

    let live_tex = match &src.kind {
        SourceKind::DisplayCapture {
            monitor_id,
            capture_cursor,
            ..
        } => {
            let resolved_id = app
                .capture_catalog
                .find_display(monitor_id)
                .map(|d| d.id.as_str())
                .unwrap_or(monitor_id.as_str());
            let key = format!("DISP:{}:{}", resolved_id, capture_cursor);
            app.capture_engine.get_or_update_texture(egui_ctx, &key)
        }
        SourceKind::WindowCapture {
            window_hwnd,
            capture_cursor,
            ..
        }
        | SourceKind::GameCapture {
            window_hwnd,
            capture_cursor,
            ..
        } => {
            let key = format!("WIN:{}:{}", window_hwnd, capture_cursor);
            app.capture_engine.get_or_update_texture(egui_ctx, &key)
        }
        SourceKind::WebcamDevice {
            device_id,
            resolution,
            ..
        } => {
            let key = format!("CAM:{}:{}x{}", device_id, resolution.0, resolution.1);
            app.capture_engine.get_or_update_filtered_texture(egui_ctx, &key, &src.filter)
        }
        SourceKind::MediaSource { file_path, .. } => {
            let key = format!("MEDIA:{}", file_path);
            app.capture_engine.get_or_update_filtered_texture(egui_ctx, &key, &src.filter)
        }
        SourceKind::ImageSource { file_path, .. } => {
            let key = format!("IMG:{}", file_path);
            app.capture_engine.get_or_update_filtered_texture(egui_ctx, &key, &src.filter)
        }
        SourceKind::PowerPointSource {
            window_hwnd,
            file_path,
            slides,
            slide_index,
            ..
        } => {
            if *window_hwnd != 0 {
                let key = format!("WIN:{}:true", window_hwnd);
                app.capture_engine.get_or_update_texture(egui_ctx, &key)
            } else if let Some(current_slide) = slides.get(*slide_index) {
                let key = format!("IMG:{}", current_slide);
                app.capture_engine.get_or_update_filtered_texture(egui_ctx, &key, &src.filter)
            } else if !file_path.is_empty() {
                let exported = crate::platform::get_or_export_ppt_slides(file_path);
                if let Some(first) = exported.first() {
                    let key = format!("IMG:{}", first);
                    app.capture_engine.get_or_update_filtered_texture(egui_ctx, &key, &src.filter)
                } else {
                    None
                }
            } else {
                None
            }
        }
        SourceKind::PdfSource {
            file_path,
            pages,
            page_index,
            ..
        } => {
            let valid_page = if let Some(p_str) = pages.get(*page_index) {
                if std::path::Path::new(p_str).exists() {
                    Some(p_str.clone())
                } else {
                    crate::platform::ensure_pdf_page_rendered(file_path, *page_index)
                }
            } else {
                crate::platform::ensure_pdf_page_rendered(file_path, *page_index)
            };

            if let Some(ref current_page) = valid_page {
                let key = format!("IMG:{}", current_page);
                app.capture_engine.get_or_update_filtered_texture(egui_ctx, &key, &src.filter)
            } else {
                None
            }
        }
        _ => None,
    };

    draw_source_layer_visual(
        painter,
        raw_layer_rect,
        &src.name,
        &src.kind,
        &src.transform,
        live_tex.as_ref(),
        time_sec,
        alpha_mul,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_transformed_image(
    painter: &egui::Painter,
    texture_id: egui::TextureId,
    rect: Rect,
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
    tint: Color32,
    flip_h: bool,
    flip_v: bool,
    rotation_deg: i32,
    corner_radius: f32,
) {
    let (mut left_u, mut right_u) = (u0, u1);
    let (mut top_v, mut bot_v) = (v0, v1);
    if flip_h {
        std::mem::swap(&mut left_u, &mut right_u);
    }
    if flip_v {
        std::mem::swap(&mut top_v, &mut bot_v);
    }

    let rot = ((rotation_deg % 360) + 360) % 360;

    let calc_uv = |pos: Pos2| -> Pos2 {
        let nx = if rect.width() > 0.0 {
            ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let ny = if rect.height() > 0.0 {
            ((pos.y - rect.min.y) / rect.height()).clamp(0.0, 1.0)
        } else {
            0.0
        };
        match rot {
            90 => Pos2::new(
                left_u + ny * (right_u - left_u),
                bot_v + nx * (top_v - bot_v),
            ),
            180 => Pos2::new(
                right_u + nx * (left_u - right_u),
                bot_v + ny * (top_v - bot_v),
            ),
            270 => Pos2::new(
                right_u + ny * (left_u - right_u),
                top_v + nx * (bot_v - top_v),
            ),
            _ => Pos2::new(
                left_u + nx * (right_u - left_u),
                top_v + ny * (bot_v - top_v),
            ),
        }
    };

    let r = corner_radius.min(rect.width() * 0.5).min(rect.height() * 0.5);

    if r <= 0.5 {
        if rot == 0 {
            painter.image(
                texture_id,
                rect,
                Rect::from_min_max(Pos2::new(left_u, top_v), Pos2::new(right_u, bot_v)),
                tint,
            );
            return;
        }

        let mut mesh = egui::Mesh::with_texture(texture_id);
        let idx = mesh.vertices.len() as u32;
        mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.min.x, rect.min.y), uv: calc_uv(Pos2::new(rect.min.x, rect.min.y)), color: tint });
        mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.max.x, rect.min.y), uv: calc_uv(Pos2::new(rect.max.x, rect.min.y)), color: tint });
        mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.max.x, rect.max.y), uv: calc_uv(Pos2::new(rect.max.x, rect.max.y)), color: tint });
        mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(rect.min.x, rect.max.y), uv: calc_uv(Pos2::new(rect.min.x, rect.max.y)), color: tint });
        mesh.add_triangle(idx, idx + 1, idx + 2);
        mesh.add_triangle(idx, idx + 2, idx + 3);
        painter.add(egui::Shape::mesh(mesh));
        return;
    }

    // High quality rounded polygon triangulation (fan from center)
    let mut perimeter: Vec<Pos2> = Vec::with_capacity(36);
    let x0 = rect.min.x;
    let x1 = rect.max.x;
    let y0 = rect.min.y;
    let y1 = rect.max.y;

    const N: usize = 8;
    let corners = [
        (x1 - r, y0 + r, -std::f32::consts::FRAC_PI_2), // Top-Right
        (x1 - r, y1 - r, 0.0),                          // Bottom-Right
        (x0 + r, y1 - r, std::f32::consts::FRAC_PI_2),  // Bottom-Left
        (x0 + r, y0 + r, std::f32::consts::PI),         // Top-Left
    ];

    for &(cx, cy, start_angle) in &corners {
        for s in 0..N {
            let angle = start_angle + (s as f32 / N as f32) * std::f32::consts::FRAC_PI_2;
            perimeter.push(Pos2::new(cx + r * angle.cos(), cy + r * angle.sin()));
        }
    }

    let mut mesh = egui::Mesh::with_texture(texture_id);
    let center_pos = rect.center();
    mesh.vertices.push(egui::epaint::Vertex {
        pos: center_pos,
        uv: calc_uv(center_pos),
        color: tint,
    });

    let num_pts = perimeter.len();
    for &p in &perimeter {
        mesh.vertices.push(egui::epaint::Vertex {
            pos: p,
            uv: calc_uv(p),
            color: tint,
        });
    }

    for i in 1..=num_pts {
        let next_i = if i == num_pts { 1 } else { i + 1 };
        mesh.add_triangle(0, i as u32, next_i as u32);
    }

    painter.add(egui::Shape::mesh(mesh));
}

#[allow(clippy::too_many_arguments)]
fn draw_slideshow_layer(
    painter: &egui::Painter,
    rect: Rect,
    transform: &SourceTransform,
    cur_tex: &egui::TextureHandle,
    next_tex: Option<&egui::TextureHandle>,
    effect: SlideTransitionEffect,
    slide_progress: f32,
    trans_t: f32,
    alpha_mul: f32,
) {
    let clipped = painter.with_clip_rect(rect);
    let (u0, v0, u1, v1) = transform.uv_bounds();
    let base_uv = Rect::from_min_max(Pos2::new(u0, v0), Pos2::new(u1, v1));
    let a_factor = alpha_mul.clamp(0.0, 1.0);
    let full_tint = Color32::from_rgba_unmultiplied(255, 255, 255, (a_factor * 255.0) as u8);

    match effect {
        SlideTransitionEffect::Cut => {
            clipped.image(cur_tex.id(), rect, base_uv, full_tint);
        }
        SlideTransitionEffect::CrossFade => {
            if trans_t > 0.001 {
                if let Some(n_tex) = next_tex {
                    let cur_a = ((1.0 - trans_t) * a_factor * 255.0).round() as u8;
                    let nxt_a = (trans_t * a_factor * 255.0).round() as u8;
                    clipped.image(
                        cur_tex.id(),
                        rect,
                        base_uv,
                        Color32::from_rgba_unmultiplied(255, 255, 255, cur_a),
                    );
                    clipped.image(
                        n_tex.id(),
                        rect,
                        base_uv,
                        Color32::from_rgba_unmultiplied(255, 255, 255, nxt_a),
                    );
                    return;
                }
            }
            clipped.image(cur_tex.id(), rect, base_uv, full_tint);
        }
        SlideTransitionEffect::SlideLeft => {
            if trans_t > 0.001 {
                if let Some(n_tex) = next_tex {
                    let w = rect.width();
                    let cur_r = rect.translate(Vec2::new(-w * trans_t, 0.0));
                    let nxt_r = rect.translate(Vec2::new(w * (1.0 - trans_t), 0.0));
                    clipped.image(cur_tex.id(), cur_r, base_uv, full_tint);
                    clipped.image(n_tex.id(), nxt_r, base_uv, full_tint);
                    return;
                }
            }
            clipped.image(cur_tex.id(), rect, base_uv, full_tint);
        }
        SlideTransitionEffect::SlideRight => {
            if trans_t > 0.001 {
                if let Some(n_tex) = next_tex {
                    let w = rect.width();
                    let cur_r = rect.translate(Vec2::new(w * trans_t, 0.0));
                    let nxt_r = rect.translate(Vec2::new(-w * (1.0 - trans_t), 0.0));
                    clipped.image(cur_tex.id(), cur_r, base_uv, full_tint);
                    clipped.image(n_tex.id(), nxt_r, base_uv, full_tint);
                    return;
                }
            }
            clipped.image(cur_tex.id(), rect, base_uv, full_tint);
        }
        SlideTransitionEffect::SlideUp => {
            if trans_t > 0.001 {
                if let Some(n_tex) = next_tex {
                    let h = rect.height();
                    let cur_r = rect.translate(Vec2::new(0.0, -h * trans_t));
                    let nxt_r = rect.translate(Vec2::new(0.0, h * (1.0 - trans_t)));
                    clipped.image(cur_tex.id(), cur_r, base_uv, full_tint);
                    clipped.image(n_tex.id(), nxt_r, base_uv, full_tint);
                    return;
                }
            }
            clipped.image(cur_tex.id(), rect, base_uv, full_tint);
        }
        SlideTransitionEffect::SlideDown => {
            if trans_t > 0.001 {
                if let Some(n_tex) = next_tex {
                    let h = rect.height();
                    let cur_r = rect.translate(Vec2::new(0.0, h * trans_t));
                    let nxt_r = rect.translate(Vec2::new(0.0, -h * (1.0 - trans_t)));
                    clipped.image(cur_tex.id(), cur_r, base_uv, full_tint);
                    clipped.image(n_tex.id(), nxt_r, base_uv, full_tint);
                    return;
                }
            }
            clipped.image(cur_tex.id(), rect, base_uv, full_tint);
        }
        SlideTransitionEffect::ZoomInKenBurns => {
            let zoom_inset = 0.09 * slide_progress;
            let uw = (u1 - u0) * zoom_inset * 0.5;
            let vh = (v1 - v0) * zoom_inset * 0.5;
            let kb_uv = Rect::from_min_max(
                Pos2::new(u0 + uw, v0 + vh),
                Pos2::new(u1 - uw, v1 - vh),
            );
            if trans_t > 0.001 {
                if let Some(n_tex) = next_tex {
                    let cur_a = ((1.0 - trans_t) * a_factor * 255.0).round() as u8;
                    let nxt_a = (trans_t * a_factor * 255.0).round() as u8;
                    clipped.image(
                        cur_tex.id(),
                        rect,
                        kb_uv,
                        Color32::from_rgba_unmultiplied(255, 255, 255, cur_a),
                    );
                    clipped.image(
                        n_tex.id(),
                        rect,
                        base_uv,
                        Color32::from_rgba_unmultiplied(255, 255, 255, nxt_a),
                    );
                    return;
                }
            }
            clipped.image(cur_tex.id(), rect, kb_uv, full_tint);
        }
        SlideTransitionEffect::ZoomOut => {
            let zoom_inset = 0.10 * (1.0 - slide_progress);
            let uw = (u1 - u0) * zoom_inset * 0.5;
            let vh = (v1 - v0) * zoom_inset * 0.5;
            let zo_uv = Rect::from_min_max(
                Pos2::new(u0 + uw, v0 + vh),
                Pos2::new(u1 - uw, v1 - vh),
            );
            if trans_t > 0.001 {
                if let Some(n_tex) = next_tex {
                    let cur_a = ((1.0 - trans_t) * a_factor * 255.0).round() as u8;
                    let nxt_a = (trans_t * a_factor * 255.0).round() as u8;
                    clipped.image(
                        cur_tex.id(),
                        rect,
                        zo_uv,
                        Color32::from_rgba_unmultiplied(255, 255, 255, cur_a),
                    );
                    clipped.image(
                        n_tex.id(),
                        rect,
                        base_uv,
                        Color32::from_rgba_unmultiplied(255, 255, 255, nxt_a),
                    );
                    return;
                }
            }
            clipped.image(cur_tex.id(), rect, zo_uv, full_tint);
        }
    }
}

fn draw_source_layer_visual(
    painter: &egui::Painter,
    rect: Rect,
    name: &str,
    kind: &SourceKind,
    transform: &SourceTransform,
    live_texture: Option<&egui::TextureHandle>,
    time_sec: f64,
    alpha_mul: f32,
) {
    let a_factor = alpha_mul.clamp(0.0, 1.0);
    if let SourceKind::ColorSource { color_rgba, .. } = kind {
        let c = Color32::from_rgba_unmultiplied(
            color_rgba[0],
            color_rgba[1],
            color_rgba[2],
            ((color_rgba[3] as f32) * a_factor) as u8,
        );
        painter.rect_filled(rect, CornerRadius::ZERO, c);
        return;
    }

    if let SourceKind::TextGdiPlus {
        text,
        font_size,
        color_rgba,
        effect,
        scroll_speed,
        loop_animation,
        loop_gap_px,
        banner_style,
        bg_color_rgba,
        accent_color_rgba,
        badge_text,
        text_shadow,
    } = kind
    {
        draw_tv_text_layer(
            painter,
            rect,
            text,
            *font_size,
            *color_rgba,
            *effect,
            *scroll_speed,
            *loop_animation,
            *loop_gap_px,
            *banner_style,
            *bg_color_rgba,
            *accent_color_rgba,
            badge_text,
            *text_shadow,
            time_sec,
            a_factor,
        );
        return;
    }

    if let SourceKind::TimeSource {
        mode,
        target_duration_sec,
        elapsed_sec,
        is_running,
        effect,
        font_size,
        font_family,
        bold_font,
        border_thickness,
        show_badge,
        text_color_rgba,
        bg_color_rgba,
        accent_color_rgba,
        show_hours,
        blink_colon,
        use_24h_format,
        badge_text,
        badge_font_size,
        badge_position,
        badge_offset_x,
        badge_offset_y,
        finish_text,
    } = kind
    {
        draw_time_source_layer(
            painter,
            rect,
            *mode,
            *target_duration_sec,
            *elapsed_sec,
            *is_running,
            *effect,
            *font_size,
            *font_family,
            *bold_font,
            *border_thickness,
            *show_badge,
            *text_color_rgba,
            *bg_color_rgba,
            *accent_color_rgba,
            *show_hours,
            *blink_colon,
            *use_24h_format,
            badge_text,
            *badge_font_size,
            *badge_position,
            *badge_offset_x,
            *badge_offset_y,
            finish_text,
            time_sec,
            a_factor,
        );
        return;
    }

    let (fill_color, accent_color) = match kind {
        SourceKind::DisplayCapture { .. } => (
            Color32::from_rgb(22, 34, 52),
            Color32::from_rgb(56, 139, 253),
        ),
        SourceKind::WindowCapture { .. } | SourceKind::GameCapture { .. } => (
            Color32::from_rgb(25, 42, 46),
            Color32::from_rgb(34, 184, 207),
        ),
        SourceKind::WebcamDevice { .. } => (
            Color32::from_rgb(42, 28, 54),
            Color32::from_rgb(168, 85, 247),
        ),
        SourceKind::MediaSource { .. } => (
            Color32::from_rgb(28, 36, 48),
            Color32::from_rgb(59, 130, 246),
        ),
        _ => (
            Color32::from_rgb(38, 42, 30),
            Color32::from_rgb(132, 204, 22),
        ),
    };

    let (corner_radius_val, border_thick_val, border_col_val) = match kind {
        SourceKind::WebcamDevice {
            corner_radius,
            border_thickness,
            border_color_rgba,
            ..
        } => {
            let scale_factor = (rect.height() / 240.0).clamp(0.1, 8.0);
            let eff_radius = (*corner_radius * scale_factor)
                .min(rect.width() * 0.5)
                .min(rect.height() * 0.5);
            let eff_border = if *border_thickness > 0.0 {
                (*border_thickness * scale_factor).clamp(0.5, 40.0)
            } else {
                0.0
            };
            let b_col = Color32::from_rgba_unmultiplied(
                border_color_rgba[0],
                border_color_rgba[1],
                border_color_rgba[2],
                ((border_color_rgba[3] as f32) * a_factor) as u8,
            );
            (eff_radius, eff_border, Some(b_col))
        }
        _ => (0.0, 0.0, None),
    };

    if let Some(tex) = live_texture {
        // Render real live captured frame with cropped UV coordinates and ZERO text overlay
        let (u0, v0, u1, v1) = transform.uv_bounds();
        let base_opacity = if let SourceKind::ImageSource { opacity, .. } = kind {
            opacity.clamp(0.0, 1.0)
        } else {
            1.0
        };
        let alpha_u8 = ((base_opacity * a_factor) * 255.0).round() as u8;
        let tint = Color32::from_rgba_unmultiplied(255, 255, 255, alpha_u8);
        paint_transformed_image(
            painter,
            tex.id(),
            rect,
            u0,
            v0,
            u1,
            v1,
            tint,
            transform.flip_h,
            transform.flip_v,
            transform.rotation_deg,
            corner_radius_val,
        );

        if border_thick_val > 0.0 {
            if let Some(b_col) = border_col_val {
                painter.rect_stroke(
                    rect,
                    CornerRadius::same(corner_radius_val as u8),
                    Stroke::new(border_thick_val, b_col),
                    egui::StrokeKind::Inside,
                );
            }
        }
    } else {
        let cr_u8 = if corner_radius_val > 0.5 {
            corner_radius_val as u8
        } else {
            3
        };
        painter.rect_filled(rect, CornerRadius::same(cr_u8), fill_color.gamma_multiply(a_factor));

        if border_thick_val > 0.0 {
            if let Some(b_col) = border_col_val {
                painter.rect_stroke(
                    rect,
                    CornerRadius::same(cr_u8),
                    Stroke::new(border_thick_val, b_col),
                    egui::StrokeKind::Inside,
                );
            }
        }

        // Animated telemetry scan bar only when waiting for frame
        let phase = ((time_sec * 0.35).fract()) as f32;
        let bar_y = rect.min.y + rect.height() * phase;
        painter.line_segment(
            [Pos2::new(rect.min.x, bar_y), Pos2::new(rect.max.x, bar_y)],
            Stroke::new(1.5_f32, accent_color.gamma_multiply(0.35)),
        );

        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            format!("{} {}", kind.tabler_icon(), name),
            FontId::proportional(11.0),
            Color32::from_rgba_unmultiplied(220, 228, 240, 160),
        );
    }
}

fn paint_text_with_optional_shadow(
    painter: &egui::Painter,
    pos: Pos2,
    text: &str,
    font_id: FontId,
    color: Color32,
    shadow: bool,
    alpha_mul: f32,
) -> Vec2 {
    if shadow {
        let shadow_alpha = ((175.0 * alpha_mul).clamp(0.0, 255.0)) as u8;
        let shadow_col = Color32::from_rgba_unmultiplied(0, 0, 0, shadow_alpha);
        let shadow_galley =
            painter.layout_no_wrap(text.to_string(), font_id.clone(), shadow_col);
        painter.galley(Pos2::new(pos.x + 1.5, pos.y + 1.5), shadow_galley, shadow_col);
    }
    let galley = painter.layout_no_wrap(text.to_string(), font_id, color);
    let sz = galley.size();
    painter.galley(pos, galley, color);
    sz
}

#[allow(clippy::too_many_arguments)]
fn draw_tv_text_layer(
    painter: &egui::Painter,
    rect: Rect,
    text: &str,
    font_size: f32,
    color_rgba: [u8; 4],
    effect: TextAnimationEffect,
    scroll_speed: f32,
    loop_animation: bool,
    loop_gap_px: f32,
    banner_style: TvBannerStyle,
    bg_color_rgba: [u8; 4],
    accent_color_rgba: [u8; 4],
    badge_text: &str,
    text_shadow: bool,
    time_sec: f64,
    alpha_mul: f32,
) {
    if rect.width() < 8.0 || rect.height() < 8.0 {
        return;
    }

    let clipped = painter.with_clip_rect(rect);

    // Compute pulse / fade multipliers if active
    let speed_factor = (scroll_speed.clamp(15.0, 360.0) / 95.0) as f64;
    let effect_alpha_mul = match effect {
        TextAnimationEffect::FadeInOut => {
            let s = ((time_sec * speed_factor * 2.2).sin() * 0.5 + 0.5) as f32;
            0.15 + 0.85 * s
        }
        TextAnimationEffect::PulseGlow => {
            let s = ((time_sec * speed_factor * 3.0).sin() * 0.5 + 0.5) as f32;
            0.65 + 0.35 * s
        }
        _ => 1.0,
    };
    let final_alpha = (alpha_mul * effect_alpha_mul).clamp(0.0, 1.0);

    let bg_col = Color32::from_rgba_unmultiplied(
        bg_color_rgba[0],
        bg_color_rgba[1],
        bg_color_rgba[2],
        ((bg_color_rgba[3] as f32) * alpha_mul.clamp(0.0, 1.0)) as u8,
    );
    let accent_col = Color32::from_rgba_unmultiplied(
        accent_color_rgba[0],
        accent_color_rgba[1],
        accent_color_rgba[2],
        ((accent_color_rgba[3] as f32) * alpha_mul.clamp(0.0, 1.0)) as u8,
    );
    let text_col = Color32::from_rgba_unmultiplied(
        color_rgba[0],
        color_rgba[1],
        color_rgba[2],
        ((color_rgba[3] as f32) * final_alpha) as u8,
    );

    // Scale font proportionally to canvas height so preview, fullscreen projector, and thumbnail match
    let h_scale = (rect.height() / 56.0).clamp(0.12, 2.5);
    let pulse_scale = if effect == TextAnimationEffect::PulseGlow {
        1.0 + 0.04 * ((time_sec * speed_factor * 3.0).sin() as f32)
    } else {
        1.0
    };
    let effective_font_px = (font_size * h_scale * pulse_scale).clamp(3.5, 120.0);

    // 1. Draw TV Banner Background & Left Badge (if any), and compute `text_area_rect`
    let mut text_area_rect = rect;
    let badge_clean = badge_text.trim();

    match banner_style {
        TvBannerStyle::PlainText => {}
        TvBannerStyle::SolidBanner => {
            clipped.rect_filled(rect, CornerRadius::same(4), bg_col);
            let top_line = Rect::from_min_max(
                rect.min,
                Pos2::new(rect.max.x, (rect.min.y + 2.5).min(rect.max.y)),
            );
            clipped.rect_filled(top_line, CornerRadius::ZERO, accent_col);
            text_area_rect = rect.shrink2(Vec2::new(10.0, 2.0));
        }
        TvBannerStyle::BreakingNewsTv => {
            clipped.rect_filled(rect, CornerRadius::ZERO, bg_col);
            let top_line = Rect::from_min_max(
                rect.min,
                Pos2::new(rect.max.x, (rect.min.y + 3.0).min(rect.max.y)),
            );
            clipped.rect_filled(top_line, CornerRadius::ZERO, accent_col);

            if !badge_clean.is_empty() {
                let badge_font = FontId::proportional((effective_font_px * 0.78).clamp(10.0, 32.0));
                let badge_label = format!("{} {}", icons::PLAYER_RECORD, badge_clean);
                let badge_galley =
                    clipped.layout_no_wrap(badge_label.clone(), badge_font.clone(), Color32::WHITE);
                let badge_w = (badge_galley.size().x + 22.0).min(rect.width() * 0.45);
                let badge_rect = Rect::from_min_max(
                    rect.min,
                    Pos2::new(rect.min.x + badge_w, rect.max.y),
                );
                clipped.rect_filled(badge_rect, CornerRadius::ZERO, accent_col);
                let b_pos = Pos2::new(
                    badge_rect.center().x - badge_galley.size().x * 0.5,
                    badge_rect.center().y - badge_galley.size().y * 0.5,
                );
                clipped.galley(b_pos, badge_galley, Color32::WHITE);

                text_area_rect = Rect::from_min_max(
                    Pos2::new(badge_rect.max.x + 8.0, rect.min.y + 3.0),
                    Pos2::new(rect.max.x - 6.0, rect.max.y - 2.0),
                );
            } else {
                text_area_rect = rect.shrink2(Vec2::new(10.0, 3.0));
            }
        }
        TvBannerStyle::LowerThirdCard => {
            clipped.rect_filled(rect, CornerRadius::same(6), bg_col);
            let left_bar = Rect::from_min_max(
                rect.min,
                Pos2::new((rect.min.x + 6.0).min(rect.max.x), rect.max.y),
            );
            clipped.rect_filled(
                left_bar,
                CornerRadius {
                    nw: 6,
                    ne: 0,
                    sw: 6,
                    se: 0,
                },
                accent_col,
            );

            if !badge_clean.is_empty() {
                let badge_font = FontId::proportional((effective_font_px * 0.62).clamp(9.0, 22.0));
                let badge_galley =
                    clipped.layout_no_wrap(badge_clean.to_string(), badge_font, Color32::WHITE);
                let pill_w = badge_galley.size().x + 12.0;
                let pill_h = (badge_galley.size().y + 4.0).min(rect.height() * 0.42);
                let pill_rect = Rect::from_min_size(
                    Pos2::new(rect.min.x + 14.0, rect.min.y + 5.0),
                    Vec2::new(pill_w, pill_h),
                );
                clipped.rect_filled(pill_rect, CornerRadius::same(4), accent_col);
                clipped.galley(
                    Pos2::new(
                        pill_rect.center().x - badge_galley.size().x * 0.5,
                        pill_rect.center().y - badge_galley.size().y * 0.5,
                    ),
                    badge_galley,
                    Color32::WHITE,
                );
                text_area_rect = Rect::from_min_max(
                    Pos2::new(rect.min.x + 14.0, pill_rect.max.y + 2.0),
                    Pos2::new(rect.max.x - 10.0, rect.max.y - 4.0),
                );
            } else {
                text_area_rect = Rect::from_min_max(
                    Pos2::new(rect.min.x + 14.0, rect.min.y + 4.0),
                    Pos2::new(rect.max.x - 10.0, rect.max.y - 4.0),
                );
            }
        }
        TvBannerStyle::HeadlineStrap => {
            clipped.rect_filled(rect, CornerRadius::same(4), bg_col);
            let top_stripe = Rect::from_min_max(
                rect.min,
                Pos2::new(rect.max.x, (rect.min.y + 3.0).min(rect.max.y)),
            );
            let bot_stripe = Rect::from_min_max(
                Pos2::new(rect.min.x, (rect.max.y - 3.0).max(rect.min.y)),
                rect.max,
            );
            clipped.rect_filled(top_stripe, CornerRadius::ZERO, accent_col);
            clipped.rect_filled(bot_stripe, CornerRadius::ZERO, accent_col);

            if !badge_clean.is_empty() {
                let badge_font = FontId::proportional((effective_font_px * 0.72).clamp(10.0, 28.0));
                let badge_galley =
                    clipped.layout_no_wrap(badge_clean.to_string(), badge_font, Color32::WHITE);
                let b_w = (badge_galley.size().x + 18.0).min(rect.width() * 0.38);
                let b_rect = Rect::from_min_max(
                    Pos2::new(rect.min.x, rect.min.y + 3.0),
                    Pos2::new(rect.min.x + b_w, rect.max.y - 3.0),
                );
                clipped.rect_filled(b_rect, CornerRadius::ZERO, accent_col);
                clipped.galley(
                    Pos2::new(
                        b_rect.center().x - badge_galley.size().x * 0.5,
                        b_rect.center().y - badge_galley.size().y * 0.5,
                    ),
                    badge_galley,
                    Color32::WHITE,
                );
                text_area_rect = Rect::from_min_max(
                    Pos2::new(b_rect.max.x + 10.0, rect.min.y + 4.0),
                    Pos2::new(rect.max.x - 10.0, rect.max.y - 4.0),
                );
            } else {
                text_area_rect = rect.shrink2(Vec2::new(12.0, 4.0));
            }
        }
        TvBannerStyle::CornerBugLive => {
            clipped.rect_filled(rect, CornerRadius::same(8), bg_col);
            clipped.rect_stroke(
                rect,
                CornerRadius::same(8),
                Stroke::new(1.2_f32, accent_col),
                egui::StrokeKind::Inside,
            );
            if !badge_clean.is_empty() {
                let badge_font = FontId::proportional((effective_font_px * 0.78).clamp(10.0, 26.0));
                let lbl = format!("{} {}", icons::PLAYER_RECORD, badge_clean);
                let badge_galley = clipped.layout_no_wrap(lbl, badge_font, Color32::WHITE);
                let b_w = (badge_galley.size().x + 14.0).min(rect.width() * 0.50);
                let b_rect = Rect::from_min_max(
                    rect.min,
                    Pos2::new(rect.min.x + b_w, rect.max.y),
                );
                clipped.rect_filled(
                    b_rect,
                    CornerRadius {
                        nw: 8,
                        ne: 0,
                        sw: 8,
                        se: 0,
                    },
                    accent_col,
                );
                clipped.galley(
                    Pos2::new(
                        b_rect.center().x - badge_galley.size().x * 0.5,
                        b_rect.center().y - badge_galley.size().y * 0.5,
                    ),
                    badge_galley,
                    Color32::WHITE,
                );
                text_area_rect = Rect::from_min_max(
                    Pos2::new(b_rect.max.x + 6.0, rect.min.y + 2.0),
                    Pos2::new(rect.max.x - 6.0, rect.max.y - 2.0),
                );
            } else {
                text_area_rect = rect.shrink2(Vec2::new(8.0, 2.0));
            }
        }
    }

    if text_area_rect.width() < 4.0 || text_area_rect.height() < 4.0 {
        return;
    }

    let text_painter = painter.with_clip_rect(text_area_rect);
    let font_id = FontId::proportional(effective_font_px);

    // 2. Render text with selected animation effect inside `text_area_rect`
    match effect {
        TextAnimationEffect::ScrollLeft => {
            let single_line = text.replace('\n', "   •   ");
            let measure_galley =
                text_painter.layout_no_wrap(single_line.clone(), font_id.clone(), text_col);
            let text_w = measure_galley.size().x.max(20.0);
            let text_h = measure_galley.size().y;
            let y_pos = text_area_rect.center().y - text_h * 0.5;
            let gap = loop_gap_px.clamp(20.0, 400.0);
            let spd = scroll_speed.clamp(10.0, 500.0) as f64;

            if loop_animation {
                let cycle = (text_w + gap) as f64;
                let offset = ((time_sec * spd) % cycle) as f32;
                let mut x = text_area_rect.min.x - offset;
                while x < text_area_rect.max.x + gap {
                    paint_text_with_optional_shadow(
                        &text_painter,
                        Pos2::new(x, y_pos),
                        &single_line,
                        font_id.clone(),
                        text_col,
                        text_shadow,
                        final_alpha,
                    );
                    x += text_w + gap;
                }
            } else {
                let total_dist = (text_area_rect.width() + text_w) as f64;
                let traveled = (time_sec * spd).min(total_dist) as f32;
                let x = text_area_rect.max.x - traveled;
                paint_text_with_optional_shadow(
                    &text_painter,
                    Pos2::new(x, y_pos),
                    &single_line,
                    font_id,
                    text_col,
                    text_shadow,
                    final_alpha,
                );
            }
        }
        TextAnimationEffect::ScrollRight => {
            let single_line = text.replace('\n', "   •   ");
            let measure_galley =
                text_painter.layout_no_wrap(single_line.clone(), font_id.clone(), text_col);
            let text_w = measure_galley.size().x.max(20.0);
            let text_h = measure_galley.size().y;
            let y_pos = text_area_rect.center().y - text_h * 0.5;
            let gap = loop_gap_px.clamp(20.0, 400.0);
            let spd = scroll_speed.clamp(10.0, 500.0) as f64;

            if loop_animation {
                let cycle = (text_w + gap) as f64;
                let offset = ((time_sec * spd) % cycle) as f32;
                let mut x = text_area_rect.min.x - (text_w + gap) + offset;
                while x < text_area_rect.max.x + gap {
                    paint_text_with_optional_shadow(
                        &text_painter,
                        Pos2::new(x, y_pos),
                        &single_line,
                        font_id.clone(),
                        text_col,
                        text_shadow,
                        final_alpha,
                    );
                    x += text_w + gap;
                }
            } else {
                let total_dist = (text_area_rect.width() + text_w) as f64;
                let traveled = (time_sec * spd).min(total_dist) as f32;
                let x = text_area_rect.min.x - text_w + traveled;
                paint_text_with_optional_shadow(
                    &text_painter,
                    Pos2::new(x, y_pos),
                    &single_line,
                    font_id,
                    text_col,
                    text_shadow,
                    final_alpha,
                );
            }
        }
        TextAnimationEffect::ScrollUp => {
            let measure_galley =
                text_painter.layout_no_wrap(text.to_string(), font_id.clone(), text_col);
            let text_w = measure_galley.size().x;
            let text_h = measure_galley.size().y.max(20.0);
            let x_pos = (text_area_rect.center().x - text_w * 0.5).max(text_area_rect.min.x + 4.0);
            let gap = loop_gap_px.clamp(20.0, 400.0);
            let spd = scroll_speed.clamp(10.0, 500.0) as f64;

            if loop_animation {
                let cycle = (text_h + gap) as f64;
                let offset = ((time_sec * spd) % cycle) as f32;
                let mut y = text_area_rect.min.y - offset;
                while y < text_area_rect.max.y + gap {
                    paint_text_with_optional_shadow(
                        &text_painter,
                        Pos2::new(x_pos, y),
                        text,
                        font_id.clone(),
                        text_col,
                        text_shadow,
                        final_alpha,
                    );
                    y += text_h + gap;
                }
            } else {
                let total_dist = (text_area_rect.height() + text_h) as f64;
                let traveled = (time_sec * spd).min(total_dist) as f32;
                let y = text_area_rect.max.y - traveled;
                paint_text_with_optional_shadow(
                    &text_painter,
                    Pos2::new(x_pos, y),
                    text,
                    font_id,
                    text_col,
                    text_shadow,
                    final_alpha,
                );
            }
        }
        TextAnimationEffect::BounceHorizontal => {
            let measure_galley =
                text_painter.layout_no_wrap(text.to_string(), font_id.clone(), text_col);
            let text_w = measure_galley.size().x;
            let text_h = measure_galley.size().y;
            let y_pos = text_area_rect.center().y - text_h * 0.5;
            let free_w = (text_area_rect.width() - text_w).max(24.0);
            let phase = ((time_sec * speed_factor * 1.4).sin() * 0.5 + 0.5) as f32;
            let x_pos = text_area_rect.min.x + free_w * phase;
            paint_text_with_optional_shadow(
                &text_painter,
                Pos2::new(x_pos, y_pos),
                text,
                font_id,
                text_col,
                text_shadow,
                final_alpha,
            );
        }
        TextAnimationEffect::Typewriter => {
            let chars: Vec<char> = text.chars().collect();
            let total_chars = chars.len().max(1);
            let chars_per_sec = (scroll_speed.clamp(15.0, 360.0) / 8.0) as f64;
            let hold_chars = 10usize;
            let cycle_len = total_chars + hold_chars;
            let step = if loop_animation {
                ((time_sec * chars_per_sec) as usize) % cycle_len
            } else {
                ((time_sec * chars_per_sec) as usize).min(total_chars)
            };
            let visible_count = step.min(total_chars);
            let cursor = if ((time_sec * 3.0) as u64).is_multiple_of(2) {
                "|"
            } else {
                " "
            };
            let sub: String = chars.into_iter().take(visible_count).collect();
            let typed_str = format!("{}{}", sub, cursor);

            let measure_galley =
                text_painter.layout_no_wrap(typed_str.clone(), font_id.clone(), text_col);
            let y_pos = text_area_rect.center().y - measure_galley.size().y * 0.5;
            let x_pos = if banner_style == TvBannerStyle::PlainText {
                (text_area_rect.center().x - measure_galley.size().x * 0.5)
                    .max(text_area_rect.min.x)
            } else {
                text_area_rect.min.x + 4.0
            };
            paint_text_with_optional_shadow(
                &text_painter,
                Pos2::new(x_pos, y_pos),
                &typed_str,
                font_id,
                text_col,
                text_shadow,
                final_alpha,
            );
        }
        TextAnimationEffect::Static
        | TextAnimationEffect::PulseGlow
        | TextAnimationEffect::FadeInOut => {
            let measure_galley =
                text_painter.layout_no_wrap(text.to_string(), font_id.clone(), text_col);
            let y_pos = text_area_rect.center().y - measure_galley.size().y * 0.5;
            let x_pos = match banner_style {
                TvBannerStyle::LowerThirdCard | TvBannerStyle::HeadlineStrap => {
                    text_area_rect.min.x + 2.0
                }
                _ => (text_area_rect.center().x - measure_galley.size().x * 0.5)
                    .max(text_area_rect.min.x + 2.0),
            };
            paint_text_with_optional_shadow(
                &text_painter,
                Pos2::new(x_pos, y_pos),
                text,
                font_id,
                text_col,
                text_shadow,
                final_alpha,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_time_source_layer(
    painter: &egui::Painter,
    rect: Rect,
    mode: TimeSourceMode,
    target_duration_sec: u32,
    elapsed_sec: f32,
    is_running: bool,
    effect: TimeVisualEffect,
    font_size: f32,
    font_family: TimeFontFamily,
    bold_font: bool,
    border_thickness: f32,
    show_badge: bool,
    text_color_rgba: [u8; 4],
    bg_color_rgba: [u8; 4],
    accent_color_rgba: [u8; 4],
    show_hours: bool,
    _blink_colon: bool,
    use_24h_format: bool,
    badge_text: &str,
    badge_font_size: f32,
    badge_position: TimeBadgePosition,
    badge_offset_x: f32,
    badge_offset_y: f32,
    finish_text: &str,
    time_sec: f64,
    alpha_mul: f32,
) {
    if rect.width() < 10.0 || rect.height() < 10.0 {
        return;
    }

    let clipped = painter.with_clip_rect(rect);

    // 1. Compute time string depending on mode (solid colon ':' without blinking)
    let time_display = match mode {
        TimeSourceMode::RealTimeClock => {
            let now = chrono::Local::now();
            use chrono::Timelike;
            if use_24h_format {
                if show_hours {
                    format!("{:02}:{:02}:{:02}", now.hour(), now.minute(), now.second())
                } else {
                    format!("{:02}:{:02}", now.minute(), now.second())
                }
            } else {
                let (pm, h12) = now.hour12();
                let ampm = if pm { "PM" } else { "AM" };
                if show_hours {
                    format!("{:02}:{:02}:{:02} {}", h12, now.minute(), now.second(), ampm)
                } else {
                    format!("{:02}:{:02} {}", now.minute(), now.second(), ampm)
                }
            }
        }
        TimeSourceMode::CountUpStopwatch => {
            let total_secs = elapsed_sec.max(0.0) as u64;
            let h = total_secs / 3600;
            let m = (total_secs % 3600) / 60;
            let s = total_secs % 60;
            if show_hours || h > 0 {
                format!("{:02}:{:02}:{:02}", h, m, s)
            } else {
                format!("{:02}:{:02}", m, s)
            }
        }
        TimeSourceMode::CountdownTimer => {
            let remaining = (target_duration_sec as f32 - elapsed_sec).max(0.0);
            let is_finished = remaining <= 0.001;
            if is_finished && !finish_text.trim().is_empty() {
                finish_text.trim().to_string()
            } else {
                let total_secs = remaining.ceil() as u64;
                let h = total_secs / 3600;
                let m = (total_secs % 3600) / 60;
                let s = total_secs % 60;
                if show_hours || h > 0 {
                    format!("{:02}:{:02}:{:02}", h, m, s)
                } else {
                    format!("{:02}:{:02}", m, s)
                }
            }
        }
    };

    // 2. Compute effective font size & FontId ensuring text fits gracefully on both Canvas and Thumbnail
    let char_count = time_display.chars().count().max(1) as f32;
    let char_aspect = if font_family == TimeFontFamily::Monospace {
        0.60_f32
    } else {
        0.52_f32
    };

    // Scaled border & corner radius so they look refined on both high-res canvas and small thumbnails
    let eff_border = if border_thickness > 0.0 {
        (border_thickness * (rect.height() / 120.0).clamp(0.25, 1.5)).clamp(0.5, border_thickness.max(0.5))
    } else {
        0.0
    };
    let eff_radius = (6.0 * (rect.height() / 120.0).clamp(0.3, 1.2)).clamp(2.0, 8.0);
    let base_scale = (rect.height() / 90.0).clamp(0.08, 6.0);

    let has_badge = show_badge && rect.height() >= 16.0;
    let eff_badge_font_px = (badge_font_size * base_scale).clamp(3.5, (rect.height() * 0.45).max(7.0));
    let badge_font = FontId::proportional(eff_badge_font_px);

    let is_badge_at_top = matches!(
        badge_position,
        TimeBadgePosition::TopLeft | TimeBadgePosition::TopCenter | TimeBadgePosition::TopRight
    );
    let is_badge_at_bottom = matches!(
        badge_position,
        TimeBadgePosition::BottomLeft | TimeBadgePosition::BottomCenter | TimeBadgePosition::BottomRight
    );

    let badge_h = if has_badge {
        match effect {
            TimeVisualEffect::TvCornerBadge => 0.0,
            _ => (eff_badge_font_px * 1.5).max((rect.height() * 0.20).clamp(6.0, 40.0)),
        }
    } else {
        0.0
    };

    let pad_x = (rect.width() * 0.06).clamp(3.0, 26.0) + eff_border;
    let mut avail_w = (rect.width() - pad_x * 2.0).max(4.0);
    if effect == TimeVisualEffect::TvCornerBadge && has_badge {
        avail_w = (avail_w - rect.width() * 0.35).max(4.0);
    }
    let max_font_w = avail_w / (char_count * char_aspect);

    let pad_y = (rect.height() * 0.07).clamp(2.0, 18.0);
    let avail_h = (rect.height() - badge_h - pad_y * 2.0).max(4.0);
    let max_font_h = avail_h * 0.92;

    let desired_font = font_size * base_scale;
    let effective_font_px = desired_font.min(max_font_w).min(max_font_h).clamp(3.0, 260.0);

    let font_id = match font_family {
        TimeFontFamily::Monospace => FontId::monospace(effective_font_px),
        TimeFontFamily::Proportional => FontId::proportional(effective_font_px),
        TimeFontFamily::Serif => FontId::new(effective_font_px, egui::FontFamily::Proportional),
    };

    // 3. Colors
    let text_col = Color32::from_rgba_unmultiplied(
        text_color_rgba[0],
        text_color_rgba[1],
        text_color_rgba[2],
        ((text_color_rgba[3] as f32) * alpha_mul) as u8,
    );
    let bg_col = Color32::from_rgba_unmultiplied(
        bg_color_rgba[0],
        bg_color_rgba[1],
        bg_color_rgba[2],
        ((bg_color_rgba[3] as f32) * alpha_mul) as u8,
    );
    let accent_col = Color32::from_rgba_unmultiplied(
        accent_color_rgba[0],
        accent_color_rgba[1],
        accent_color_rgba[2],
        ((accent_color_rgba[3] as f32) * alpha_mul) as u8,
    );

    let default_badge = match mode {
        TimeSourceMode::RealTimeClock => "CLOCK",
        TimeSourceMode::CountUpStopwatch => "STOPWATCH",
        TimeSourceMode::CountdownTimer => "COUNTDOWN",
    };
    let active_badge = if badge_text.trim().is_empty() {
        default_badge
    } else {
        badge_text.trim()
    };

    let margin_x = (rect.width() * 0.05).clamp(4.0, 30.0);
    let margin_y = (rect.height() * 0.07).clamp(3.0, 24.0);

    let draw_time_text = |target_painter: &egui::Painter, pos: Pos2, align: Align2, text: &str, font: FontId, color: Color32| {
        if bold_font {
            let bold_dx = (font.size * 0.024).clamp(0.4, 2.4);
            target_painter.text(Pos2::new(pos.x + bold_dx, pos.y), align, text, font.clone(), color);
            target_painter.text(Pos2::new(pos.x, pos.y + bold_dx * 0.25), align, text, font.clone(), color);
        }
        target_painter.text(pos, align, text, font, color);
    };

    let time_center_y = if has_badge && is_badge_at_top {
        rect.min.y + badge_h * 0.68 + (rect.height() - badge_h * 0.68) * 0.5
    } else if has_badge && is_badge_at_bottom {
        rect.min.y + (rect.height() - badge_h * 0.68) * 0.5
    } else {
        rect.center().y
    };

    // 4. Render based on TimeVisualEffect
    match effect {
        TimeVisualEffect::DigitalNeon => {
            // High-tech LED Neon Frame
            clipped.rect_filled(rect, CornerRadius::same(eff_radius as u8), bg_col);
            if eff_border > 0.0 {
                clipped.rect_stroke(
                    rect,
                    CornerRadius::same(eff_radius as u8),
                    Stroke::new(eff_border, accent_col),
                    egui::StrokeKind::Inside,
                );
                if rect.width() > 24.0 && rect.height() > 18.0 && eff_border >= 1.2 {
                    let inner_thick = (eff_border * 0.5).clamp(0.5, 2.0);
                    clipped.rect_stroke(
                        rect.shrink(eff_border + 1.0),
                        CornerRadius::same((eff_radius - 1.5).max(0.0) as u8),
                        Stroke::new(inner_thick, accent_col.gamma_multiply(0.30)),
                        egui::StrokeKind::Inside,
                    );
                }
            }

            // Header badge with custom size, anchor position, and user offsets
            if has_badge {
                let badge_col = accent_col.gamma_multiply(0.85);
                let badge_str = format!("{} {}", icons::CLOCK, active_badge);
                let badge_galley = clipped.layout_no_wrap(badge_str.clone(), badge_font.clone(), badge_col);
                let badge_w = badge_galley.size().x;
                let badge_h = badge_galley.size().y;

                let (base_x, base_y) = match badge_position {
                    TimeBadgePosition::TopLeft => (rect.min.x + margin_x, rect.min.y + margin_y),
                    TimeBadgePosition::TopCenter => (rect.center().x - badge_w * 0.5, rect.min.y + margin_y),
                    TimeBadgePosition::TopRight => (rect.max.x - margin_x - badge_w, rect.min.y + margin_y),
                    TimeBadgePosition::BottomLeft => (rect.min.x + margin_x, rect.max.y - margin_y - badge_h),
                    TimeBadgePosition::BottomCenter => (rect.center().x - badge_w * 0.5, rect.max.y - margin_y - badge_h),
                    TimeBadgePosition::BottomRight => (rect.max.x - margin_x - badge_w, rect.max.y - margin_y - badge_h),
                };
                let final_x = base_x + badge_offset_x * base_scale;
                let final_y = base_y + badge_offset_y * base_scale;

                clipped.text(
                    Pos2::new(final_x, final_y),
                    Align2::LEFT_TOP,
                    badge_str,
                    badge_font,
                    badge_col,
                );
            }

            // Glowing neon shadow effect
            let center = Pos2::new(rect.center().x, time_center_y);
            let glow_col = accent_col.gamma_multiply(0.20 * alpha_mul);
            let glow_dist = (effective_font_px * 0.035).clamp(0.4, 2.0);
            let glow_offsets = [(-glow_dist, 0.0), (glow_dist, 0.0), (0.0, -glow_dist), (0.0, glow_dist)];
            for (dx, dy) in glow_offsets {
                clipped.text(
                    Pos2::new(center.x + dx, center.y + dy),
                    Align2::CENTER_CENTER,
                    &time_display,
                    font_id.clone(),
                    glow_col,
                );
            }

            // Crisp front text
            draw_time_text(&clipped, center, Align2::CENTER_CENTER, &time_display, font_id, text_col);
        }
        TimeVisualEffect::TvCornerBadge => {
            // TV Broadcast Lower-Third / Corner Bug Pill
            clipped.rect_filled(rect, CornerRadius::same(eff_radius as u8), bg_col);
            if eff_border > 0.0 {
                clipped.rect_stroke(
                    rect,
                    CornerRadius::same(eff_radius as u8),
                    Stroke::new(eff_border, accent_col),
                    egui::StrokeKind::Inside,
                );
            }

            if has_badge {
                let badge_font = FontId::proportional(eff_badge_font_px.clamp(6.0, 36.0));
                let badge_galley = clipped.layout_no_wrap(active_badge.to_string(), badge_font, Color32::WHITE);
                let badge_pad = (rect.width() * 0.03).clamp(3.0, 16.0);
                let badge_w = (badge_galley.size().x + badge_pad * 2.0).min(rect.width() * 0.45);
                let badge_h = rect.height();

                let (base_x, _) = match badge_position {
                    TimeBadgePosition::TopRight | TimeBadgePosition::BottomRight => {
                        (rect.max.x - badge_w, rect.min.y)
                    }
                    _ => (rect.min.x, rect.min.y),
                };
                let final_x = base_x + badge_offset_x * base_scale;
                let badge_rect = Rect::from_min_size(
                    Pos2::new(final_x, rect.min.y),
                    Vec2::new(badge_w, badge_h),
                );
                clipped.rect_filled(
                    badge_rect,
                    CornerRadius::same(eff_radius as u8),
                    accent_col,
                );
                clipped.galley(
                    Pos2::new(
                        badge_rect.center().x - badge_galley.size().x * 0.5,
                        badge_rect.center().y - badge_galley.size().y * 0.5,
                    ),
                    badge_galley,
                    Color32::WHITE,
                );

                let time_area = if base_x > rect.center().x {
                    Rect::from_min_max(rect.min, Pos2::new(badge_rect.min.x - 4.0, rect.max.y))
                } else {
                    Rect::from_min_max(Pos2::new(badge_rect.max.x + 4.0, rect.min.y), rect.max)
                };
                draw_time_text(&clipped, time_area.center(), Align2::CENTER_CENTER, &time_display, font_id, text_col);
            } else {
                draw_time_text(&clipped, rect.center(), Align2::CENTER_CENTER, &time_display, font_id, text_col);
            }
        }
        TimeVisualEffect::MinimalistClean => {
            // Crisp floating text with optional border and shadow, completely transparent background
            if eff_border > 0.0 {
                clipped.rect_stroke(
                    rect,
                    CornerRadius::same(eff_radius as u8),
                    Stroke::new(eff_border, accent_col),
                    egui::StrokeKind::Inside,
                );
            }
            let text_pos = Pos2::new(rect.center().x, time_center_y);
            draw_time_text(&clipped, text_pos, Align2::CENTER_CENTER, &time_display, font_id.clone(), text_col);
            if has_badge {
                let badge_col = accent_col.gamma_multiply(0.9);
                let badge_galley = clipped.layout_no_wrap(active_badge.to_string(), badge_font.clone(), badge_col);
                let badge_w = badge_galley.size().x;
                let badge_h = badge_galley.size().y;

                let (base_x, base_y) = match badge_position {
                    TimeBadgePosition::TopLeft => (rect.min.x + margin_x, rect.min.y + margin_y),
                    TimeBadgePosition::TopCenter => (rect.center().x - badge_w * 0.5, rect.min.y + margin_y),
                    TimeBadgePosition::TopRight => (rect.max.x - margin_x - badge_w, rect.min.y + margin_y),
                    TimeBadgePosition::BottomLeft => (rect.min.x + margin_x, rect.max.y - margin_y - badge_h),
                    TimeBadgePosition::BottomCenter => (rect.center().x - badge_w * 0.5, rect.max.y - margin_y - badge_h),
                    TimeBadgePosition::BottomRight => (rect.max.x - margin_x - badge_w, rect.max.y - margin_y - badge_h),
                };
                let final_x = base_x + badge_offset_x * base_scale;
                let final_y = base_y + badge_offset_y * base_scale;

                clipped.text(
                    Pos2::new(final_x, final_y),
                    Align2::LEFT_TOP,
                    active_badge,
                    badge_font,
                    badge_col,
                );
            }
        }
        TimeVisualEffect::DarkGlassCard => {
            // Modern frosted dark glassmorphism card
            clipped.rect_filled(rect, CornerRadius::same(eff_radius as u8), bg_col);
            if eff_border > 0.0 {
                let border_col = if eff_border > 1.0 {
                    accent_col
                } else {
                    Color32::from_rgba_unmultiplied(255, 255, 255, (55.0 * alpha_mul) as u8)
                };
                clipped.rect_stroke(
                    rect,
                    CornerRadius::same(eff_radius as u8),
                    Stroke::new(eff_border, border_col),
                    egui::StrokeKind::Inside,
                );
            }

            // Pulsing status dot & badge text if show_badge is enabled
            if has_badge {
                let dot_radius = (eff_badge_font_px * 0.28).clamp(2.0, (rect.height() * 0.15).max(3.0));
                let dot_spacing = (eff_badge_font_px * 0.35).clamp(4.0, 20.0);
                let badge_col = Color32::from_rgba_unmultiplied(200, 210, 225, (200.0 * alpha_mul) as u8);
                let badge_galley = clipped.layout_no_wrap(active_badge.to_string(), badge_font.clone(), badge_col);
                let badge_total_w = dot_radius * 2.0 + dot_spacing + badge_galley.size().x;
                let badge_total_h = badge_galley.size().y.max(dot_radius * 2.0);

                let (base_x, base_y) = match badge_position {
                    TimeBadgePosition::TopLeft => (rect.min.x + margin_x, rect.min.y + margin_y),
                    TimeBadgePosition::TopCenter => (rect.center().x - badge_total_w * 0.5, rect.min.y + margin_y),
                    TimeBadgePosition::TopRight => (rect.max.x - margin_x - badge_total_w, rect.min.y + margin_y),
                    TimeBadgePosition::BottomLeft => (rect.min.x + margin_x, rect.max.y - margin_y - badge_total_h),
                    TimeBadgePosition::BottomCenter => (rect.center().x - badge_total_w * 0.5, rect.max.y - margin_y - badge_total_h),
                    TimeBadgePosition::BottomRight => (rect.max.x - margin_x - badge_total_w, rect.max.y - margin_y - badge_total_h),
                };
                let final_x = base_x + badge_offset_x * base_scale;
                let final_y = base_y + badge_offset_y * base_scale;

                let dot_center = Pos2::new(final_x + dot_radius, final_y + badge_total_h * 0.5);
                let dot_col = if is_running {
                    let pulse = ((time_sec * 4.0).sin() * 0.3 + 0.7) as f32;
                    accent_col.gamma_multiply(pulse)
                } else {
                    Color32::from_rgba_unmultiplied(160, 160, 160, (150.0 * alpha_mul) as u8)
                };
                clipped.circle_filled(dot_center, dot_radius, dot_col);

                clipped.text(
                    Pos2::new(final_x + dot_radius * 2.0 + dot_spacing, final_y),
                    Align2::LEFT_TOP,
                    active_badge,
                    badge_font,
                    badge_col,
                );
            }

            draw_time_text(&clipped, Pos2::new(rect.center().x, time_center_y), Align2::CENTER_CENTER, &time_display, font_id, text_col);
        }
    }
}
