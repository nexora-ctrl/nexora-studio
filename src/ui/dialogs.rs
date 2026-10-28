use crate::app::NexoraApp;
use crate::settings::{BitratePreset, ThemeMode, VideoPreset};
use crate::studio::source::{SourceColorTag, SourceKind};
use crate::ui::icons;
use crate::ui::theme::{apply_studio_theme, ThemePalette};
use egui::{RichText, Stroke, Vec2};
use std::path::PathBuf;

pub fn render_library_drawer_if_open(ctx: &egui::Context, app: &mut NexoraApp) {
    if !app.settings.panels.show_library_drawer {
        return;
    }

    let p = ThemePalette::for_mode(app.settings.theme);

    let drawer_w = app.settings.library_drawer_width.clamp(220.0, 380.0);
    let drawer_resp = egui::SidePanel::right("nexora_library_drawer")
        .resizable(true)
        .default_width(drawer_w)
        .width_range(220.0..=380.0)
        .frame(
            egui::Frame::new()
                .fill(p.bg_panel)
                .inner_margin(egui::Margin::same(8))
                .stroke(Stroke::new(1.0_f32, p.border_subtle)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("RECORDING LIBRARY")
                        .strong()
                        .size(12.5)
                        .color(p.text_primary),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("✕").clicked() {
                        app.settings.panels.show_library_drawer = false;
                    }
                    if ui
                        .small_button(icons::REFRESH)
                        .on_hover_text("Rescan output directory")
                        .clicked()
                    {
                        app.media_library
                            .scan_directory(&app.settings.output_directory);
                    }
                });
            });
            ui.separator();

            ui.horizontal(|ui| {
                if ui
                    .button(format!("{} Open Output Folder", icons::FOLDER_OPEN))
                    .clicked()
                {
                    let _ =
                        crate::platform::open_folder_in_explorer(&app.settings.output_directory);
                }
            });
            ui.add_space(4.0);

            if app.media_library.entries.is_empty() {
                ui.label(
                    RichText::new("No recorded video files (.mp4) found in the output folder yet.")
                        .small()
                        .color(p.text_muted),
                );
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("library_entries_scroll")
                    .show(ui, |ui| {
                        for entry in &app.media_library.entries {
                            egui::Frame::new()
                                .fill(p.bg_elevated)
                                .stroke(Stroke::new(1.0_f32, p.border_subtle))
                                .corner_radius(egui::CornerRadius::same(4))
                                .inner_margin(egui::Margin::same(6))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(&entry.file_name)
                                            .strong()
                                            .size(11.5)
                                            .color(p.text_primary),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "{} • {}",
                                            entry.recorded_at.format("%Y-%m-%d %H:%M"),
                                            entry.formatted_size()
                                        ))
                                        .small()
                                        .color(p.text_muted),
                                    );
                                    ui.horizontal(|ui| {
                                        if ui
                                            .small_button(format!("{} Play", icons::PLAYER_PLAY))
                                            .clicked()
                                        {
                                            let _ = crate::platform::open_file_with_system_player(
                                                &entry.full_path,
                                            );
                                        }
                                        if ui
                                            .small_button(format!("{} Folder", icons::FOLDER_OPEN))
                                            .clicked()
                                        {
                                            if let Some(parent) = entry.full_path.parent() {
                                                let _ = crate::platform::open_folder_in_explorer(
                                                    parent,
                                                );
                                            }
                                        }
                                    });
                                });
                            ui.add_space(4.0);
                        }
                    });
            }
        });

    let measured_w = drawer_resp.response.rect.width();
    if (measured_w - app.settings.library_drawer_width).abs() > 1.0 {
        app.settings.library_drawer_width = measured_w.clamp(220.0, 380.0);
    }
}

pub fn render_dialogs(ctx: &egui::Context, app: &mut NexoraApp) {
    // 1. Settings Modal Dialog
    if app.show_settings_dialog {
        let mut open = app.show_settings_dialog;
        egui::Window::new(format!("{} NEXORA Studio Settings", icons::SETTINGS))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(430.0)
            .show(ctx, |ui| {
                ui.heading("Appearance & Workspace");
                ui.horizontal(|ui| {
                    ui.label("UI Theme:");
                    if ui
                        .selectable_value(
                            &mut app.settings.theme,
                            ThemeMode::Dark,
                            format!("{} Dark Mode", icons::MOON),
                        )
                        .clicked()
                    {
                        apply_studio_theme(ctx, app.settings.theme);
                        let _ = app.settings.save();
                    }
                    if ui
                        .selectable_value(
                            &mut app.settings.theme,
                            ThemeMode::Light,
                            format!("{} Light Mode", icons::SUN),
                        )
                        .clicked()
                    {
                        apply_studio_theme(ctx, app.settings.theme);
                        let _ = app.settings.save();
                    }
                });

                ui.separator();
                ui.heading("Video Output Configuration");

                egui::Grid::new("settings_video_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Resolution & FPS:");
                        egui::ComboBox::from_id_salt("preset_combo")
                            .selected_text(app.settings.video_preset.label())
                            .width(230.0)
                            .show_ui(ui, |ui| {
                                for preset in VideoPreset::all() {
                                    if ui
                                        .selectable_value(
                                            &mut app.settings.video_preset,
                                            *preset,
                                            preset.label(),
                                        )
                                        .changed()
                                    {
                                        let _ = app.settings.save();
                                    }
                                }
                            });
                        ui.end_row();

                        ui.label("Video Bitrate:");
                        egui::ComboBox::from_id_salt("bitrate_combo")
                            .selected_text(app.settings.bitrate_preset.label())
                            .width(230.0)
                            .show_ui(ui, |ui| {
                                for br in BitratePreset::all() {
                                    if ui
                                        .selectable_value(
                                            &mut app.settings.bitrate_preset,
                                            *br,
                                            br.label(),
                                        )
                                        .changed()
                                    {
                                        let _ = app.settings.save();
                                    }
                                }
                            });
                        ui.end_row();

                        ui.label("Container Format:");
                        ui.label(RichText::new("MP4 (H.264 + AAC)").monospace());
                        ui.end_row();

                        ui.label("Output Directory:");
                        let mut path_str =
                            app.settings.output_directory.to_string_lossy().to_string();
                        if ui.text_edit_singleline(&mut path_str).changed() {
                            app.settings.output_directory = PathBuf::from(path_str);
                            let _ = app.settings.save();
                        }
                        ui.end_row();
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "Platform: {}",
                            crate::platform::target_os_summary()
                        ))
                        .small(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Save & Close").clicked() {
                            let _ = app.settings.save();
                            app.show_settings_dialog = false;
                        }
                    });
                });
            });

        if !open {
            app.show_settings_dialog = false;
        }
    }

    // 2. Rename Scene Modal Dialog
    if app.show_rename_scene_dialog {
        let mut open = app.show_rename_scene_dialog;
        egui::Window::new("Rename Scene")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(290.0)
            .show(ctx, |ui| {
                ui.label("Enter a new name for the active Preview scene:");
                ui.text_edit_singleline(&mut app.rename_scene_buffer);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        let trimmed = app.rename_scene_buffer.trim();
                        if !trimmed.is_empty() {
                            app.studio.preview_scene_mut().name = trimmed.to_string();
                            app.studio.sync_if_linked();
                        }
                        app.show_rename_scene_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        app.show_rename_scene_dialog = false;
                    }
                });
            });
        if !open {
            app.show_rename_scene_dialog = false;
        }
    }

    // 3. Rename Source Modal Dialog (F2 / Right-Click -> Rename...)
    if app.show_rename_source_dialog {
        let mut open = app.show_rename_source_dialog;
        egui::Window::new("Rename Source")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(290.0)
            .show(ctx, |ui| {
                ui.label("Enter a new name for the selected Source:");
                ui.text_edit_singleline(&mut app.rename_source_buffer);
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        let trimmed = app.rename_source_buffer.trim();
                        if !trimmed.is_empty() {
                            if let Some(src) = app.studio.preview_scene_mut().selected_source_mut()
                            {
                                src.name = trimmed.to_string();
                            }
                            app.studio.sync_if_linked();
                        }
                        app.show_rename_source_dialog = false;
                    }
                    if ui.button("Cancel").clicked() {
                        app.show_rename_source_dialog = false;
                    }
                });
            });
        if !open {
            app.show_rename_source_dialog = false;
        }
    }

    // 4. OBS-Style Source Properties Modal Window (Properties for Selected Source)
    if app.show_source_properties_dialog {
        let mut open = app.show_source_properties_dialog;
        let mut close_clicked = false;
        let mut any_changed = false;
        let mut refresh_catalog = false;

        let displays = app.capture_catalog.displays.clone();
        let windows = app.capture_catalog.windows.clone();
        let cameras = app.capture_catalog.cameras.clone();

        let title = if let Some(src) = app.studio.preview_scene().selected_source() {
            format!("Properties for '{}'", src.name)
        } else {
            "Source Properties".to_string()
        };

        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(440.0)
            .show(ctx, |ui| {
                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                    egui::Grid::new("source_props_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Source Name:");
                            any_changed |= ui.text_edit_singleline(&mut src.name).changed();
                            ui.end_row();

                            ui.label("Source Type:");
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "{} {}",
                                        src.kind.tabler_icon(),
                                        src.kind.type_name()
                                    ))
                                    .strong(),
                                );
                                if ui
                                    .small_button(format!("{} Refresh Devices", icons::REFRESH))
                                    .on_hover_text("Re-scan connected displays and open windows")
                                    .clicked()
                                {
                                    refresh_catalog = true;
                                }
                            });
                            ui.end_row();

                            ui.label("Color Label:");
                            egui::ComboBox::from_id_salt("prop_color_combo")
                                .selected_text(src.color_tag.label())
                                .width(240.0)
                                .show_ui(ui, |ui| {
                                    for tag in SourceColorTag::all() {
                                        any_changed |= ui
                                            .selectable_value(
                                                &mut src.color_tag,
                                                *tag,
                                                tag.label(),
                                            )
                                            .changed();
                                    }
                                });
                            ui.end_row();

                            match &mut src.kind {
                                SourceKind::DisplayCapture {
                                    monitor_id,
                                    monitor_name,
                                    resolution,
                                    capture_cursor,
                                } => {
                                    ui.label("Display / Monitor:");
                                    egui::ComboBox::from_id_salt("prop_display_combo")
                                        .selected_text(format!(
                                            "{} ({}×{})",
                                            monitor_name, resolution.0, resolution.1
                                        ))
                                        .width(260.0)
                                        .show_ui(ui, |ui| {
                                            for d in &displays {
                                                if ui
                                                    .selectable_label(
                                                        *monitor_id == d.id,
                                                        format!(
                                                            "{} ({}×{})",
                                                            d.name, d.width, d.height
                                                        ),
                                                    )
                                                    .clicked()
                                                {
                                                    *monitor_id = d.id.clone();
                                                    *monitor_name = d.name.clone();
                                                    *resolution = (d.width, d.height);
                                                    any_changed = true;
                                                }
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Options:");
                                    any_changed |= ui
                                        .checkbox(capture_cursor, "Capture Mouse Cursor")
                                        .changed();
                                    ui.end_row();
                                }
                                SourceKind::WindowCapture {
                                    window_hwnd,
                                    window_title,
                                    executable_name,
                                    capture_cursor,
                                } => {
                                    ui.label("Application Window:");
                                    egui::ComboBox::from_id_salt("prop_window_combo")
                                        .selected_text(format!(
                                            "[{}] {}",
                                            executable_name, window_title
                                        ))
                                        .width(260.0)
                                        .show_ui(ui, |ui| {
                                            for w in &windows {
                                                if ui
                                                    .selectable_label(
                                                        *window_hwnd == w.hwnd,
                                                        format!("[{}] {}", w.executable, w.title),
                                                    )
                                                    .clicked()
                                                {
                                                    *window_hwnd = w.hwnd;
                                                    *window_title = w.title.clone();
                                                    *executable_name = w.executable.clone();
                                                    any_changed = true;
                                                }
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Options:");
                                    any_changed |= ui
                                        .checkbox(capture_cursor, "Capture Mouse Cursor")
                                        .changed();
                                    ui.end_row();
                                }
                                SourceKind::WebcamDevice {
                                    device_id,
                                    device_name,
                                    resolution,
                                } => {
                                    ui.label("Camera Device:");
                                    let current_display = if let Some(cam) = cameras.iter().find(|c| c.id == *device_id) {
                                        if cam.is_online {
                                            format!("{} (Online)", cam.name)
                                        } else {
                                            format!("{} (Offline)", cam.name)
                                        }
                                    } else {
                                        device_name.clone()
                                    };
                                    egui::ComboBox::from_id_salt("prop_cam_combo")
                                        .selected_text(current_display)
                                        .width(260.0)
                                        .show_ui(ui, |ui| {
                                            for cam in &cameras {
                                                let status_tag = if cam.is_online { "🟢 Ready" } else { "⚪ Disconnected" };
                                                let cam_label = format!("{} — {}", cam.name, status_tag);
                                                if ui
                                                    .selectable_label(*device_id == cam.id, cam_label)
                                                    .clicked()
                                                {
                                                    *device_id = cam.id.clone();
                                                    *device_name = cam.name.clone();
                                                    any_changed = true;
                                                }
                                            }
                                        });
                                    ui.end_row();

                                    let res_options = [
                                        ((1920, 1080), "1920×1080 (Full HD)"),
                                        ((1280, 720), "1280×720 (HD)"),
                                        ((640, 480), "640×480 (Compact)"),
                                    ];
                                    ui.label("Resolution:");
                                    egui::ComboBox::from_id_salt("prop_cam_res_combo")
                                        .selected_text(format!(
                                            "{}×{}",
                                            resolution.0, resolution.1
                                        ))
                                        .width(250.0)
                                        .show_ui(ui, |ui| {
                                            for (res_pair, res_label) in res_options {
                                                if ui
                                                    .selectable_label(
                                                        *resolution == res_pair,
                                                        res_label,
                                                    )
                                                    .clicked()
                                                {
                                                    *resolution = res_pair;
                                                    any_changed = true;
                                                }
                                            }
                                        });
                                    ui.end_row();
                                }
                                SourceKind::ImageSource { file_path, opacity } => {
                                    ui.label("Image File Path:");
                                    any_changed |= ui.text_edit_singleline(file_path).changed();
                                    ui.end_row();

                                    ui.label("Opacity:");
                                    any_changed |= ui
                                        .add(
                                            egui::Slider::new(opacity, 0.10..=1.0)
                                                .fixed_decimals(2),
                                        )
                                        .changed();
                                    ui.end_row();
                                }
                            }
                        });

                    ui.separator();
                    ui.label(RichText::new("Transform & Layout Preset").strong());
                    ui.horizontal_wrapped(|ui| {
                        if ui.small_button("Fit to Screen").clicked() {
                            src.transform.reset_full_canvas();
                            any_changed = true;
                        }
                        if ui.small_button("PiP Bottom-Right").clicked() {
                            src.transform.preset_pip_bottom_right();
                            any_changed = true;
                        }
                        if ui.small_button("PiP Bottom-Left").clicked() {
                            src.transform.preset_pip_bottom_left();
                            any_changed = true;
                        }
                        if ui.small_button("PiP Top-Right").clicked() {
                            src.transform.preset_pip_top_right();
                            any_changed = true;
                        }
                        if ui.small_button("Center").clicked() {
                            src.transform.preset_center();
                            any_changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Scale:");
                        any_changed |= ui
                            .add(
                                egui::Slider::new(&mut src.transform.scale, 0.15..=1.25)
                                    .fixed_decimals(2),
                            )
                            .changed();
                        any_changed |= ui.checkbox(&mut src.locked, "Lock Position").changed();
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("OK").clicked() {
                                close_clicked = true;
                            }
                        });
                    });
                } else {
                    ui.label("No source is currently selected.");
                    if ui.button("Close").clicked() {
                        close_clicked = true;
                    }
                }
            });

        if refresh_catalog {
            app.capture_catalog.refresh_from_system();
        }
        if any_changed {
            app.studio.sync_if_linked();
            app.save_all_state();
        }
        if !open || close_clicked {
            app.show_source_properties_dialog = false;
        }
    }

    // 5. Save Screen & Layout Preset Modal Dialog
    if app.show_save_layout_dialog {
        let mut open = app.show_save_layout_dialog;
        let mut save_triggered = false;
        let mut cancel_triggered = false;

        egui::Window::new(format!(
            "{} Save Screen & Layout Preset",
            icons::DEVICE_FLOPPY
        ))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(360.0)
        .show(ctx, |ui| {
            ui.label(
                "Save your current Screen, Camera PiP positions, Scenes, Audio Mixer, and Dock Layout as a reusable preset:",
            );
            ui.add_space(4.0);
            let resp = ui.text_edit_singleline(&mut app.save_layout_name_buffer);
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                save_triggered = true;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .button(format!("{} Save Preset", icons::DEVICE_FLOPPY))
                    .clicked()
                {
                    save_triggered = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel_triggered = true;
                }
            });
        });

        if save_triggered {
            let preset_name = app.save_layout_name_buffer.trim().to_string();
            if !preset_name.is_empty() {
                app.save_layout_preset(&preset_name);
            }
            app.show_save_layout_dialog = false;
        } else if !open || cancel_triggered {
            app.show_save_layout_dialog = false;
        }
    }

    // 6. Dedicated Fullscreen / Windowed Program Projector
    if app.show_projector_window {
        let mut p_open = app.show_projector_window;
        let proj_title = if app.projector_target_label.is_empty() {
            "Program Projector".to_string()
        } else {
            format!("Program Projector — {}", app.projector_target_label)
        };

        egui::Window::new(proj_title)
            .open(&mut p_open)
            .default_size(Vec2::new(800.0, 450.0))
            .min_size(Vec2::new(320.0, 180.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("● PROGRAM PROJECTOR OUTPUT").strong().color(egui::Color32::from_rgb(239, 68, 68)));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close Projector").clicked() {
                            app.show_projector_window = false;
                        }
                    });
                });
                ui.separator();

                let avail = ui.available_size();
                let aspect = 16.0 / 9.0;
                let (w, h) = if avail.x / avail.y > aspect {
                    (avail.y * aspect, avail.y)
                } else {
                    (avail.x, avail.x / aspect)
                };
                let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), egui::Sense::hover());
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, egui::CornerRadius::ZERO, egui::Color32::BLACK);

                // Draw program scene layers inside projector
                let program_scene = app.studio.program_scene().clone();
                let egui_ctx = ui.ctx().clone();
                for src in &program_scene.sources {
                    if !src.visible {
                        continue;
                    }
                    let src_w = rect.width() * src.transform.scale;
                    let src_h = rect.height() * src.transform.scale;
                    let src_min = egui::Pos2::new(
                        rect.min.x + rect.width() * src.transform.pos_x,
                        rect.min.y + rect.height() * src.transform.pos_y,
                    );
                    let layer_rect = egui::Rect::from_min_size(src_min, Vec2::new(src_w, src_h)).intersect(rect);
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

                    if let Some(tex) = live_tex {
                        painter.image(
                            tex.id(),
                            layer_rect,
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    } else {
                        painter.rect_filled(layer_rect, egui::CornerRadius::same(2), egui::Color32::from_rgb(22, 34, 52));
                    }
                }
            });

        if !p_open {
            app.show_projector_window = false;
        }
    }
}
