use crate::app::NexoraApp;
use crate::recording::RecordingStatus;
use crate::studio::scene::Scene;
use crate::studio::source::SourceKind;
use crate::ui::icons;
use crate::ui::theme::ThemePalette;
use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};

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

                // Quick Transform Controls for Selected Source in Preview
                let has_selected = app.studio.preview_scene().selected_source().is_some();
                ui.add_enabled_ui(has_selected, |ui| {
                    ui.label(RichText::new("Transform:").small().color(p.text_secondary));
                    if ui
                        .button("Fit Fullscreen")
                        .on_hover_text("Reset selected source to fill 16:9 canvas")
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
                    if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                        ui.label(RichText::new("Scale").small().color(p.text_muted));
                        if ui
                            .add(
                                egui::Slider::new(&mut src.transform.scale, 0.15..=1.25)
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
                ui.add_space((height * 0.22).max(8.0));

                ui.label(
                    RichText::new("STUDIO")
                        .small()
                        .strong()
                        .color(p.text_muted),
                );

                ui.add_space(6.0);

                let cut_btn = egui::Button::new(
                    RichText::new(format!("CUT {}", icons::ARROW_RIGHT))
                        .strong()
                        .size(11.5)
                        .color(Color32::WHITE),
                )
                .fill(if pending {
                    p.accent_primary
                } else {
                    p.border_strong
                })
                .min_size(Vec2::new(72.0, 28.0));

                if ui
                    .add(cut_btn)
                    .on_hover_text("Apply current Preview scene & layer transforms to Program Output")
                    .clicked()
                {
                    app.studio.transition_preview_to_program();
                }

                ui.add_space(4.0);

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(format!("Apply {}", icons::ARROW_RIGHT)).size(11.0),
                        )
                        .min_size(Vec2::new(72.0, 28.0)),
                    )
                    .on_hover_text("Synchronize Preview scene to Program")
                    .clicked()
                {
                    app.studio.transition_preview_to_program();
                }

                ui.add_space(8.0);
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

                ui.add_space(6.0);
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

            // Handle interactive drag of selected source in Preview viewport
            if role == ViewportRole::Preview && response.dragged() {
                let delta = response.drag_delta();
                if canvas_rect.width() > 10.0 && canvas_rect.height() > 10.0 {
                    if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                        if !src.locked {
                            src.transform.pos_x =
                                (src.transform.pos_x + delta.x / canvas_rect.width()).clamp(-0.5, 0.92);
                            src.transform.pos_y =
                                (src.transform.pos_y + delta.y / canvas_rect.height()).clamp(-0.5, 0.92);
                        }
                    }
                    app.studio.sync_if_linked();
                }
            }

            // Render visible source layers
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
                for src in &scene_clone.sources {
                    if !src.visible {
                        continue;
                    }
                    let src_w = canvas_rect.width() * src.transform.scale;
                    let src_h = canvas_rect.height() * src.transform.scale;
                    let src_min = Pos2::new(
                        canvas_rect.min.x + canvas_rect.width() * src.transform.pos_x,
                        canvas_rect.min.y + canvas_rect.height() * src.transform.pos_y,
                    );
                    let layer_rect =
                        Rect::from_min_size(src_min, Vec2::new(src_w, src_h)).intersect(canvas_rect);

                    if layer_rect.width() <= 2.0 || layer_rect.height() <= 2.0 {
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
                        _ => None,
                    };

                    draw_source_layer_visual(
                        &painter,
                        layer_rect,
                        &src.name,
                        &src.kind,
                        live_tex.as_ref(),
                        time_sec,
                    );

                    // Highlight selected source bounding box in Preview
                    if role == ViewportRole::Preview
                        && scene_clone.selected_source_id == Some(src.id)
                    {
                        painter.rect_stroke(
                            layer_rect,
                            CornerRadius::same(2),
                            Stroke::new(1.5_f32, p.accent_preview),
                            egui::StrokeKind::Inside,
                        );
                        // Corner gizmo handles
                        for corner in [
                            layer_rect.left_top(),
                            layer_rect.right_top(),
                            layer_rect.left_bottom(),
                            layer_rect.right_bottom(),
                        ] {
                            let handle = Rect::from_center_size(corner, Vec2::splat(6.0));
                            painter.rect_filled(handle, CornerRadius::ZERO, p.accent_preview);
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
                        let disp_entry = format!(r"\\.\DISPLAY{}: {}x{} @ {},{}", d.id.replace("MONITOR_", ""), d.width, d.height, d.left, d.top);
                        if ui.button(disp_entry).clicked() {
                            app.projector_target_label = format!(r"\\.\DISPLAY{}: {}x{} @ {},{}", d.id.replace("MONITOR_", ""), d.width, d.height, d.left, d.top);
                            app.show_projector_window = true;
                            ui.close_menu();
                        }
                    }
                    if ui.button("New window").clicked() {
                        app.projector_target_label = "New Window (Resizable)".to_string();
                        app.show_projector_window = true;
                        ui.close_menu();
                    }
                });

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

fn draw_source_layer_visual(
    painter: &egui::Painter,
    rect: Rect,
    name: &str,
    kind: &SourceKind,
    live_texture: Option<&egui::TextureHandle>,
    time_sec: f64,
) {
    let (fill_color, accent_color, subtitle) = match kind {
        SourceKind::DisplayCapture {
            monitor_name,
            resolution,
            capture_cursor,
            ..
        } => (
            Color32::from_rgb(22, 34, 52),
            Color32::from_rgb(56, 139, 253),
            format!(
                "{} • {}×{} {}",
                monitor_name,
                resolution.0,
                resolution.1,
                if *capture_cursor { "• Cursor ON" } else { "" }
            ),
        ),
        SourceKind::WindowCapture {
            window_title,
            executable_name,
            ..
        } => (
            Color32::from_rgb(25, 42, 46),
            Color32::from_rgb(34, 184, 207),
            format!("[{}] {}", executable_name, window_title),
        ),
        SourceKind::WebcamDevice {
            device_name,
            resolution,
            ..
        } => (
            Color32::from_rgb(42, 28, 54),
            Color32::from_rgb(168, 85, 247),
            format!("{} ({}×{})", device_name, resolution.0, resolution.1),
        ),
        SourceKind::ImageSource { file_path, opacity } => (
            Color32::from_rgb(38, 42, 30),
            Color32::from_rgb(132, 204, 22),
            format!("{} ({:.0}%)", file_path, opacity * 100.0),
        ),
    };

    if let Some(tex) = live_texture {
        // Render real live captured frame (Display / Window capture)
        painter.image(
            tex.id(),
            rect,
            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        painter.rect_filled(rect, CornerRadius::same(3), fill_color);

        // Animated telemetry scan bar when waiting for frame or non-screen source
        let phase = ((time_sec * 0.35).fract()) as f32;
        let bar_y = rect.min.y + rect.height() * phase;
        painter.line_segment(
            [Pos2::new(rect.min.x, bar_y), Pos2::new(rect.max.x, bar_y)],
            Stroke::new(1.5_f32, accent_color.gamma_multiply(0.35)),
        );
    }

    // Top layer badge + automatic source info at the top of the source frame only
    let header_h = 20.0_f32.min(rect.height());
    let header_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), header_h));
    painter.rect_filled(
        header_rect,
        CornerRadius::same(2),
        Color32::from_rgba_unmultiplied(10, 12, 18, 175),
    );

    let top_label = if rect.width() > 220.0 {
        format!("{} {} — {}", kind.tabler_icon(), name, subtitle)
    } else {
        format!("{} {}", kind.tabler_icon(), name)
    };

    painter.text(
        Pos2::new(header_rect.min.x + 8.0, header_rect.center().y),
        Align2::LEFT_CENTER,
        top_label,
        FontId::proportional(10.5),
        Color32::from_rgb(235, 240, 248),
    );
}
