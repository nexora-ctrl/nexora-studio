use crate::app::{NexoraApp, PreviewDragMode};
use crate::recording::RecordingStatus;
use crate::studio::scene::Scene;
use crate::studio::source::{SourceKind, SourceTransform};
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
                        ui.label(RichText::new(tag).small().strong().color(header_accent));
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
                        .strong()
                        .size(12.0)
                        .color(p.text_primary),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{} layer(s)", source_count))
                            .small()
                            .color(p.text_muted),
                    );
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
            if role == ViewportRole::Preview && canvas_rect.width() > 10.0 && canvas_rect.height() > 10.0 {
                let alt_held = ui.input(|i| i.modifiers.alt) || app.preview_crop_mode;
                let shift_held = ui.input(|i| i.modifiers.shift);
                let pointer_pos = response.interact_pointer_pos().or_else(|| ui.input(|i| i.pointer.hover_pos()));

                // Compute current selected source's screen rect for handle hit-testing
                let selected_screen_rect = scene_clone
                    .selected_source()
                    .filter(|s| s.visible)
                    .map(|s| compute_layer_screen_rect(canvas_rect, &s.transform));

                // Update cursor icon when hovering over handles or selected source
                if let Some(ptr) = pointer_pos {
                    if app.preview_drag_mode == PreviewDragMode::None {
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
                                    ui.ctx().set_cursor_icon(CursorIcon::Grab);
                                }
                                PreviewDragMode::None => {}
                            }
                        }
                    }
                }

                // Click or drag start: hit-test handles of selected source first, then top-most visible layer
                if response.clicked() || response.drag_started() {
                    if let Some(ptr) = pointer_pos {
                        let mut matched_handle = PreviewDragMode::None;
                        if let Some(sel_rect) = selected_screen_rect {
                            matched_handle = hit_test_handles(sel_rect, ptr, alt_held);
                        }

                        if matched_handle != PreviewDragMode::None && matched_handle != PreviewDragMode::Move {
                            app.preview_drag_mode = matched_handle;
                        } else {
                            // Hit-test visible sources from top-most to bottom-most
                            let mut hit_id = None;
                            for src in scene_clone.sources.iter().rev() {
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
                                app.preview_drag_mode = PreviewDragMode::Move;
                            } else {
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
                    let dx = delta.x / canvas_rect.width();
                    let dy = delta.y / canvas_rect.height();

                    if dx.abs() > 0.0001 || dy.abs() > 0.0001 {
                        let mode = app.preview_drag_mode;
                        if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                            if !src.locked {
                                apply_transform_drag(&mut src.transform, mode, dx, dy, shift_held);
                            }
                        }
                        app.studio.sync_if_linked();
                    }
                }

                if response.drag_stopped() {
                    app.preview_drag_mode = PreviewDragMode::None;
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
                        for src in &from_scene.sources {
                            if !src.visible {
                                continue;
                            }
                            let raw_layer_rect =
                                compute_layer_screen_rect(canvas_rect, &src.transform);
                            let clipped_rect = raw_layer_rect.intersect(canvas_rect);
                            if clipped_rect.width() <= 2.0 || clipped_rect.height() <= 2.0 {
                                continue;
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
                                    app.capture_engine.get_or_update_texture(&egui_ctx, &key)
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
                                    app.capture_engine.get_or_update_texture(&egui_ctx, &key)
                                }
                                SourceKind::WebcamDevice {
                                    device_id,
                                    resolution,
                                    ..
                                } => {
                                    let key = format!(
                                        "CAM:{}:{}x{}",
                                        device_id, resolution.0, resolution.1
                                    );
                                    app.capture_engine.get_or_update_texture(&egui_ctx, &key)
                                }
                                SourceKind::MediaSource { file_path, .. } => {
                                    let key = format!("MEDIA:{}", file_path);
                                    app.capture_engine.get_or_update_texture(&egui_ctx, &key)
                                }
                                SourceKind::ImageSource { file_path, .. } => {
                                    let key = format!("IMG:{}", file_path);
                                    app.capture_engine.get_or_update_texture(&egui_ctx, &key)
                                }
                                _ => None,
                            };
                            draw_source_layer_visual(
                                &canvas_painter,
                                raw_layer_rect,
                                &src.name,
                                &src.kind,
                                &src.transform,
                                live_tex.as_ref(),
                                time_sec,
                                out_alpha,
                            );
                        }
                    }
                }

                let active_alpha = fade_progress.unwrap_or(1.0);

                for src in &scene_clone.sources {
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
                            app.capture_engine.get_or_update_texture(&egui_ctx, &key)
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
                            app.capture_engine.get_or_update_texture(&egui_ctx, &key)
                        }
                        SourceKind::WebcamDevice {
                            device_id,
                            resolution,
                            ..
                        } => {
                            let key = format!("CAM:{}:{}x{}", device_id, resolution.0, resolution.1);
                            app.capture_engine.get_or_update_texture(&egui_ctx, &key)
                        }
                        SourceKind::MediaSource { file_path, .. } => {
                            let key = format!("MEDIA:{}", file_path);
                            app.capture_engine.get_or_update_texture(&egui_ctx, &key)
                        }
                        SourceKind::ImageSource { file_path, .. } => {
                            let key = format!("IMG:{}", file_path);
                            app.capture_engine.get_or_update_texture(&egui_ctx, &key)
                        }
                        _ => None,
                    };

                    draw_source_layer_visual(
                        &canvas_painter,
                        raw_layer_rect,
                        &src.name,
                        &src.kind,
                        &src.transform,
                        live_tex.as_ref(),
                        time_sec,
                        active_alpha,
                    );

                    // Highlight selected source bounding box + 8 OBS resize/crop handles in Preview
                    if role == ViewportRole::Preview
                        && scene_clone.selected_source_id == Some(src.id)
                    {
                        let crop_active = ui.input(|i| i.modifiers.alt) || app.preview_crop_mode;
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
                                [raw_layer_rect.left_bottom(), raw_layer_rect.right_bottom()],
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
                                canvas_painter.rect_filled(handle, CornerRadius::same(1), base_box_color);
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
                                    Pos2::new(raw_layer_rect.center().x, raw_layer_rect.min.y),
                                    t.crop_top > 0.002 || crop_active,
                                ),
                                (
                                    Pos2::new(raw_layer_rect.center().x, raw_layer_rect.max.y),
                                    t.crop_bottom > 0.002 || crop_active,
                                ),
                                (
                                    Pos2::new(raw_layer_rect.min.x, raw_layer_rect.center().y),
                                    t.crop_left > 0.002 || crop_active,
                                ),
                                (
                                    Pos2::new(raw_layer_rect.max.x, raw_layer_rect.center().y),
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
                                canvas_painter.rect_filled(handle, CornerRadius::same(1), h_color);
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

fn compute_layer_screen_rect(canvas_rect: Rect, transform: &SourceTransform) -> Rect {
    let (vis_x, vis_y, vis_w, vis_h) = transform.visible_norm_rect();
    let min = Pos2::new(
        canvas_rect.min.x + canvas_rect.width() * vis_x,
        canvas_rect.min.y + canvas_rect.height() * vis_y,
    );
    let size = Vec2::new(canvas_rect.width() * vis_w, canvas_rect.height() * vis_h);
    Rect::from_min_size(min, size)
}

fn hit_test_handles(layer_rect: Rect, ptr: Pos2, alt_crop_mode: bool) -> PreviewDragMode {
    let r = 9.0_f32;

    let tl = layer_rect.left_top();
    let tr = layer_rect.right_top();
    let bl = layer_rect.left_bottom();
    let br = layer_rect.right_bottom();

    let top_mid = Pos2::new(layer_rect.center().x, layer_rect.min.y);
    let bot_mid = Pos2::new(layer_rect.center().x, layer_rect.max.y);
    let left_mid = Pos2::new(layer_rect.min.x, layer_rect.center().y);
    let right_mid = Pos2::new(layer_rect.max.x, layer_rect.center().y);

    if alt_crop_mode {
        if ptr.distance(left_mid) <= r
            || ( (ptr.x - layer_rect.min.x).abs() <= r && ptr.y >= layer_rect.min.y && ptr.y <= layer_rect.max.y )
        {
            return PreviewDragMode::CropLeft;
        }
        if ptr.distance(right_mid) <= r
            || ( (ptr.x - layer_rect.max.x).abs() <= r && ptr.y >= layer_rect.min.y && ptr.y <= layer_rect.max.y )
        {
            return PreviewDragMode::CropRight;
        }
        if ptr.distance(top_mid) <= r
            || ( (ptr.y - layer_rect.min.y).abs() <= r && ptr.x >= layer_rect.min.x && ptr.x <= layer_rect.max.x )
        {
            return PreviewDragMode::CropTop;
        }
        if ptr.distance(bot_mid) <= r
            || ( (ptr.y - layer_rect.max.y).abs() <= r && ptr.x >= layer_rect.min.x && ptr.x <= layer_rect.max.x )
        {
            return PreviewDragMode::CropBottom;
        }
    }

    if ptr.distance(tl) <= r {
        return PreviewDragMode::ResizeTopLeft;
    }
    if ptr.distance(tr) <= r {
        return PreviewDragMode::ResizeTopRight;
    }
    if ptr.distance(bl) <= r {
        return PreviewDragMode::ResizeBottomLeft;
    }
    if ptr.distance(br) <= r {
        return PreviewDragMode::ResizeBottomRight;
    }

    if ptr.distance(top_mid) <= r {
        return PreviewDragMode::ResizeTop;
    }
    if ptr.distance(bot_mid) <= r {
        return PreviewDragMode::ResizeBottom;
    }
    if ptr.distance(left_mid) <= r {
        return PreviewDragMode::ResizeLeft;
    }
    if ptr.distance(right_mid) <= r {
        return PreviewDragMode::ResizeRight;
    }

    if layer_rect.contains(ptr) {
        return PreviewDragMode::Move;
    }

    PreviewDragMode::None
}

fn apply_transform_drag(
    t: &mut SourceTransform,
    mode: PreviewDragMode,
    dx: f32,
    dy: f32,
    shift_held: bool,
) {
    let (old_vx, old_vy, old_vw, old_vh) = t.visible_norm_rect();
    let (u0, v0, u1, v1) = t.uv_bounds();
    let uv_w = (u1 - u0).max(0.05);
    let uv_h = (v1 - v0).max(0.05);

    match mode {
        PreviewDragMode::Move => {
            t.pos_x = (t.pos_x + dx).clamp(-0.90, 0.96);
            t.pos_y = (t.pos_y + dy).clamp(-0.90, 0.96);
        }
        PreviewDragMode::ResizeBottomRight => {
            if t.lock_aspect && !shift_held {
                let d = if dx.abs() > dy.abs() {
                    dx / uv_w
                } else {
                    dy / uv_h
                };
                t.scale = (t.scale + d).clamp(0.08, 2.0);
            } else {
                let new_fw = (t.effective_width_norm() + dx / uv_w).clamp(0.06, 2.5);
                let new_fh = (t.effective_height_norm() + dy / uv_h).clamp(0.06, 2.5);
                t.scale_x = (new_fw / t.scale.max(0.05)).clamp(0.10, 3.0);
                t.scale_y = (new_fh / t.scale.max(0.05)).clamp(0.10, 3.0);
            }
        }
        PreviewDragMode::ResizeTopLeft => {
            let br_x = old_vx + old_vw;
            let br_y = old_vy + old_vh;
            if t.lock_aspect && !shift_held {
                let d = if dx.abs() > dy.abs() {
                    -dx / uv_w
                } else {
                    -dy / uv_h
                };
                t.scale = (t.scale + d).clamp(0.08, 2.0);
            } else {
                let new_fw = (t.effective_width_norm() - dx / uv_w).clamp(0.06, 2.5);
                let new_fh = (t.effective_height_norm() - dy / uv_h).clamp(0.06, 2.5);
                t.scale_x = (new_fw / t.scale.max(0.05)).clamp(0.10, 3.0);
                t.scale_y = (new_fh / t.scale.max(0.05)).clamp(0.10, 3.0);
            }
            let (_, _, new_vw, new_vh) = t.visible_norm_rect();
            t.pos_x = (br_x - new_vw) - t.effective_width_norm() * u0;
            t.pos_y = (br_y - new_vh) - t.effective_height_norm() * v0;
        }
        PreviewDragMode::ResizeTopRight => {
            let bl_x = old_vx;
            let bl_y = old_vy + old_vh;
            if t.lock_aspect && !shift_held {
                let d = if dx.abs() > dy.abs() {
                    dx / uv_w
                } else {
                    -dy / uv_h
                };
                t.scale = (t.scale + d).clamp(0.08, 2.0);
            } else {
                let new_fw = (t.effective_width_norm() + dx / uv_w).clamp(0.06, 2.5);
                let new_fh = (t.effective_height_norm() - dy / uv_h).clamp(0.06, 2.5);
                t.scale_x = (new_fw / t.scale.max(0.05)).clamp(0.10, 3.0);
                t.scale_y = (new_fh / t.scale.max(0.05)).clamp(0.10, 3.0);
            }
            let (_, _, _, new_vh) = t.visible_norm_rect();
            t.pos_x = bl_x - t.effective_width_norm() * u0;
            t.pos_y = (bl_y - new_vh) - t.effective_height_norm() * v0;
        }
        PreviewDragMode::ResizeBottomLeft => {
            let tr_x = old_vx + old_vw;
            let tr_y = old_vy;
            if t.lock_aspect && !shift_held {
                let d = if dx.abs() > dy.abs() {
                    -dx / uv_w
                } else {
                    dy / uv_h
                };
                t.scale = (t.scale + d).clamp(0.08, 2.0);
            } else {
                let new_fw = (t.effective_width_norm() - dx / uv_w).clamp(0.06, 2.5);
                let new_fh = (t.effective_height_norm() + dy / uv_h).clamp(0.06, 2.5);
                t.scale_x = (new_fw / t.scale.max(0.05)).clamp(0.10, 3.0);
                t.scale_y = (new_fh / t.scale.max(0.05)).clamp(0.10, 3.0);
            }
            let (_, _, new_vw, _) = t.visible_norm_rect();
            t.pos_x = (tr_x - new_vw) - t.effective_width_norm() * u0;
            t.pos_y = tr_y - t.effective_height_norm() * v0;
        }
        PreviewDragMode::ResizeRight => {
            let new_fw = (t.effective_width_norm() + dx / uv_w).clamp(0.06, 2.5);
            t.scale_x = (new_fw / t.scale.max(0.05)).clamp(0.10, 3.0);
        }
        PreviewDragMode::ResizeLeft => {
            let right_edge = old_vx + old_vw;
            let new_fw = (t.effective_width_norm() - dx / uv_w).clamp(0.06, 2.5);
            t.scale_x = (new_fw / t.scale.max(0.05)).clamp(0.10, 3.0);
            let (_, _, new_vw, _) = t.visible_norm_rect();
            t.pos_x = (right_edge - new_vw) - t.effective_width_norm() * u0;
        }
        PreviewDragMode::ResizeBottom => {
            let new_fh = (t.effective_height_norm() + dy / uv_h).clamp(0.06, 2.5);
            t.scale_y = (new_fh / t.scale.max(0.05)).clamp(0.10, 3.0);
        }
        PreviewDragMode::ResizeTop => {
            let bottom_edge = old_vy + old_vh;
            let new_fh = (t.effective_height_norm() - dy / uv_h).clamp(0.06, 2.5);
            t.scale_y = (new_fh / t.scale.max(0.05)).clamp(0.10, 3.0);
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
    } = kind
    {
        let c = Color32::from_rgba_unmultiplied(
            color_rgba[0],
            color_rgba[1],
            color_rgba[2],
            ((color_rgba[3] as f32) * a_factor) as u8,
        );
        let scaled_font = (*font_size * transform.scale.clamp(0.4, 2.5)).clamp(10.0, 96.0);
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            FontId::proportional(scaled_font),
            c,
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
        painter.image(
            tex.id(),
            rect,
            Rect::from_min_max(Pos2::new(u0, v0), Pos2::new(u1, v1)),
            tint,
        );
    } else {
        painter.rect_filled(rect, CornerRadius::same(3), fill_color.gamma_multiply(a_factor));

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
