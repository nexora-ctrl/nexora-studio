use crate::app::NexoraApp;
use crate::settings::{BitratePreset, MediaDecodeScale, ThemeMode, VideoPreset};
use crate::studio::source::{MediaColorRange, SourceColorTag, SourceKind};
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
        let gpus = app.capture_catalog.gpus.clone();
        egui::Window::new(format!("{} NEXORA Studio Settings", icons::SETTINGS))
            .id(egui::Id::new("nexora_settings_modal_win"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(485.0)
            .max_width(485.0)
            .show(ctx, |ui| {
                const SETTINGS_FIELD_W: f32 = 290.0;
                ui.set_width(465.0);
                ui.set_max_width(465.0);

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
                ui.heading("Hardware GPU / VGA & Video Output");

                egui::Grid::new("settings_video_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Hardware GPU / VGA:");
                        let current_gpu_idx = app.settings.preferred_gpu_index.unwrap_or(0);
                        let selected_gpu_text = gpus
                            .iter()
                            .find(|g| g.index == current_gpu_idx)
                            .map(|g| g.display_label())
                            .unwrap_or_else(|| {
                                if !app.settings.preferred_gpu_name.is_empty() {
                                    app.settings.preferred_gpu_name.clone()
                                } else {
                                    "GPU 0: Default System VGA".to_string()
                                }
                            });

                        egui::ComboBox::from_id_salt("settings_gpu_adapter_combo")
                            .selected_text(selected_gpu_text.clone())
                            .width(SETTINGS_FIELD_W)
                            .truncate()
                            .show_ui(ui, |ui| {
                                for gpu in &gpus {
                                    let is_sel =
                                        app.settings.preferred_gpu_index == Some(gpu.index);
                                    if ui
                                        .selectable_label(is_sel, gpu.display_label())
                                        .clicked()
                                    {
                                        app.settings.preferred_gpu_index = Some(gpu.index);
                                        app.settings.preferred_gpu_name = gpu.name.clone();
                                        let _ = app.settings.save();
                                    }
                                }
                            })
                            .response
                            .on_hover_text(selected_gpu_text);
                        ui.end_row();

                        ui.label("Media Video Decode:");
                        egui::ComboBox::from_id_salt("settings_media_decode_scale_combo")
                            .selected_text(app.settings.media_decode_scale.label())
                            .width(SETTINGS_FIELD_W)
                            .truncate()
                            .show_ui(ui, |ui| {
                                for scale in MediaDecodeScale::all() {
                                    if ui
                                        .selectable_value(
                                            &mut app.settings.media_decode_scale,
                                            *scale,
                                            scale.label(),
                                        )
                                        .changed()
                                    {
                                        let _ = app.settings.save();
                                    }
                                }
                            });
                        ui.end_row();

                        ui.label("Resolution & FPS:");
                        egui::ComboBox::from_id_salt("preset_combo")
                            .selected_text(app.settings.video_preset.label())
                            .width(SETTINGS_FIELD_W)
                            .truncate()
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
                            .width(SETTINGS_FIELD_W)
                            .truncate()
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
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut path_str)
                                    .desired_width(SETTINGS_FIELD_W - 8.0),
                            )
                            .changed()
                        {
                            app.settings.output_directory = PathBuf::from(path_str);
                            let _ = app.settings.save();
                        }
                        ui.end_row();
                    });

                ui.separator();
                ui.allocate_ui_with_layout(
                    Vec2::new(465.0, 26.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
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
                    },
                );
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
        let gpus = app.capture_catalog.gpus.clone();
        let mut selected_gpu_idx = app.settings.preferred_gpu_index.unwrap_or(0);
        let mut selected_decode_scale = app.settings.media_decode_scale;
        let mut gpu_or_scale_changed = false;

        let title = if let Some(src) = app.studio.preview_scene().selected_source() {
            format!("Properties for '{}'", src.name)
        } else {
            "Source Properties".to_string()
        };

        egui::Window::new(title)
            .id(egui::Id::new("nexora_source_properties_modal_win"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(495.0)
            .max_width(495.0)
            .show(ctx, |ui| {
                const PROP_FIELD_W: f32 = 280.0;
                ui.set_width(475.0);
                ui.set_max_width(475.0);

                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                    egui::Grid::new("source_props_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Source Name:");
                            any_changed |= ui
                                .add(
                                    egui::TextEdit::singleline(&mut src.name)
                                        .desired_width(PROP_FIELD_W - 8.0),
                                )
                                .changed();
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
                                .width(PROP_FIELD_W)
                                .truncate()
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
                                        .width(PROP_FIELD_W)
                                        .truncate()
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
                                        .width(PROP_FIELD_W)
                                        .truncate()
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
                                        .width(PROP_FIELD_W)
                                        .truncate()
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
                                        .width(PROP_FIELD_W)
                                        .truncate()
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
                                SourceKind::MediaSource {
                                    is_local_file,
                                    file_path,
                                    loop_playback,
                                    restart_on_activate,
                                    use_hardware_decoding,
                                    hide_when_ended,
                                    close_when_inactive,
                                    speed_percent,
                                    volume,
                                    muted,
                                    is_playing,
                                    color_range,
                                    restart_nonce,
                                } => {
                                    ui.label("Source Mode:");
                                    any_changed |=
                                        ui.checkbox(is_local_file, "Local File").changed();
                                    ui.end_row();

                                    ui.label(if *is_local_file {
                                        "Media File (Video / MP3):"
                                    } else {
                                        "Input URL / Path:"
                                    });
                                    ui.horizontal(|ui| {
                                        any_changed |= ui
                                            .add(
                                                egui::TextEdit::singleline(file_path)
                                                    .desired_width(198.0)
                                                    .hint_text("Select .mp4, .mkv, .mp3, .wav..."),
                                            )
                                            .changed();
                                        if ui
                                            .button(format!("{} Browse...", icons::FOLDER_OPEN))
                                            .on_hover_text(
                                                "Browse local Video (.mp4, .mkv, .mov, .avi) or Audio (.mp3, .wav, .flac, .m4a) file",
                                            )
                                            .clicked()
                                        {
                                            if let Some(picked) =
                                                crate::platform::pick_media_file_dialog()
                                            {
                                                *file_path =
                                                    picked.to_string_lossy().to_string();
                                                *is_playing = true;
                                                *restart_nonce = restart_nonce.wrapping_add(1);
                                                any_changed = true;
                                            }
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Hardware GPU / VGA:");
                                    let gpu_label = gpus
                                        .iter()
                                        .find(|g| g.index == selected_gpu_idx)
                                        .map(|g| g.display_label())
                                        .unwrap_or_else(|| "GPU 0: Default System VGA".to_string());
                                    egui::ComboBox::from_id_salt("prop_media_gpu_combo")
                                        .selected_text(gpu_label.clone())
                                        .width(PROP_FIELD_W)
                                        .truncate()
                                        .show_ui(ui, |ui| {
                                            for gpu in &gpus {
                                                if ui
                                                    .selectable_label(
                                                        selected_gpu_idx == gpu.index,
                                                        gpu.display_label(),
                                                    )
                                                    .clicked()
                                                {
                                                    selected_gpu_idx = gpu.index;
                                                    gpu_or_scale_changed = true;
                                                }
                                            }
                                        })
                                        .response
                                        .on_hover_text(gpu_label);
                                    ui.end_row();

                                    ui.label("Video Decode Performance:");
                                    egui::ComboBox::from_id_salt("prop_media_decode_scale_combo")
                                        .selected_text(selected_decode_scale.label())
                                        .width(PROP_FIELD_W)
                                        .truncate()
                                        .show_ui(ui, |ui| {
                                            for scale in MediaDecodeScale::all() {
                                                if ui
                                                    .selectable_value(
                                                        &mut selected_decode_scale,
                                                        *scale,
                                                        scale.label(),
                                                    )
                                                    .changed()
                                                {
                                                    gpu_or_scale_changed = true;
                                                }
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Playback Options:");
                                    ui.vertical(|ui| {
                                        any_changed |=
                                            ui.checkbox(loop_playback, "Loop").changed();
                                        any_changed |= ui
                                            .checkbox(
                                                restart_on_activate,
                                                "Restart playback when source becomes active",
                                            )
                                            .changed();
                                        any_changed |= ui
                                            .checkbox(
                                                use_hardware_decoding,
                                                "Use hardware decoding when available (DXGI / D3D11)",
                                            )
                                            .changed();
                                        any_changed |= ui
                                            .checkbox(
                                                hide_when_ended,
                                                "Show nothing when playback ends",
                                            )
                                            .changed();
                                        any_changed |= ui
                                            .checkbox(
                                                close_when_inactive,
                                                "Close file when inactive",
                                            )
                                            .changed();
                                    });
                                    ui.end_row();

                                    ui.label("Playback Speed:");
                                    ui.horizontal(|ui| {
                                        any_changed |= ui
                                            .add(
                                                egui::Slider::new(speed_percent, 10..=200)
                                                    .suffix("%"),
                                            )
                                            .changed();
                                        if *speed_percent != 100
                                            && ui.small_button("100%").clicked()
                                        {
                                            *speed_percent = 100;
                                            any_changed = true;
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Audio Gain / Volume:");
                                    ui.horizontal(|ui| {
                                        any_changed |= ui
                                            .add_enabled(
                                                !*muted,
                                                egui::Slider::new(volume, 0.0..=1.0)
                                                    .fixed_decimals(2),
                                            )
                                            .changed();
                                        any_changed |= ui.checkbox(muted, "Mute").changed();
                                    });
                                    ui.end_row();

                                    ui.label("YUV Color Range:");
                                    egui::ComboBox::from_id_salt("media_color_range_combo")
                                        .selected_text(color_range.label())
                                        .width(PROP_FIELD_W)
                                        .truncate()
                                        .show_ui(ui, |ui| {
                                            for cr in MediaColorRange::all() {
                                                any_changed |= ui
                                                    .selectable_value(
                                                        color_range,
                                                        *cr,
                                                        cr.label(),
                                                    )
                                                    .changed();
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Transport Controls:");
                                    ui.horizontal(|ui| {
                                        let play_label = if *is_playing {
                                            format!("{} Pause", icons::PLAYER_PAUSE)
                                        } else {
                                            format!("{} Play", icons::PLAYER_PLAY)
                                        };
                                        if ui.button(play_label).clicked() {
                                            *is_playing = !*is_playing;
                                            any_changed = true;
                                        }
                                        if ui
                                            .button(format!("{} Restart", icons::REFRESH))
                                            .on_hover_text("Seek media back to 00:00 and play")
                                            .clicked()
                                        {
                                            *is_playing = true;
                                            *restart_nonce = restart_nonce.wrapping_add(1);
                                            any_changed = true;
                                        }
                                    });
                                    ui.end_row();
                                }
                                SourceKind::ImageSource { file_path, opacity } => {
                                    ui.label("Image File:");
                                    ui.horizontal(|ui| {
                                        any_changed |= ui
                                            .add(
                                                egui::TextEdit::singleline(file_path)
                                                    .desired_width(198.0)
                                                    .hint_text("Select .png, .jpg, .webp..."),
                                            )
                                            .changed();
                                        if ui
                                            .button(format!("{} Browse...", icons::FOLDER_OPEN))
                                            .on_hover_text("Browse local image file (.png, .jpg, .bmp, .webp)")
                                            .clicked()
                                        {
                                            if let Some(picked) =
                                                crate::platform::pick_image_file_dialog()
                                            {
                                                *file_path =
                                                    picked.to_string_lossy().to_string();
                                                any_changed = true;
                                            }
                                        }
                                    });
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
                                SourceKind::GameCapture {
                                    mode,
                                    window_hwnd,
                                    window_title,
                                    executable_name,
                                    capture_cursor,
                                    anti_cheat_hook,
                                } => {
                                    ui.label("Capture Mode:");
                                    egui::ComboBox::from_id_salt("game_mode_combo")
                                        .selected_text(mode.as_str())
                                        .width(PROP_FIELD_W)
                                        .truncate()
                                        .show_ui(ui, |ui| {
                                            for m in [
                                                "Capture any fullscreen application",
                                                "Capture specific window",
                                                "Capture foreground window with hotkey",
                                            ] {
                                                any_changed |= ui
                                                    .selectable_value(mode, m.to_string(), m)
                                                    .changed();
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Game Window:");
                                    egui::ComboBox::from_id_salt("prop_game_win_combo")
                                        .selected_text(format!(
                                            "[{}] {}",
                                            executable_name, window_title
                                        ))
                                        .width(PROP_FIELD_W)
                                        .truncate()
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
                                    ui.vertical(|ui| {
                                        any_changed |= ui
                                            .checkbox(capture_cursor, "Capture Cursor")
                                            .changed();
                                        any_changed |= ui
                                            .checkbox(
                                                anti_cheat_hook,
                                                "Use anti-cheat compatibility hook",
                                            )
                                            .changed();
                                    });
                                    ui.end_row();
                                }
                                SourceKind::ApplicationAudioCapture {
                                    window_title,
                                    executable_name,
                                    volume,
                                    muted,
                                } => {
                                    ui.label("Window:");
                                    egui::ComboBox::from_id_salt("prop_app_audio_combo")
                                        .selected_text(format!(
                                            "[{}] {}",
                                            executable_name, window_title
                                        ))
                                        .width(PROP_FIELD_W)
                                        .truncate()
                                        .show_ui(ui, |ui| {
                                            for w in &windows {
                                                if ui
                                                    .selectable_label(
                                                        *executable_name == w.executable,
                                                        format!("[{}] {}", w.executable, w.title),
                                                    )
                                                    .clicked()
                                                {
                                                    *window_title = w.title.clone();
                                                    *executable_name = w.executable.clone();
                                                    any_changed = true;
                                                }
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Volume:");
                                    ui.horizontal(|ui| {
                                        any_changed |=
                                            ui.add(egui::Slider::new(volume, 0.0..=1.0)).changed();
                                        any_changed |= ui.checkbox(muted, "Mute").changed();
                                    });
                                    ui.end_row();
                                }
                                SourceKind::AudioInputCapture {
                                    device_name,
                                    volume,
                                    muted,
                                }
                                | SourceKind::AudioOutputCapture {
                                    device_name,
                                    volume,
                                    muted,
                                } => {
                                    ui.label("Audio Device:");
                                    any_changed |= ui.text_edit_singleline(device_name).changed();
                                    ui.end_row();

                                    ui.label("Volume:");
                                    ui.horizontal(|ui| {
                                        any_changed |=
                                            ui.add(egui::Slider::new(volume, 0.0..=1.0)).changed();
                                        any_changed |= ui.checkbox(muted, "Mute").changed();
                                    });
                                    ui.end_row();
                                }
                                SourceKind::ColorSource {
                                    color_rgba,
                                    width,
                                    height,
                                } => {
                                    ui.label("Color:");
                                    any_changed |= ui
                                        .color_edit_button_srgba_unmultiplied(color_rgba)
                                        .changed();
                                    ui.end_row();

                                    ui.label("Dimensions:");
                                    ui.horizontal(|ui| {
                                        any_changed |= ui
                                            .add(egui::DragValue::new(width).range(64..=3840))
                                            .changed();
                                        ui.label("×");
                                        any_changed |= ui
                                            .add(egui::DragValue::new(height).range(64..=2160))
                                            .changed();
                                    });
                                    ui.end_row();
                                }
                                SourceKind::TextGdiPlus {
                                    text,
                                    font_size,
                                    color_rgba,
                                } => {
                                    ui.label("Text:");
                                    any_changed |= ui.text_edit_multiline(text).changed();
                                    ui.end_row();

                                    ui.label("Font Size & Color:");
                                    ui.horizontal(|ui| {
                                        any_changed |= ui
                                            .add(egui::Slider::new(font_size, 12.0..=96.0))
                                            .changed();
                                        any_changed |= ui
                                            .color_edit_button_srgba_unmultiplied(color_rgba)
                                            .changed();
                                    });
                                    ui.end_row();
                                }
                                SourceKind::BrowserSource {
                                    url,
                                    width,
                                    height,
                                    fps,
                                } => {
                                    ui.label("URL:");
                                    any_changed |= ui.text_edit_singleline(url).changed();
                                    ui.end_row();

                                    ui.label("Dimensions / FPS:");
                                    ui.horizontal(|ui| {
                                        any_changed |= ui
                                            .add(egui::DragValue::new(width).range(120..=3840))
                                            .changed();
                                        ui.label("×");
                                        any_changed |= ui
                                            .add(egui::DragValue::new(height).range(120..=2160))
                                            .changed();
                                        ui.label("@");
                                        any_changed |=
                                            ui.add(egui::DragValue::new(fps).range(15..=60)).changed();
                                        ui.label("FPS");
                                    });
                                    ui.end_row();
                                }
                                SourceKind::ImageSlideShow {
                                    files,
                                    slide_time_ms,
                                    loop_slides,
                                } => {
                                    ui.label("Slide Images:");
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{} file(s)", files.len()));
                                        if ui
                                            .button(format!("{} Add Image...", icons::PLUS))
                                            .clicked()
                                        {
                                            if let Some(picked) =
                                                crate::platform::pick_image_file_dialog()
                                            {
                                                files.push(picked.to_string_lossy().to_string());
                                                any_changed = true;
                                            }
                                        }
                                        if !files.is_empty() && ui.small_button("Clear").clicked() {
                                            files.clear();
                                            any_changed = true;
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Slide Timing (ms):");
                                    ui.horizontal(|ui| {
                                        any_changed |= ui
                                            .add(
                                                egui::Slider::new(slide_time_ms, 1000..=30000)
                                                    .suffix(" ms"),
                                            )
                                            .changed();
                                        any_changed |= ui.checkbox(loop_slides, "Loop").changed();
                                    });
                                    ui.end_row();
                                }
                                SourceKind::SceneSource { scene_name } => {
                                    ui.label("Target Scene:");
                                    any_changed |= ui.text_edit_singleline(scene_name).changed();
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
                                egui::Slider::new(&mut src.transform.scale, 0.10..=1.50)
                                    .fixed_decimals(2),
                            )
                            .changed();
                        any_changed |= ui
                            .checkbox(&mut src.transform.lock_aspect, "Lock Aspect")
                            .changed();
                        any_changed |= ui.checkbox(&mut src.locked, "Lock Layer").changed();
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Crop Margins (or Hold ALT + Drag edges in Preview)")
                                .strong(),
                        );
                        if src.transform.has_crop() && ui.small_button("Reset Crop").clicked() {
                            src.transform.reset_crop();
                            any_changed = true;
                        }
                    });
                    egui::Grid::new("source_crop_grid")
                        .num_columns(4)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Left:");
                            any_changed |= ui
                                .add(
                                    egui::Slider::new(&mut src.transform.crop_left, 0.0..=0.80)
                                        .fixed_decimals(2),
                                )
                                .changed();
                            ui.label("Right:");
                            any_changed |= ui
                                .add(
                                    egui::Slider::new(&mut src.transform.crop_right, 0.0..=0.80)
                                        .fixed_decimals(2),
                                )
                                .changed();
                            ui.end_row();

                            ui.label("Top:");
                            any_changed |= ui
                                .add(
                                    egui::Slider::new(&mut src.transform.crop_top, 0.0..=0.80)
                                        .fixed_decimals(2),
                                )
                                .changed();
                            ui.label("Bottom:");
                            any_changed |= ui
                                .add(
                                    egui::Slider::new(&mut src.transform.crop_bottom, 0.0..=0.80)
                                        .fixed_decimals(2),
                                )
                                .changed();
                            ui.end_row();
                        });

                    ui.separator();
                    ui.allocate_ui_with_layout(
                        Vec2::new(460.0, 26.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if ui.button("OK").clicked() {
                                close_clicked = true;
                            }
                        },
                    );
                } else {
                    ui.label("No source is currently selected.");
                    if ui.button("Close").clicked() {
                        close_clicked = true;
                    }
                }
            });

        if gpu_or_scale_changed {
            app.settings.preferred_gpu_index = Some(selected_gpu_idx);
            if let Some(g) = gpus.iter().find(|g| g.index == selected_gpu_idx) {
                app.settings.preferred_gpu_name = g.name.clone();
            }
            app.settings.media_decode_scale = selected_decode_scale;
            any_changed = true;
        }
        if refresh_catalog {
            app.capture_catalog.refresh_from_system();
        }
        if any_changed {
            app.studio.sync_media_transport_to_program();
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

    // 6. Dedicated Native OS Fullscreen / Windowed Projector (Multi-Monitor OBS Style)
    if app.show_projector_window {
        let target = app.projector_target.clone().unwrap_or(crate::app::ProjectorTarget {
            is_program: true,
            is_fullscreen: false,
            label: app.projector_target_label.clone(),
            left: 120,
            top: 120,
            width: 960,
            height: 540,
            snap_frames_remaining: 0,
        });

        let esc_in_main = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let esc_global = crate::platform::is_escape_pressed_global();
        if esc_in_main || esc_global {
            app.show_projector_window = false;
            app.projector_target = None;
            return;
        }

        let win_title = if target.is_fullscreen {
            "NEXORA Studio Projector".to_string()
        } else {
            let role_str = if target.is_program { "Program" } else { "Preview" };
            format!("NEXORA Studio — {} Projector ({})", role_str, target.label)
        };

        let viewport_id = if target.is_fullscreen {
            egui::ViewportId::from_hash_of(("nexora_projector_fs", target.left, target.top, target.width, target.height))
        } else {
            egui::ViewportId::from_hash_of("nexora_projector_windowed")
        };

        let mut builder = egui::ViewportBuilder::default()
            .with_title(&win_title)
            .with_position([target.left as f32, target.top as f32])
            .with_inner_size([target.width.max(320) as f32, target.height.max(180) as f32]);

        if target.is_fullscreen {
            builder = builder
                .with_decorations(false)
                .with_always_on_top()
                .with_resizable(false);
        } else {
            builder = builder
                .with_decorations(true)
                .with_min_inner_size([320.0, 180.0])
                .with_resizable(true);
        }

        let mut should_close = false;
        let scene_to_render = if target.is_program {
            app.studio.program_scene().clone()
        } else {
            app.studio.preview_scene().clone()
        };

        ctx.show_viewport_immediate(viewport_id, builder, |vp_ctx, _class| {
            let vp_close_or_esc = vp_ctx.input(|i| {
                i.viewport().close_requested() || i.key_pressed(egui::Key::Escape)
            });
            if vp_close_or_esc || crate::platform::is_escape_pressed_global() {
                should_close = true;
            }

            if target.is_fullscreen {
                if let Some(proj) = app.projector_target.as_mut() {
                    if proj.snap_frames_remaining > 0 {
                        let bring_front = proj.snap_frames_remaining >= 5;
                        let snapped = crate::platform::snap_window_to_monitor_rect(
                            &win_title,
                            proj.left,
                            proj.top,
                            proj.width as i32,
                            proj.height as i32,
                            bring_front,
                        );
                        if snapped {
                            proj.snap_frames_remaining = proj.snap_frames_remaining.saturating_sub(1);
                        }
                    }
                }
            }

            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(egui::Color32::BLACK)
                        .inner_margin(egui::Margin::ZERO),
                )
                .show(vp_ctx, |ui| {
                    let full_rect = ui.max_rect();
                    let response = ui.allocate_rect(full_rect, egui::Sense::click());
                    let painter = ui.painter_at(full_rect);
                    painter.rect_filled(full_rect, egui::CornerRadius::ZERO, egui::Color32::BLACK);

                    let avail = full_rect.size();
                    let aspect = 16.0 / 9.0;
                    let (w, h) = if avail.x / avail.y.max(1.0) > aspect {
                        (avail.y * aspect, avail.y)
                    } else {
                        (avail.x, avail.x / aspect)
                    };
                    let rect = egui::Rect::from_center_size(full_rect.center(), Vec2::new(w, h));
                    let egui_ctx = ui.ctx().clone();
                    let proj_painter = painter.with_clip_rect(rect);

                    for src in &scene_to_render.sources {
                        if !src.visible {
                            continue;
                        }
                        let (vis_x, vis_y, vis_w, vis_h) = src.transform.visible_norm_rect();
                        let src_w = rect.width() * vis_w;
                        let src_h = rect.height() * vis_h;
                        let src_min = egui::Pos2::new(
                            rect.min.x + rect.width() * vis_x,
                            rect.min.y + rect.height() * vis_y,
                        );
                        let layer_rect = egui::Rect::from_min_size(src_min, Vec2::new(src_w, src_h));
                        if layer_rect.intersect(rect).width() <= 2.0
                            || layer_rect.intersect(rect).height() <= 2.0
                        {
                            continue;
                        }

                        if let SourceKind::ColorSource { color_rgba, .. } = &src.kind {
                            proj_painter.rect_filled(
                                layer_rect,
                                egui::CornerRadius::ZERO,
                                egui::Color32::from_rgba_unmultiplied(
                                    color_rgba[0],
                                    color_rgba[1],
                                    color_rgba[2],
                                    color_rgba[3],
                                ),
                            );
                            continue;
                        }
                        if let SourceKind::TextGdiPlus { text, font_size, .. } = &src.kind {
                            let scaled_font = (*font_size as f32 * (rect.height() / 1080.0)).clamp(10.0, 96.0);
                            proj_painter.text(
                                layer_rect.left_top() + Vec2::new(6.0, 4.0),
                                egui::Align2::LEFT_TOP,
                                text,
                                egui::FontId::proportional(scaled_font),
                                egui::Color32::WHITE,
                            );
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

                        if let Some(tex) = live_tex {
                            let (u0, v0, u1, v1) = src.transform.uv_bounds();
                            proj_painter.image(
                                tex.id(),
                                layer_rect,
                                egui::Rect::from_min_max(
                                    egui::Pos2::new(u0, v0),
                                    egui::Pos2::new(u1, v1),
                                ),
                                egui::Color32::WHITE,
                            );
                        }
                    }

                    response.context_menu(|ui| {
                        ui.set_min_width(180.0);
                        if ui.button("Close Projector (Esc)").clicked() {
                            should_close = true;
                            ui.close_menu();
                        }
                    });
                });

            vp_ctx.request_repaint_after(std::time::Duration::from_millis(33));
        });

        if should_close {
            app.show_projector_window = false;
            app.projector_target = None;
        }
    }
}
