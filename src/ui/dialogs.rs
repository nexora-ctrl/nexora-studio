use crate::app::{NexoraApp, SettingsCategory, VirtualCameraOutputType};
use crate::settings::{BitratePreset, MediaDecodeScale, ThemeMode, VideoPreset};
use crate::studio::source::{
    MediaColorRange, SlideTransitionEffect, SourceColorTag, SourceItem, SourceKind,
    TextAnimationEffect, TimeFontFamily, TimeSourceMode, TimeVisualEffect, TvBannerStyle,
    TvTextTemplate,
};
use crate::ui::icons;
use crate::ui::theme::{apply_studio_theme, ThemePalette};
use egui::{Pos2, RichText, Stroke, Vec2};
use std::path::PathBuf;

/// Standardized singleline TextEdit with vertical centering and balanced margins
pub(crate) fn standard_singleline_text<'a>(text: &'a mut String, hint: &'a str) -> egui::TextEdit<'a> {
    let mut edit = egui::TextEdit::singleline(text)
        .desired_width(f32::INFINITY)
        .vertical_align(egui::Align::Center)
        .margin(egui::Margin::symmetric(6, 2));
    if !hint.is_empty() {
        edit = edit.hint_text(hint);
    }
    edit
}

pub(crate) fn standard_singleline_text_width<'a>(text: &'a mut String, hint: &'a str, width: f32) -> egui::TextEdit<'a> {
    let mut edit = egui::TextEdit::singleline(text)
        .desired_width(width)
        .vertical_align(egui::Align::Center)
        .margin(egui::Margin::symmetric(6, 2));
    if !hint.is_empty() {
        edit = edit.hint_text(hint);
    }
    edit
}

pub(crate) fn truncate_str(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        text.to_string()
    } else {
        let keep = max_chars.saturating_sub(1).max(6);
        let s: String = text.chars().take(keep).collect();
        format!("{}...", s)
    }
}

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

/// Sinks drag events inside the window content body so that modal windows can ONLY be dragged
/// by their Title Bar, preventing accidental window movement when clicking/dragging inside the body.
pub fn block_window_body_drag(ui: &egui::Ui) {
    ui.interact(
        ui.max_rect(),
        ui.id().with("__nexora_win_body_drag_blocker"),
        egui::Sense::drag(),
    );
}

pub fn render_dialogs(ctx: &egui::Context, app: &mut NexoraApp) {
    // Modal Backdrop / Scrim Shield: When any modal dialog is open, render a semi-transparent
    // backdrop across the viewport that intercepts all background clicks and drags so underlying
    // controls (canvas, dock, scene switches, recording controls) cannot be triggered accidentally.
    if app.is_any_modal_open() && !app.eyedropper_active {
        let screen_rect = ctx.screen_rect();
        let scrim_col = match app.settings.theme {
            ThemeMode::Dark => egui::Color32::from_black_alpha(130),
            ThemeMode::Light => egui::Color32::from_black_alpha(65),
        };
        egui::Area::new(egui::Id::new("nexora_modal_scrim_layer"))
            .order(egui::Order::Middle)
            .fixed_pos(screen_rect.min)
            .interactable(false)
            .show(ctx, |ui| {
                ui.allocate_response(screen_rect.size(), egui::Sense::click_and_drag());
                ui.painter()
                    .rect_filled(screen_rect, egui::CornerRadius::ZERO, scrim_col);
            });
    }

    // 1. Preferences / Settings Modal Dialog (2-Column Category Layout matching OBS / Preferences standard)
    if app.show_settings_dialog {
        let mut open = app.show_settings_dialog;
        let mut apply_triggered = false;
        let mut save_and_close = false;
        let mut cancel_triggered = false;
        let gpus = app.capture_catalog.gpus.clone();

        egui::Window::new(format!("{} Preferences", icons::SETTINGS))
            .id(egui::Id::new("nexora_preferences_modal_win"))
            .order(egui::Order::Foreground)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .fixed_size(Vec2::new(760.0, 485.0))
            .show(ctx, |ui| {
                block_window_body_drag(ui);
                let p = ThemePalette::for_mode(app.settings.theme);
                const LEFT_COL_W: f32 = 180.0;
                const RIGHT_COL_W: f32 = 540.0;
                const BODY_H: f32 = 385.0;
                const FIELD_W: f32 = 270.0;
                const FIELD_H: f32 = 24.0;

                ui.set_width(735.0);
                ui.set_min_width(735.0);
                ui.set_max_width(735.0);
                ui.set_height(BODY_H + 45.0);

                // Two-Column Body: Categories on left, Category Content Options on right
                ui.horizontal(|ui| {
                    // Left Column: Categories List
                    ui.allocate_ui_with_layout(
                        Vec2::new(LEFT_COL_W, BODY_H),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.label(
                                RichText::new("Categories")
                                    .size(11.5)
                                    .strong()
                                    .color(p.text_secondary),
                            );
                            ui.add_space(3.0);

                            // Inset frame container for categories list
                            egui::Frame::new()
                                .fill(p.bg_canvas_well)
                                .stroke(Stroke::new(1.0_f32, p.border_subtle))
                                .corner_radius(egui::CornerRadius::same(4))
                                .inner_margin(egui::Margin::symmetric(4, 4))
                                .show(ui, |ui| {
                                    ui.set_width(LEFT_COL_W - 8.0);
                                    ui.set_height(BODY_H - 25.0);

                                    let cat_items = [
                                        (SettingsCategory::General, icons::PALETTE, "General & Theme"),
                                        (SettingsCategory::VideoOutput, icons::VIDEO, "Video & Output"),
                                        (SettingsCategory::HardwarePerformance, icons::CPU, "Hardware & GPU"),
                                        (SettingsCategory::AudioMixer, icons::VOLUME, "Audio & Mixer"),
                                        (SettingsCategory::Update, icons::WORLD, "Update"),
                                        (SettingsCategory::About, icons::INFO_CIRCLE, "About & System"),
                                    ];

                                    for (cat, icon, label) in cat_items {
                                        let is_sel = app.settings_category == cat;
                                        let (rect, resp) = ui.allocate_exact_size(
                                            Vec2::new(LEFT_COL_W - 16.0, 32.0),
                                            egui::Sense::click(),
                                        );
                                        if resp.clicked() {
                                            app.settings_category = cat;
                                        }

                                        let bg_fill = if is_sel {
                                            p.accent_primary.gamma_multiply(0.20)
                                        } else if resp.hovered() {
                                            p.bg_elevated
                                        } else {
                                            egui::Color32::TRANSPARENT
                                        };

                                        ui.painter().rect_filled(
                                            rect,
                                            egui::CornerRadius::same(3),
                                            bg_fill,
                                        );

                                        if is_sel {
                                            // Active category left accent stripe
                                            let stripe_rect = egui::Rect::from_min_size(
                                                rect.min,
                                                Vec2::new(3.5, rect.height()),
                                            );
                                            ui.painter().rect_filled(
                                                stripe_rect,
                                                egui::CornerRadius { nw: 3, sw: 3, ne: 0, se: 0 },
                                                p.accent_primary,
                                            );
                                        }

                                        let text_col = if is_sel {
                                            p.text_primary
                                        } else {
                                            p.text_secondary
                                        };

                                        let text_pos = egui::Pos2::new(rect.min.x + 12.0, rect.center().y);
                                        ui.painter().text(
                                            text_pos,
                                            egui::Align2::LEFT_CENTER,
                                            format!("{}  {}", icon, label),
                                            egui::FontId::proportional(12.0),
                                            text_col,
                                        );
                                    }
                                });
                        },
                    );

                    ui.add_space(8.0);

                    // Right Column: Active Category Options
                    ui.allocate_ui_with_layout(
                        Vec2::new(RIGHT_COL_W, BODY_H),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            // Header banner matching reference: [Icon] Category Title Options
                            let (header_icon, header_title) = match app.settings_category {
                                SettingsCategory::General => (icons::PALETTE, "General & Appearance Options"),
                                SettingsCategory::VideoOutput => (icons::VIDEO, "Video & Recording Output Options"),
                                SettingsCategory::HardwarePerformance => (icons::CPU, "Hardware GPU & Engine Options"),
                                SettingsCategory::AudioMixer => (icons::VOLUME, "Audio System & Mixer Options"),
                                SettingsCategory::Update => (icons::WORLD, "Update Application Options"),
                                SettingsCategory::About => (icons::INFO_CIRCLE, "About Application & System Options"),
                            };

                            egui::Frame::new()
                                .fill(p.bg_canvas_well)
                                .stroke(Stroke::new(1.0_f32, p.border_subtle))
                                .corner_radius(egui::CornerRadius::same(4))
                                .inner_margin(egui::Margin::symmetric(10, 6))
                                .show(ui, |ui| {
                                    ui.set_width(RIGHT_COL_W - 20.0);
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(header_icon).size(16.0).color(p.accent_primary));
                                        ui.label(RichText::new(header_title).size(13.0).strong().color(p.text_primary));
                                    });
                                });

                            ui.add_space(6.0);

                            // Options Content Body inside card
                            egui::Frame::new()
                                .fill(p.bg_panel)
                                .stroke(Stroke::new(1.0_f32, p.border_subtle))
                                .corner_radius(egui::CornerRadius::same(4))
                                .inner_margin(egui::Margin::same(12))
                                .show(ui, |ui| {
                                    ui.set_width(RIGHT_COL_W - 24.0);
                                    ui.set_height(BODY_H - 62.0);

                                    match app.settings_category {
                                        SettingsCategory::General => {
                                            ui.label(RichText::new("TAMPILAN ANTARMUKA (THEME)").small().strong().color(p.text_muted));
                                            ui.add_space(4.0);

                                            egui::Grid::new("settings_general_grid")
                                                .num_columns(2)
                                                .spacing([12.0, 10.0])
                                                .show(ui, |ui| {
                                                    ui.label("UI Theme Mode:");
                                                    ui.horizontal(|ui| {
                                                        if ui.selectable_value(
                                                            &mut app.settings.theme,
                                                            ThemeMode::Dark,
                                                            format!("{} Dark Mode", icons::MOON),
                                                        ).clicked() {
                                                            apply_studio_theme(ctx, app.settings.theme);
                                                        }
                                                        if ui.selectable_value(
                                                            &mut app.settings.theme,
                                                            ThemeMode::Light,
                                                            format!("{} Light Mode", icons::SUN),
                                                        ).clicked() {
                                                            apply_studio_theme(ctx, app.settings.theme);
                                                        }
                                                    });
                                                    ui.end_row();

                                                    ui.label("Scenes Panel Mode:");
                                                    ui.horizontal(|ui| {
                                                        ui.selectable_value(
                                                            &mut app.settings.scenes_view_mode,
                                                            crate::settings::ScenesViewMode::List,
                                                            format!("{} List Mode", icons::LIST),
                                                        );
                                                        ui.selectable_value(
                                                            &mut app.settings.scenes_view_mode,
                                                            crate::settings::ScenesViewMode::Grid,
                                                            format!("{} Grid (16:9)", icons::LAYOUT_GRID),
                                                        );
                                                    });
                                                    ui.end_row();

                                                    ui.label("Workspace Mode:");
                                                    ui.checkbox(&mut app.settings.panels.studio_mode, "Studio Mode (Dual Preview & Program)");
                                                    ui.end_row();

                                                    ui.label("Preview Helpers:");
                                                    ui.vertical(|ui| {
                                                        ui.checkbox(&mut app.settings.show_safe_margins, "Show Safe Margin Guides (80% & 90%)");
                                                        ui.checkbox(&mut app.settings.preview_fit_to_window, "Fit to Window (Auto Canvas Scale)");
                                                    });
                                                    ui.end_row();
                                                });
                                        }

                                        SettingsCategory::VideoOutput => {
                                            ui.label(RichText::new("RECORDING RESOLUTION & ENCODING").small().strong().color(p.text_muted));
                                            ui.add_space(4.0);

                                            egui::Grid::new("settings_video_grid")
                                                .num_columns(2)
                                                .spacing([12.0, 10.0])
                                                .show(ui, |ui| {
                                                    ui.label("Resolution & FPS:");
                                                    egui::ComboBox::from_id_salt("pref_res_fps")
                                                        .selected_text(app.settings.video_preset.label())
                                                        .width(FIELD_W)
                                                        .show_ui(ui, |ui| {
                                                            for preset in VideoPreset::all() {
                                                                ui.selectable_value(&mut app.settings.video_preset, *preset, preset.label());
                                                            }
                                                        });
                                                    ui.end_row();

                                                    ui.label("Video Bitrate:");
                                                    egui::ComboBox::from_id_salt("pref_bitrate")
                                                        .selected_text(app.settings.bitrate_preset.label())
                                                        .width(FIELD_W)
                                                        .show_ui(ui, |ui| {
                                                            for br in BitratePreset::all() {
                                                                ui.selectable_value(&mut app.settings.bitrate_preset, *br, br.label());
                                                            }
                                                        });
                                                    ui.end_row();

                                                    ui.label("Container Format:");
                                                    ui.label(RichText::new("MP4 (H.264 Video + AAC Audio)").monospace());
                                                    ui.end_row();

                                                    ui.label("Output Directory:");
                                                    ui.vertical(|ui| {
                                                        let mut path_str = app.settings.output_directory.to_string_lossy().to_string();
                                                        ui.add_sized(
                                                            Vec2::new(FIELD_W, FIELD_H),
                                                            standard_singleline_text(&mut path_str, "Recordings folder..."),
                                                        );
                                                        app.settings.output_directory = PathBuf::from(path_str);

                                                        ui.add_space(3.0);
                                                        ui.horizontal(|ui| {
                                                            if ui.button(format!("{} Open Folder", icons::FOLDER_OPEN)).clicked() {
                                                                let _ = crate::platform::open_folder_in_explorer(&app.settings.output_directory);
                                                            }
                                                            if ui.button("Default Folder").clicked() {
                                                                app.settings.output_directory = crate::settings::AppSettings::default_recordings_dir();
                                                            }
                                                        });
                                                    });
                                                    ui.end_row();
                                                });
                                        }

                                        SettingsCategory::HardwarePerformance => {
                                            ui.label(RichText::new("HARDWARE & GRAPHICS ACCELERATION").small().strong().color(p.text_muted));
                                            ui.add_space(4.0);

                                            egui::Grid::new("settings_hw_grid")
                                                .num_columns(2)
                                                .spacing([12.0, 10.0])
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

                                                    egui::ComboBox::from_id_salt("pref_gpu_adapter")
                                                        .selected_text(selected_gpu_text.clone())
                                                        .width(FIELD_W)
                                                        .truncate()
                                                        .show_ui(ui, |ui| {
                                                            for gpu in &gpus {
                                                                let is_sel = app.settings.preferred_gpu_index == Some(gpu.index);
                                                                if ui.selectable_label(is_sel, gpu.display_label()).clicked() {
                                                                    app.settings.preferred_gpu_index = Some(gpu.index);
                                                                    app.settings.preferred_gpu_name = gpu.name.clone();
                                                                }
                                                            }
                                                        })
                                                        .response
                                                        .on_hover_text(selected_gpu_text);
                                                    ui.end_row();

                                                    ui.label("Media Decode Scale:");
                                                    egui::ComboBox::from_id_salt("pref_decode_scale")
                                                        .selected_text(app.settings.media_decode_scale.label())
                                                        .width(FIELD_W)
                                                        .truncate()
                                                        .show_ui(ui, |ui| {
                                                            for scale in MediaDecodeScale::all() {
                                                                ui.selectable_value(&mut app.settings.media_decode_scale, *scale, scale.label());
                                                            }
                                                        });
                                                    ui.end_row();

                                                    ui.label("Graphics Pipeline:");
                                                    ui.label(RichText::new("Direct3D 11 Hardware Compositor (NV12/BGRA)").monospace().size(11.0));
                                                    ui.end_row();

                                                    ui.label("Encoder Hardware:");
                                                    ui.label(RichText::new("Windows Media Foundation (MF H.264 HW)").monospace().size(11.0));
                                                    ui.end_row();
                                                });
                                        }

                                        SettingsCategory::AudioMixer => {
                                            ui.label(RichText::new("AUDIO SYSTEM & DEVICES").small().strong().color(p.text_muted));
                                            ui.add_space(4.0);

                                            egui::Grid::new("settings_audio_grid")
                                                .num_columns(2)
                                                .spacing([12.0, 10.0])
                                                .show(ui, |ui| {
                                                    ui.label("Master Sample Rate:");
                                                    ui.label(RichText::new("48.000 Hz (48 kHz Broadcast Standard)").monospace());
                                                    ui.end_row();

                                                    ui.label("Desktop Audio Device:");
                                                    egui::ComboBox::from_id_salt("pref_desk_audio")
                                                        .selected_text(truncate_str(&app.audio_mixer.desktop_audio.selected_device, 32))
                                                        .width(FIELD_W)
                                                        .truncate()
                                                        .show_ui(ui, |ui| {
                                                            for dev in &app.audio_mixer.desktop_audio.available_devices {
                                                                ui.selectable_value(&mut app.audio_mixer.desktop_audio.selected_device, dev.clone(), dev);
                                                            }
                                                        });
                                                    ui.end_row();

                                                    ui.label("Microphone Device:");
                                                    egui::ComboBox::from_id_salt("pref_mic_audio")
                                                        .selected_text(truncate_str(&app.audio_mixer.microphone.selected_device, 32))
                                                        .width(FIELD_W)
                                                        .truncate()
                                                        .show_ui(ui, |ui| {
                                                            for dev in &app.audio_mixer.microphone.available_devices {
                                                                ui.selectable_value(&mut app.audio_mixer.microphone.selected_device, dev.clone(), dev);
                                                            }
                                                        });
                                                    ui.end_row();

                                                    ui.label("Mixer Controls:");
                                                    if ui.button(format!("{} Reset Levels & Devices", icons::REFRESH)).clicked() {
                                                        app.audio_mixer.reset_levels();
                                                        app.audio_mixer.refresh_devices_from_system();
                                                    }
                                                    ui.end_row();
                                                });
                                        }


                                        SettingsCategory::Update => {
                                            ui.label(RichText::new("APPLICATION UPDATE").small().strong().color(p.text_muted));
                                            ui.add_space(8.0);

                                            let updater_arc = app.updater.clone();
                                            let (status, repo_url) = {
                                                let st = updater_arc.lock().unwrap();
                                                (st.status.clone(), st.repo_url.clone())
                                            };

                                            ui.vertical_centered(|ui| {
                                                ui.add_space(18.0);

                                                // Centered World/Globe Icon matching reference design
                                                ui.label(RichText::new(icons::WORLD).size(38.0).color(p.accent_primary));
                                                ui.add_space(6.0);

                                                ui.label(RichText::new("NEXORA Studio Auto-Updater").heading().strong());
                                                ui.add_space(2.0);
                                                ui.label(
                                                    RichText::new(format!("Installed Version: v{} (Native 64-bit)", crate::platform::updater::CURRENT_VERSION))
                                                        .small()
                                                        .color(p.text_secondary),
                                                );

                                                ui.add_space(20.0);

                                                match status {
                                                    crate::platform::updater::UpdateStatus::Idle => {
                                                        let check_btn = egui::Button::new(
                                                            RichText::new(format!("{}  Check for Updates Now", icons::REFRESH))
                                                                .size(13.0)
                                                                .strong()
                                                        )
                                                        .min_size(Vec2::new(280.0, 36.0));

                                                        if ui.add(check_btn).clicked() {
                                                            crate::platform::updater::check_for_updates_async(
                                                                app.updater.clone(),
                                                                ui.ctx().clone(),
                                                            );
                                                        }
                                                        ui.add_space(8.0);
                                                        ui.label(
                                                            RichText::new("Click to check for the latest release available on GitHub.")
                                                                .small()
                                                                .color(p.text_muted),
                                                        );
                                                    }

                                                    crate::platform::updater::UpdateStatus::Checking => {
                                                        ui.spinner();
                                                        ui.add_space(8.0);
                                                        ui.label(RichText::new("Connecting to GitHub server...").strong());
                                                        ui.label(RichText::new("Checking release tags and repository...").small().color(p.text_secondary));
                                                    }

                                                    crate::platform::updater::UpdateStatus::UpToDate { version, check_time } => {
                                                        const CARD_W: f32 = 460.0;

                                                        // 1. Centered Status Card
                                                        ui.allocate_ui_with_layout(
                                                            Vec2::new(CARD_W, 0.0),
                                                            egui::Layout::top_down(egui::Align::Center),
                                                            |ui| {
                                                                egui::Frame::new()
                                                                    .fill(p.bg_canvas_well)
                                                                    .stroke(Stroke::new(1.0_f32, p.meter_ok.gamma_multiply(0.45)))
                                                                    .corner_radius(egui::CornerRadius::same(6))
                                                                    .inner_margin(egui::Margin::symmetric(16, 14))
                                                                    .show(ui, |ui| {
                                                                        ui.set_width(CARD_W - 32.0);
                                                                        ui.horizontal(|ui| {
                                                                            ui.label(RichText::new(icons::CIRCLE_CHECK).size(28.0).color(p.meter_ok));
                                                                            ui.add_space(8.0);
                                                                            ui.vertical(|ui| {
                                                                                ui.label(RichText::new("Application is Up to Date").strong().color(p.meter_ok));
                                                                                ui.label(RichText::new(format!("NEXORA Studio v{} is the latest version.", version)).small());
                                                                                ui.label(RichText::new(format!("Last checked at {}", check_time)).small().color(p.text_muted));
                                                                            });
                                                                        });
                                                                    });
                                                            },
                                                        );

                                                        ui.add_space(14.0);

                                                        // 2. Centered Action Buttons
                                                        const BTN_GROUP_W: f32 = 135.0 + 10.0 + 135.0;
                                                        ui.allocate_ui_with_layout(
                                                            Vec2::new(BTN_GROUP_W, 32.0),
                                                            egui::Layout::left_to_right(egui::Align::Center),
                                                            |ui| {
                                                                let refresh_btn = egui::Button::new(
                                                                    RichText::new(format!("{}  Check Again", icons::REFRESH)),
                                                                )
                                                                .min_size(Vec2::new(135.0, 30.0));

                                                                if ui.add(refresh_btn).clicked() {
                                                                    crate::platform::updater::check_for_updates_async(
                                                                        app.updater.clone(),
                                                                        ui.ctx().clone(),
                                                                    );
                                                                }

                                                                ui.add_space(10.0);

                                                                let github_btn = egui::Button::new(
                                                                    RichText::new(format!("{}  Open GitHub", icons::BRAND_GITHUB)),
                                                                )
                                                                .min_size(Vec2::new(135.0, 30.0));

                                                                if ui.add(github_btn).clicked() {
                                                                    ui.ctx().open_url(egui::OpenUrl::new_tab(&repo_url));
                                                                }
                                                            },
                                                        );
                                                    }

                                                    crate::platform::updater::UpdateStatus::UpdateAvailable {
                                                        current_version,
                                                        latest_version,
                                                        release_title,
                                                        release_url,
                                                        download_url,
                                                        release_notes,
                                                        file_size_bytes,
                                                    } => {
                                                        const CARD_W: f32 = 470.0;
                                                        ui.allocate_ui_with_layout(
                                                            Vec2::new(CARD_W, 0.0),
                                                            egui::Layout::top_down(egui::Align::Center),
                                                            |ui| {
                                                                egui::Frame::new()
                                                                    .fill(p.accent_primary.gamma_multiply(0.12))
                                                                    .stroke(Stroke::new(1.2_f32, p.accent_primary))
                                                                    .corner_radius(egui::CornerRadius::same(6))
                                                                    .inner_margin(egui::Margin::symmetric(16, 14))
                                                                    .show(ui, |ui| {
                                                                        ui.set_width(CARD_W - 32.0);
                                                                        ui.horizontal(|ui| {
                                                                            ui.label(RichText::new(icons::CLOUD_DOWNLOAD).size(30.0).color(p.accent_primary));
                                                                            ui.add_space(8.0);
                                                                            ui.vertical(|ui| {
                                                                                ui.label(RichText::new("New Version Available!").heading().strong().color(p.accent_primary));
                                                                                ui.label(RichText::new(format!("Current Version: v{}  ➜  New Version: {}", current_version, latest_version)).strong());
                                                                                ui.label(RichText::new(format!("Release Title: {}", release_title)).small().color(p.text_secondary));
                                                                                if let Some(sz) = file_size_bytes {
                                                                                    let sz_mb = sz as f64 / (1024.0 * 1024.0);
                                                                                    ui.label(RichText::new(format!("Binary Size: {:.1} MB", sz_mb)).small().color(p.text_muted));
                                                                                }
                                                                            });
                                                                        });

                                                                        ui.add_space(8.0);
                                                                        ui.separator();
                                                                        ui.add_space(4.0);
                                                                        ui.label(RichText::new("Release Notes:").small().strong());
                                                                        ui.label(RichText::new(&release_notes).small().color(p.text_secondary));

                                                                        ui.add_space(10.0);
                                                                        ui.horizontal(|ui| {
                                                                            if let Some(ref dl_url) = download_url {
                                                                                let dl_btn = egui::Button::new(
                                                                                    RichText::new(format!("{}  Auto-Update Now", icons::DOWNLOAD))
                                                                                        .strong()
                                                                                        .color(egui::Color32::WHITE),
                                                                                )
                                                                                .fill(p.meter_ok)
                                                                                .min_size(Vec2::new(190.0, 32.0));

                                                                                if ui.add(dl_btn).clicked() {
                                                                                    crate::platform::updater::start_download_async(
                                                                                        app.updater.clone(),
                                                                                        dl_url.clone(),
                                                                                        latest_version.clone(),
                                                                                        ui.ctx().clone(),
                                                                                    );
                                                                                }
                                                                            }

                                                                            let gh_btn = egui::Button::new(
                                                                                RichText::new(format!("{}  View on GitHub", icons::BRAND_GITHUB)),
                                                                            )
                                                                            .min_size(Vec2::new(130.0, 32.0));

                                                                            if ui.add(gh_btn).clicked() {
                                                                                ui.ctx().open_url(egui::OpenUrl::new_tab(&release_url));
                                                                            }
                                                                        });
                                                                    });
                                                            },
                                                        );
                                                    }

                                                    crate::platform::updater::UpdateStatus::Downloading {
                                                        latest_version,
                                                        progress,
                                                        downloaded_mb: _,
                                                        total_mb: _,
                                                    } => {
                                                        const CARD_W: f32 = 460.0;
                                                        ui.allocate_ui_with_layout(
                                                            Vec2::new(CARD_W, 0.0),
                                                            egui::Layout::top_down(egui::Align::Center),
                                                            |ui| {
                                                                egui::Frame::new()
                                                                    .fill(p.bg_canvas_well)
                                                                    .stroke(Stroke::new(1.0_f32, p.accent_primary))
                                                                    .corner_radius(egui::CornerRadius::same(6))
                                                                    .inner_margin(egui::Margin::symmetric(16, 14))
                                                                    .show(ui, |ui| {
                                                                        ui.set_width(CARD_W - 32.0);
                                                                        ui.label(RichText::new(format!("Downloading Update {}...", latest_version)).strong());
                                                                        ui.add_space(6.0);
                                                                        ui.add(egui::ProgressBar::new(progress).show_percentage());
                                                                        ui.add_space(4.0);
                                                                        ui.label(
                                                                            RichText::new("Downloading new binary directly from GitHub. Please do not close the application...")
                                                                                .small()
                                                                                .color(p.text_secondary),
                                                                        );
                                                                    });
                                                            },
                                                        );
                                                    }

                                                    crate::platform::updater::UpdateStatus::ReadyToRestart {
                                                        ref new_version,
                                                        ref downloaded_path,
                                                    } => {
                                                        const CARD_W: f32 = 460.0;
                                                        ui.allocate_ui_with_layout(
                                                            Vec2::new(CARD_W, 0.0),
                                                            egui::Layout::top_down(egui::Align::Center),
                                                            |ui| {
                                                                egui::Frame::new()
                                                                    .fill(p.meter_ok.gamma_multiply(0.15))
                                                                    .stroke(Stroke::new(1.2_f32, p.meter_ok))
                                                                    .corner_radius(egui::CornerRadius::same(6))
                                                                    .inner_margin(egui::Margin::symmetric(16, 14))
                                                                    .show(ui, |ui| {
                                                                        ui.set_width(CARD_W - 32.0);
                                                                        ui.horizontal(|ui| {
                                                                            ui.label(RichText::new(icons::CHECK).size(26.0).color(p.meter_ok));
                                                                            ui.add_space(6.0);
                                                                            ui.vertical(|ui| {
                                                                                ui.label(RichText::new("Update Successfully Downloaded!").heading().strong().color(p.meter_ok));
                                                                                ui.label(RichText::new(format!("Version {} is ready to install.", new_version)).small());
                                                                            });
                                                                        });

                                                                        ui.add_space(10.0);
                                                                        let restart_btn = egui::Button::new(
                                                                            RichText::new(format!("{}  Restart Application Now", icons::REFRESH))
                                                                                .strong()
                                                                                .color(egui::Color32::WHITE),
                                                                        )
                                                                        .fill(p.meter_ok)
                                                                        .min_size(Vec2::new(220.0, 36.0));

                                                                        if ui.add(restart_btn).clicked() {
                                                                            if let Err(e) = crate::platform::updater::apply_update_and_restart(downloaded_path) {
                                                                                tracing::error!("Failed to apply update: {}", e);
                                                                            }
                                                                        }
                                                                    });
                                                            },
                                                        );
                                                    }

                                                    crate::platform::updater::UpdateStatus::Failed { ref error } => {
                                                        const CARD_W: f32 = 460.0;
                                                        ui.allocate_ui_with_layout(
                                                            Vec2::new(CARD_W, 0.0),
                                                            egui::Layout::top_down(egui::Align::Center),
                                                            |ui| {
                                                                egui::Frame::new()
                                                                    .fill(p.bg_canvas_well)
                                                                    .stroke(Stroke::new(1.0_f32, p.meter_warn.gamma_multiply(0.5)))
                                                                    .corner_radius(egui::CornerRadius::same(6))
                                                                    .inner_margin(egui::Margin::symmetric(14, 12))
                                                                    .show(ui, |ui| {
                                                                        ui.set_width(CARD_W - 32.0);
                                                                        ui.horizontal(|ui| {
                                                                            ui.label(RichText::new(icons::ALERT_CIRCLE).size(22.0).color(p.meter_warn));
                                                                            ui.add_space(6.0);
                                                                            ui.vertical(|ui| {
                                                                                ui.label(RichText::new("Update Notification").strong().color(p.meter_warn));
                                                                                ui.label(RichText::new(error).small());
                                                                            });
                                                                        });
                                                                    });

                                                                ui.add_space(12.0);
                                                                const BTN_GROUP_W: f32 = 145.0 + 10.0 + 155.0;
                                                                ui.allocate_ui_with_layout(
                                                                    Vec2::new(BTN_GROUP_W, 32.0),
                                                                    egui::Layout::left_to_right(egui::Align::Center),
                                                                    |ui| {
                                                                        if ui.add(egui::Button::new(format!("{}  Check Again", icons::REFRESH)).min_size(Vec2::new(145.0, 30.0))).clicked() {
                                                                            crate::platform::updater::check_for_updates_async(
                                                                                app.updater.clone(),
                                                                                ui.ctx().clone(),
                                                                            );
                                                                        }
                                                                        ui.add_space(10.0);
                                                                        if ui.add(egui::Button::new(format!("{}  Open GitHub Manually", icons::BRAND_GITHUB)).min_size(Vec2::new(155.0, 30.0))).clicked() {
                                                                            ui.ctx().open_url(egui::OpenUrl::new_tab(&repo_url));
                                                                        }
                                                                    },
                                                                );
                                                            },
                                                        );
                                                    }
                                                }
                                            });
                                        }

                                        SettingsCategory::About => {
                                            ui.label(RichText::new("ABOUT APPLICATION").small().strong().color(p.text_muted));
                                            ui.add_space(4.0);

                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new(icons::DEVICE_DESKTOP).size(26.0).color(p.accent_primary));
                                                ui.add_space(6.0);
                                                ui.vertical(|ui| {
                                                    ui.label(RichText::new("NEXORA Studio - Your Easy Studio").heading().strong());
                                                    ui.label(RichText::new("Version v0.4.0 (Native Windows 64-bit)").small().color(p.text_secondary));
                                                });
                                            });

                                            ui.add_space(8.0);
                                            ui.separator();
                                            ui.add_space(8.0);

                                            // Vision & Story Card
                                            egui::Frame::new()
                                                .fill(p.bg_canvas_well)
                                                .stroke(Stroke::new(1.0_f32, p.border_subtle))
                                                .corner_radius(egui::CornerRadius::same(6))
                                                .inner_margin(egui::Margin::symmetric(16, 14))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new(icons::SPARKLES).size(20.0).color(p.accent_primary));
                                                        ui.add_space(4.0);
                                                        ui.label(RichText::new("Our Vision").strong().size(13.5).color(p.accent_primary));
                                                    });
                                                    ui.add_space(8.0);
                                                    ui.label(
                                                        RichText::new("We believe technology should make creativity easier, not more complicated.")
                                                            .size(13.0)
                                                            .strong()
                                                            .color(p.text_primary),
                                                    );
                                                    ui.add_space(8.0);
                                                    ui.label(
                                                        RichText::new("NEXORA Studio was created with one simple vision: to bring recording, visual production, and creative tools together in one intuitive workspace.")
                                                            .size(12.5)
                                                            .color(p.text_secondary),
                                                    );
                                                    ui.add_space(10.0);
                                                    ui.label(
                                                        RichText::new("Built with passion, driven by innovation, and designed to make every creation easier.")
                                                            .size(12.0)
                                                            .italics()
                                                            .color(p.text_muted),
                                                    );
                                                });

                                            ui.add_space(10.0);
                                            ui.separator();
                                            ui.add_space(6.0);

                                            // Saweria Developer Support Card
                                            ui.label(RichText::new("DEVELOPER SUPPORT & DONATIONS").small().strong().color(p.text_muted));
                                            ui.add_space(4.0);

                                            egui::Frame::new()
                                                .fill(p.bg_canvas_well)
                                                .stroke(Stroke::new(1.0_f32, egui::Color32::from_rgb(225, 112, 85).gamma_multiply(0.45)))
                                                .corner_radius(egui::CornerRadius::same(6))
                                                .inner_margin(egui::Margin::symmetric(14, 10))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(RichText::new(icons::HEART).size(20.0).color(egui::Color32::from_rgb(225, 112, 85)));
                                                        ui.add_space(4.0);
                                                        ui.label(RichText::new("Support NEXORA Studio Developer").strong().size(13.0));
                                                    });
                                                    ui.add_space(3.0);
                                                    ui.label(
                                                        RichText::new("Voluntary donation via Saweria to support ongoing development and new features.")
                                                            .small()
                                                            .color(p.text_secondary),
                                                    );
                                                    ui.add_space(8.0);
                                                    let donate_btn = egui::Button::new(
                                                        RichText::new(format!("{}  Support via Saweria", icons::HEART))
                                                            .strong()
                                                            .size(12.5)
                                                            .color(egui::Color32::WHITE),
                                                    )
                                                    .fill(egui::Color32::from_rgb(225, 112, 85))
                                                    .min_size(Vec2::new(165.0, 30.0));

                                                    if ui.add(donate_btn).clicked() {
                                                        ui.ctx().open_url(egui::OpenUrl::new_tab("https://saweria.co/ilopejr"));
                                                    }
                                                });
                                        }
                                    }
                                });
                        },
                    );
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                // Footer Bar matching reference image: Status on left, [OK] [Apply] [Cancel] on right
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "Platform: {}",
                            crate::platform::target_os_summary()
                        ))
                        .small()
                        .color(p.text_muted),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Cancel button
                        if ui.add(egui::Button::new("Cancel").min_size(Vec2::new(72.0, 24.0))).clicked() {
                            cancel_triggered = true;
                        }

                        // Apply button
                        if ui.add(egui::Button::new("Apply").min_size(Vec2::new(72.0, 24.0))).clicked() {
                            apply_triggered = true;
                        }

                        // OK button
                        if ui.add(egui::Button::new(RichText::new("OK").strong()).min_size(Vec2::new(72.0, 24.0))).clicked() {
                            save_and_close = true;
                        }
                    });
                });
            });

        if save_and_close {
            let _ = app.settings.save();
            apply_studio_theme(ctx, app.settings.theme);
            app.show_settings_dialog = false;
        } else if apply_triggered {
            let _ = app.settings.save();
            apply_studio_theme(ctx, app.settings.theme);
        } else if cancel_triggered || !open {
            app.show_settings_dialog = false;
        }
    }

    // 1b. Add Scene Modal Dialog (Before creating scene, user can enter name or keep default)
    if app.show_add_scene_dialog {
        let mut open = app.show_add_scene_dialog;
        let mut confirm_add = false;
        let mut cancel_add = false;

        egui::Window::new(format!("{} Add Scene", icons::PLUS))
            .id(egui::Id::new("nexora_add_scene_modal_win"))
            .order(egui::Order::Foreground)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .auto_sized()
            .default_width(320.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                block_window_body_drag(ui);
                ui.add_space(4.0);
                ui.label(RichText::new("Enter new scene name:").strong());
                ui.label(
                    RichText::new("Press OK to use default name, or enter a custom scene name.")
                        .size(11.0)
                        .color(ui.visuals().weak_text_color()),
                );
                ui.add_space(6.0);

                let focus_id = ui.id().with("add_scene_focus_init");
                let should_focus = ui.data_mut(|d| d.get_temp::<bool>(focus_id).unwrap_or(true));

                let text_resp = ui.add_sized(
                    Vec2::new(290.0, 24.0),
                    standard_singleline_text(&mut app.add_scene_name_buffer, "Scene name..."),
                );

                if should_focus {
                    text_resp.request_focus();
                    ui.data_mut(|d| d.insert_temp(focus_id, false));
                }

                if text_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    confirm_add = true;
                }
                if text_resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    confirm_add = true;
                }

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if ui
                        .add(egui::Button::new(RichText::new("  OK  ").strong()).min_size(Vec2::new(65.0, 24.0)))
                        .clicked()
                    {
                        confirm_add = true;
                    }
                    if ui
                        .add(egui::Button::new("Cancel").min_size(Vec2::new(65.0, 24.0)))
                        .clicked()
                    {
                        cancel_add = true;
                    }
                });
            });

        if confirm_add {
            let mut final_name = app.add_scene_name_buffer.trim().to_string();
            if final_name.is_empty() {
                final_name = app.studio.generate_unique_scene_name();
            }
            app.studio.add_scene(final_name);
            app.show_add_scene_dialog = false;
        } else if cancel_add || !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            app.show_add_scene_dialog = false;
        }
    }

    // 2. Rename Scene Modal Dialog
    if app.show_rename_scene_dialog {
        let mut open = app.show_rename_scene_dialog;
        egui::Window::new("Rename Scene")
            .id(egui::Id::new("nexora_rename_scene_modal_win"))
            .order(egui::Order::Foreground)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .auto_sized()
            .default_width(290.0)
            .show(ctx, |ui| {
                block_window_body_drag(ui);
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
            .id(egui::Id::new("nexora_rename_source_modal_win"))
            .order(egui::Order::Foreground)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .auto_sized()
            .default_width(290.0)
            .show(ctx, |ui| {
                block_window_body_drag(ui);
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
        let mut open_slideshow_manager = false;

        let selected_src = app.studio.preview_scene().selected_source();
        let (title, window_id) = if let Some(src) = selected_src {
            let display_name = truncate_str(&src.name, 26);
            (
                format!("Properties for '{}'", display_name),
                egui::Id::new("nexora_source_props_modal_win").with(src.id),
            )
        } else {
            (
                "Source Properties".to_string(),
                egui::Id::new("nexora_source_props_modal_win"),
            )
        };

        egui::Window::new(title)
            .id(window_id)
            .order(egui::Order::Foreground)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_width(515.0)
            .max_width(515.0)
            .auto_sized()
            .default_height(250.0)
            .show(ctx, |ui| {
                block_window_body_drag(ui);
                let p = ThemePalette::for_mode(app.settings.theme);
                const PROP_FIELD_W: f32 = 280.0;
                const PROP_FIELD_H: f32 = 24.0;
                ui.set_width(495.0);
                ui.set_max_width(495.0);

                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                    let mut pending_tv_template: Option<TvTextTemplate> = None;
                    egui::Grid::new("source_props_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Source Name:");
                            any_changed |= ui
                                .add_sized(
                                    Vec2::new(PROP_FIELD_W, PROP_FIELD_H),
                                    standard_singleline_text(&mut src.name, "Nama source..."),
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
                                let is_device_source = matches!(
                                    src.kind,
                                    SourceKind::DisplayCapture { .. }
                                        | SourceKind::WindowCapture { .. }
                                        | SourceKind::GameCapture { .. }
                                        | SourceKind::WebcamDevice { .. }
                                );
                                if is_device_source {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(format!(
                                                    "{} Refresh Devices",
                                                    icons::REFRESH
                                                ))
                                                .size(11.5),
                                            )
                                            .min_size(Vec2::new(116.0, 22.0)),
                                        )
                                        .on_hover_text("Re-scan connected displays, cameras, and open windows")
                                        .clicked()
                                    {
                                        refresh_catalog = true;
                                    }
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
                                    corner_radius,
                                    border_thickness,
                                    border_color_rgba,
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

                                    ui.label("Corner Radius:");
                                    ui.horizontal(|ui| {
                                        any_changed |= ui.add(egui::Slider::new(corner_radius, 0.0..=160.0).suffix(" px")).changed();
                                        if ui.small_button("0").on_hover_text("Standard square (0 px)").clicked() {
                                            *corner_radius = 0.0;
                                            any_changed = true;
                                        }
                                        if ui.small_button("16").on_hover_text("Soft rounded (16 px)").clicked() {
                                            *corner_radius = 16.0;
                                            any_changed = true;
                                        }
                                        if ui.small_button("32").on_hover_text("Medium rounded (32 px)").clicked() {
                                            *corner_radius = 32.0;
                                            any_changed = true;
                                        }
                                        if ui.small_button("Circle").on_hover_text("Full circle / avatar").clicked() {
                                            *corner_radius = 999.0;
                                            any_changed = true;
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Border Width:");
                                    ui.horizontal(|ui| {
                                        any_changed |= ui.add(egui::Slider::new(border_thickness, 0.0..=20.0).suffix(" px")).changed();
                                        if ui.small_button("Off").on_hover_text("No outline / border (0 px)").clicked() {
                                            *border_thickness = 0.0;
                                            any_changed = true;
                                        }
                                        if ui.small_button("2px").clicked() {
                                            *border_thickness = 2.0;
                                            any_changed = true;
                                        }
                                        if ui.small_button("4px").clicked() {
                                            *border_thickness = 4.0;
                                            any_changed = true;
                                        }
                                        if ui.small_button("6px").clicked() {
                                            *border_thickness = 6.0;
                                            any_changed = true;
                                        }
                                    });
                                    ui.end_row();

                                    if *border_thickness > 0.0 {
                                        ui.label("Border Color:");
                                        ui.horizontal(|ui| {
                                            any_changed |= ui.color_edit_button_srgba_unmultiplied(border_color_rgba).changed();
                                            let [r, g, b, _] = *border_color_rgba;
                                            ui.label(format!("#{:02X}{:02X}{:02X}", r, g, b));

                                            let color_presets: &[([u8; 4], &str)] = &[
                                                ([255, 255, 255, 255], "White"),
                                                ([0, 210, 255, 255], "Cyan"),
                                                ([168, 85, 247, 255], "Purple"),
                                                ([239, 68, 68, 255], "Red"),
                                                ([34, 197, 94, 255], "Green"),
                                                ([245, 158, 11, 255], "Gold"),
                                            ];
                                            for &(col, tip) in color_presets {
                                                let btn_col = egui::Color32::from_rgb(col[0], col[1], col[2]);
                                                let (sq_rect, sq_resp) = ui.allocate_exact_size(egui::Vec2::new(16.0, 16.0), egui::Sense::click());
                                                ui.painter().rect_filled(sq_rect, egui::CornerRadius::same(3), btn_col);
                                                ui.painter().rect_stroke(sq_rect, egui::CornerRadius::same(3), egui::Stroke::new(1.0_f32, egui::Color32::from_gray(80)), egui::StrokeKind::Inside);
                                                if sq_resp.on_hover_text(tip).clicked() {
                                                    *border_color_rgba = col;
                                                    any_changed = true;
                                                }
                                            }
                                        });
                                        ui.end_row();
                                    }
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
                                        let btn_w = 84.0;
                                        let input_w = (PROP_FIELD_W - btn_w - ui.spacing().item_spacing.x).max(120.0);
                                        any_changed |= ui
                                            .add_sized(
                                                Vec2::new(input_w, PROP_FIELD_H),
                                                standard_singleline_text(file_path, "Select .mp4, .mkv, .mp3, .wav..."),
                                            )
                                            .changed();
                                        if ui
                                            .add(
                                                egui::Button::new(format!("{} Browse...", icons::FOLDER_OPEN))
                                                    .min_size(Vec2::new(btn_w, PROP_FIELD_H)),
                                            )
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
                                        let btn_w = 84.0;
                                        let input_w = (PROP_FIELD_W - btn_w - ui.spacing().item_spacing.x).max(120.0);
                                        any_changed |= ui
                                            .add_sized(
                                                Vec2::new(input_w, PROP_FIELD_H),
                                                standard_singleline_text(file_path, "Select .png, .jpg, .webp..."),
                                            )
                                            .changed();
                                        if ui
                                            .add(
                                                egui::Button::new(format!("{} Browse...", icons::FOLDER_OPEN))
                                                    .min_size(Vec2::new(btn_w, PROP_FIELD_H)),
                                            )
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
                                    effect,
                                    scroll_speed,
                                    loop_animation,
                                    loop_gap_px,
                                    banner_style,
                                    bg_color_rgba,
                                    accent_color_rgba,
                                    badge_text,
                                    text_shadow,
                                } => {
                                    ui.label("TV Show Template:");
                                    egui::ComboBox::from_id_salt("prop_tv_text_template_combo")
                                        .selected_text("Select TV Text Show Template (1-Click)...")
                                        .width(PROP_FIELD_W)
                                        .truncate()
                                        .show_ui(ui, |ui| {
                                            for tpl in TvTextTemplate::all() {
                                                if ui
                                                    .selectable_label(false, tpl.label())
                                                    .clicked()
                                                {
                                                    pending_tv_template = Some(*tpl);
                                                }
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("TV Banner Style:");
                                    egui::ComboBox::from_id_salt("prop_tv_banner_style_combo")
                                        .selected_text(banner_style.label())
                                        .width(PROP_FIELD_W)
                                        .truncate()
                                        .show_ui(ui, |ui| {
                                            for style in TvBannerStyle::all() {
                                                any_changed |= ui
                                                    .selectable_value(
                                                        banner_style,
                                                        *style,
                                                        style.label(),
                                                    )
                                                    .changed();
                                            }
                                        });
                                    ui.end_row();

                                    if banner_style.uses_badge() {
                                        ui.label("TV Badge / Tag:");
                                        ui.horizontal(|ui| {
                                            let color_w = 76.0;
                                            let input_w = (PROP_FIELD_W - color_w - ui.spacing().item_spacing.x).max(120.0);
                                            any_changed |= ui
                                                .add_sized(
                                                    Vec2::new(input_w, PROP_FIELD_H),
                                                    standard_singleline_text(badge_text, "BREAKING NEWS / LIVE"),
                                                )
                                                .changed();
                                            ui.label("Accent:");
                                            any_changed |= ui
                                                .color_edit_button_srgba_unmultiplied(
                                                    accent_color_rgba,
                                                )
                                                .changed();
                                        });
                                        ui.end_row();
                                    }

                                    if banner_style.uses_background() {
                                        ui.label("Banner Background:");
                                        ui.horizontal(|ui| {
                                            any_changed |= ui
                                                .color_edit_button_srgba_unmultiplied(bg_color_rgba)
                                                .changed();
                                            ui.label(
                                                RichText::new("(Set TV bar color & opacity)")
                                                    .small(),
                                            );
                                        });
                                        ui.end_row();
                                    }

                                    ui.label("Text Content:");
                                    any_changed |= ui
                                        .add_sized(
                                            Vec2::new(PROP_FIELD_W, 64.0),
                                            egui::TextEdit::multiline(text)
                                                .desired_rows(3),
                                        )
                                        .changed();
                                    ui.end_row();

                                    ui.label("Font Size & Color:");
                                    ui.horizontal(|ui| {
                                        any_changed |= ui
                                            .add(
                                                egui::Slider::new(font_size, 12.0..=96.0)
                                                    .fixed_decimals(0),
                                            )
                                            .changed();
                                        any_changed |= ui
                                            .color_edit_button_srgba_unmultiplied(color_rgba)
                                            .changed();
                                        any_changed |=
                                            ui.checkbox(text_shadow, "Shadow").changed();
                                    });
                                    ui.end_row();

                                    ui.label("Text Effect / Motion:");
                                    egui::ComboBox::from_id_salt("prop_text_effect_combo")
                                        .selected_text(effect.label())
                                        .width(PROP_FIELD_W)
                                        .truncate()
                                        .show_ui(ui, |ui| {
                                            for eff in TextAnimationEffect::all() {
                                                any_changed |= ui
                                                    .selectable_value(effect, *eff, eff.label())
                                                    .changed();
                                            }
                                        });
                                    ui.end_row();

                                    if effect.is_animated() {
                                        ui.label("Loop & Speed:");
                                        ui.vertical(|ui| {
                                            any_changed |= ui
                                                .checkbox(
                                                    loop_animation,
                                                    "Continuous Seamless Loop",
                                                )
                                                .changed();
                                            ui.horizontal(|ui| {
                                                ui.label("Speed:");
                                                any_changed |= ui
                                                    .add(
                                                        egui::Slider::new(
                                                            scroll_speed,
                                                            15.0..=360.0,
                                                        )
                                                        .suffix(" px/s")
                                                        .fixed_decimals(0),
                                                    )
                                                    .changed();
                                                if ui.small_button("Slow").clicked() {
                                                    *scroll_speed = 55.0;
                                                    any_changed = true;
                                                }
                                                if ui.small_button("Normal").clicked() {
                                                    *scroll_speed = 105.0;
                                                    any_changed = true;
                                                }
                                                if ui.small_button("Fast").clicked() {
                                                    *scroll_speed = 185.0;
                                                    any_changed = true;
                                                }
                                            });
                                        });
                                        ui.end_row();
                                    }

                                    if effect.is_scrolling() && *loop_animation {
                                        ui.label("Loop Gap Spacing:");
                                        any_changed |= ui
                                            .add(
                                                egui::Slider::new(loop_gap_px, 20.0..=260.0)
                                                    .suffix(" px")
                                                    .fixed_decimals(0),
                                            )
                                            .on_hover_text(
                                                "Jarak spasi antar pengulangan teks berjalan (loop)",
                                            )
                                            .changed();
                                        ui.end_row();
                                    }
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
                                    transition_effect,
                                    transition_duration_ms,
                                } => {
                                    ui.label("Slide Images:");
                                    ui.horizontal(|ui| {
                                        let btn_w = (PROP_FIELD_W - 8.0) * 0.5;
                                        if ui
                                            .add(
                                                egui::Button::new(format!(
                                                    "{} Add Images...",
                                                    icons::PLUS
                                                ))
                                                .min_size(Vec2::new(btn_w, 24.0)),
                                            )
                                            .on_hover_text(
                                                "Hold Ctrl or Shift in Explorer to select multiple photos at once (.png, .jpg, .jpeg, .bmp, .webp, .gif)",
                                            )
                                            .clicked()
                                        {
                                            let picked_list =
                                                crate::platform::pick_multiple_image_files_dialog();
                                            if !picked_list.is_empty() {
                                                for p in picked_list {
                                                    let s = p.to_string_lossy().to_string();
                                                    if !s.trim().is_empty() {
                                                        files.push(s);
                                                    }
                                                }
                                                any_changed = true;
                                            }
                                        }
                                        if ui
                                            .add(
                                                egui::Button::new(format!(
                                                    "{} Manage ({})...",
                                                    icons::PHOTO,
                                                    files.len()
                                                ))
                                                .min_size(Vec2::new(btn_w, 24.0)),
                                            )
                                            .on_hover_text(
                                                "Open Slide Show Manager window with photo thumbnails to reorder or delete images",
                                            )
                                            .clicked()
                                        {
                                            open_slideshow_manager = true;
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Slide Transition:");
                                    egui::ComboBox::from_id_salt("prop_slide_transition_combo")
                                        .selected_text(transition_effect.label())
                                        .width(PROP_FIELD_W)
                                        .truncate()
                                        .show_ui(ui, |ui| {
                                            for eff in SlideTransitionEffect::all() {
                                                any_changed |= ui
                                                    .selectable_value(
                                                        transition_effect,
                                                        *eff,
                                                        eff.label(),
                                                    )
                                                    .changed();
                                            }
                                        });
                                    ui.end_row();

                                    if *transition_effect != SlideTransitionEffect::Cut {
                                        ui.label("Transition Speed:");
                                        any_changed |= ui
                                            .add(
                                                egui::Slider::new(
                                                    transition_duration_ms,
                                                    150..=2000,
                                                )
                                                .suffix(" ms"),
                                            )
                                            .changed();
                                        ui.end_row();
                                    }

                                    ui.label("Time Per Slide:");
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
                                SourceKind::PowerPointSource {
                                    presentation_name,
                                    file_path,
                                    window_hwnd,
                                    slide_index,
                                    slide_count,
                                    slides,
                                    ..
                                } => {
                                    if slides.is_empty() && !file_path.is_empty() && *window_hwnd == 0 {
                                        let exported = crate::platform::get_or_export_ppt_slides(file_path);
                                        if !exported.is_empty() {
                                            *slide_count = exported.len();
                                            *slides = exported;
                                            any_changed = true;
                                        }
                                    }

                                    ui.label("Presentasi:");
                                    ui.horizontal(|ui| {
                                        let btn_w = 84.0;
                                        let input_w = (PROP_FIELD_W - btn_w - ui.spacing().item_spacing.x).max(120.0);
                                        any_changed |= ui
                                            .add_sized(
                                                Vec2::new(input_w, PROP_FIELD_H),
                                                standard_singleline_text_width(presentation_name, "Nama presentasi PPT...", input_w),
                                            )
                                            .changed();
                                        if ui
                                            .add(
                                                egui::Button::new(format!("{} Browse", icons::FOLDER_OPEN))
                                                    .min_size(Vec2::new(btn_w, PROP_FIELD_H)),
                                            )
                                            .clicked()
                                        {
                                            if let Some(path) = crate::platform::pick_powerpoint_file_dialog() {
                                                let p_str = path.to_string_lossy().to_string();
                                                let exported = crate::platform::get_or_export_ppt_slides(&p_str);
                                                *file_path = p_str;
                                                *slides = exported.clone();
                                                *slide_count = exported.len();
                                                *slide_index = 0;
                                                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                                    *presentation_name = format!("PPT - {}", stem);
                                                }
                                                any_changed = true;
                                            }
                                        }
                                    });
                                    ui.end_row();

                                    if *slide_count > 0 {
                                        ui.label("Active Slide:");
                                        ui.horizontal(|ui| {
                                            let can_prev = *slide_index > 0;
                                            let can_next = *slide_index + 1 < *slide_count;
                                            if ui.add_enabled(can_prev, egui::Button::new(format!("{} Prev", icons::ARROW_LEFT)).min_size(Vec2::new(64.0, PROP_FIELD_H))).clicked() {
                                                *slide_index = slide_index.saturating_sub(1);
                                                any_changed = true;
                                            }
                                            ui.label(RichText::new(format!("Slide {} of {}", *slide_index + 1, slide_count)).strong());
                                            if ui.add_enabled(can_next, egui::Button::new(format!("Next {}", icons::ARROW_RIGHT)).min_size(Vec2::new(64.0, PROP_FIELD_H))).clicked() {
                                                *slide_index = (*slide_index + 1).min(slide_count.saturating_sub(1));
                                                any_changed = true;
                                            }
                                        });
                                        ui.end_row();
                                    }

                                    ui.label("PowerPoint Window:");
                                    let windows = app.capture_catalog.windows.clone();
                                    let current_hwnd = *window_hwnd;
                                    let current_title = windows.iter().find(|w| w.hwnd == current_hwnd).map(|w| w.title.as_str()).unwrap_or_else(|| {
                                        if current_hwnd == 0 { "Auto-detect Presentation Window" } else { "Window Not Found" }
                                    });
                                    egui::ComboBox::from_id_salt("prop_ppt_win_combo")
                                        .selected_text(current_title)
                                        .width(PROP_FIELD_W)
                                        .truncate()
                                        .show_ui(ui, |ui| {
                                            if ui.selectable_value(window_hwnd, 0, "Auto-detect Presentation Window").clicked() {
                                                any_changed = true;
                                            }
                                            for w in &windows {
                                                if ui.selectable_value(window_hwnd, w.hwnd, format!("[{}] {}", w.executable, w.title)).clicked() {
                                                    *presentation_name = w.title.clone();
                                                    any_changed = true;
                                                }
                                            }
                                        });
                                    ui.end_row();

                                    if !file_path.is_empty() {
                                        ui.label("File Path:");
                                        ui.add_sized(
                                            Vec2::new(PROP_FIELD_W, PROP_FIELD_H),
                                            egui::Label::new(RichText::new(file_path.as_str()).small().color(p.text_muted)).truncate(),
                                        );
                                        ui.end_row();
                                    }
                                }
                                SourceKind::PdfSource {
                                    document_name,
                                    file_path,
                                    page_index,
                                    page_count,
                                    pages,
                                    ..
                                } => {
                                    if pages.is_empty() && !file_path.is_empty() {
                                        let exported = crate::platform::get_or_export_pdf_pages(file_path);
                                        if !exported.is_empty() {
                                            *page_count = exported.len();
                                            *pages = exported;
                                            any_changed = true;
                                        }
                                    }

                                    ui.label("Dokumen PDF:");
                                    ui.horizontal(|ui| {
                                        let btn_w = 84.0;
                                        let input_w = (PROP_FIELD_W - btn_w - ui.spacing().item_spacing.x).max(120.0);
                                        any_changed |= ui
                                            .add_sized(
                                                Vec2::new(input_w, PROP_FIELD_H),
                                                standard_singleline_text_width(document_name, "Nama dokumen PDF...", input_w),
                                            )
                                            .changed();
                                        if ui
                                            .add(
                                                egui::Button::new(format!("{} Browse", icons::FOLDER_OPEN))
                                                    .min_size(Vec2::new(btn_w, PROP_FIELD_H)),
                                            )
                                            .clicked()
                                        {
                                            if let Some(path) = crate::platform::pick_pdf_file_dialog() {
                                                let p_str = path.to_string_lossy().to_string();
                                                let exported = crate::platform::get_or_export_pdf_pages(&p_str);
                                                *file_path = p_str;
                                                *pages = exported.clone();
                                                *page_count = exported.len();
                                                *page_index = 0;
                                                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                                    *document_name = format!("PDF - {}", stem);
                                                }
                                                any_changed = true;
                                            }
                                        }
                                    });
                                    ui.end_row();

                                    if *page_count > 0 {
                                        ui.label("Active Page:");
                                        ui.horizontal(|ui| {
                                            let can_prev = *page_index > 0;
                                            let can_next = *page_index + 1 < *page_count;
                                            if ui
                                                .add_enabled(
                                                    can_prev,
                                                    egui::Button::new(format!("{} Prev", icons::ARROW_LEFT))
                                                        .min_size(Vec2::new(64.0, PROP_FIELD_H)),
                                                )
                                                .clicked()
                                            {
                                                *page_index = page_index.saturating_sub(1);
                                                any_changed = true;
                                            }
                                            ui.label(RichText::new(format!("Page {} of {}", *page_index + 1, page_count)).strong());
                                            if ui
                                                .add_enabled(
                                                    can_next,
                                                    egui::Button::new(format!("Next {}", icons::ARROW_RIGHT))
                                                        .min_size(Vec2::new(64.0, PROP_FIELD_H)),
                                                )
                                                .clicked()
                                            {
                                                *page_index = (*page_index + 1).min(page_count.saturating_sub(1));
                                                any_changed = true;
                                            }
                                        });
                                        ui.end_row();
                                    }

                                    if !file_path.is_empty() {
                                        ui.label("File Path:");
                                        ui.add_sized(
                                            Vec2::new(PROP_FIELD_W, PROP_FIELD_H),
                                            egui::Label::new(RichText::new(file_path.as_str()).small().color(p.text_muted)).truncate(),
                                        );
                                        ui.end_row();
                                    }
                                }
                                SourceKind::TimeSource {
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
                                    blink_colon: _,
                                    use_24h_format,
                                    badge_text,
                                    badge_font_size,
                                    badge_position,
                                    badge_offset_x,
                                    badge_offset_y,
                                    finish_text,
                                } => {
                                    ui.label("Time Mode:");
                                    egui::ComboBox::from_id_salt("prop_time_mode")
                                        .selected_text(mode.label())
                                        .width(PROP_FIELD_W)
                                        .show_ui(ui, |ui| {
                                            for m in TimeSourceMode::all() {
                                                any_changed |= ui.selectable_value(mode, *m, m.label()).changed();
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Time Controls:");
                                    ui.horizontal(|ui| {
                                        let (run_icon, run_label) = if *is_running {
                                            (icons::PLAYER_PAUSE, "Pause")
                                        } else {
                                            (icons::PLAYER_PLAY, "Start")
                                        };
                                        if ui.button(format!("{} {}", run_icon, run_label)).clicked() {
                                            *is_running = !*is_running;
                                            any_changed = true;
                                        }
                                        if ui.button(format!("{} Reset", icons::REFRESH)).clicked() {
                                            *elapsed_sec = 0.0;
                                            any_changed = true;
                                        }

                                        if *mode != TimeSourceMode::RealTimeClock {
                                            if ui.button("-1m").on_hover_text("Rewind 1 minute").clicked() {
                                                if *mode == TimeSourceMode::CountdownTimer {
                                                    *elapsed_sec = (*elapsed_sec + 60.0).min(*target_duration_sec as f32);
                                                } else {
                                                    *elapsed_sec = (*elapsed_sec - 60.0).max(0.0);
                                                }
                                                any_changed = true;
                                            }
                                            if ui.button("+1m").on_hover_text("Advance 1 minute").clicked() {
                                                if *mode == TimeSourceMode::CountdownTimer {
                                                    *elapsed_sec = (*elapsed_sec - 60.0).max(0.0);
                                                } else {
                                                    *elapsed_sec += 60.0;
                                                }
                                                any_changed = true;
                                            }
                                            if ui.button("-10s").on_hover_text("Rewind 10 seconds").clicked() {
                                                if *mode == TimeSourceMode::CountdownTimer {
                                                    *elapsed_sec = (*elapsed_sec + 10.0).min(*target_duration_sec as f32);
                                                } else {
                                                    *elapsed_sec = (*elapsed_sec - 10.0).max(0.0);
                                                }
                                                any_changed = true;
                                            }
                                            if ui.button("+10s").on_hover_text("Advance 10 seconds").clicked() {
                                                if *mode == TimeSourceMode::CountdownTimer {
                                                    *elapsed_sec = (*elapsed_sec - 10.0).max(0.0);
                                                } else {
                                                    *elapsed_sec += 10.0;
                                                }
                                                any_changed = true;
                                            }
                                        }
                                    });
                                    ui.end_row();

                                    if *mode == TimeSourceMode::CountdownTimer {
                                        ui.label("Target Duration:");
                                        ui.horizontal(|ui| {
                                            let mut h = *target_duration_sec / 3600;
                                            let mut m = (*target_duration_sec % 3600) / 60;
                                            let mut s = *target_duration_sec % 60;

                                            let mut changed_dur = false;
                                            ui.label("Hr:");
                                            changed_dur |= ui.add(egui::DragValue::new(&mut h).range(0..=23)).changed();
                                            ui.label("Min:");
                                            changed_dur |= ui.add(egui::DragValue::new(&mut m).range(0..=59)).changed();
                                            ui.label("Sec:");
                                            changed_dur |= ui.add(egui::DragValue::new(&mut s).range(0..=59)).changed();

                                            if changed_dur {
                                                *target_duration_sec = h * 3600 + m * 60 + s;
                                                any_changed = true;
                                            }
                                        });
                                        ui.end_row();

                                        ui.label("Time-Up Text:");
                                        any_changed |= ui
                                            .add_sized(
                                                Vec2::new(PROP_FIELD_W, PROP_FIELD_H),
                                                standard_singleline_text(finish_text, "TIME'S UP! / FINISHED"),
                                            )
                                            .changed();
                                        ui.end_row();
                                    }

                                    ui.label("Visual Effect Style:");
                                    egui::ComboBox::from_id_salt("prop_time_effect")
                                        .selected_text(effect.label())
                                        .width(PROP_FIELD_W)
                                        .show_ui(ui, |ui| {
                                            for eff in TimeVisualEffect::all() {
                                                any_changed |= ui.selectable_value(effect, *eff, eff.label()).changed();
                                            }
                                        });
                                    ui.end_row();

                                    any_changed |= ui
                                        .checkbox(show_badge, "Badge Text (Label):")
                                        .on_hover_text("Check to enable corner badge text (label)")
                                        .changed();
                                    ui.add_enabled_ui(*show_badge, |ui| {
                                        any_changed |= ui
                                            .add_sized(
                                                Vec2::new(PROP_FIELD_W, PROP_FIELD_H),
                                                standard_singleline_text(badge_text, "LIVE / CLOCK / COUNTDOWN / STOPWATCH"),
                                            )
                                            .changed();
                                    });
                                    ui.end_row();

                                    if *show_badge {
                                        ui.label("Badge Font Size:");
                                        ui.horizontal(|ui| {
                                            any_changed |= ui.add(egui::Slider::new(badge_font_size, 8.0..=80.0).suffix(" px")).changed();
                                            if ui.small_button("Reset").on_hover_text("Reset badge font size to default 14px").clicked() {
                                                *badge_font_size = 14.0;
                                                any_changed = true;
                                            }
                                        });
                                        ui.end_row();

                                        ui.label("Badge Position / Anchor:");
                                        egui::ComboBox::from_id_salt("prop_time_badge_pos")
                                            .selected_text(badge_position.label())
                                            .width(PROP_FIELD_W)
                                            .show_ui(ui, |ui| {
                                                for pos in crate::studio::source::TimeBadgePosition::all() {
                                                    any_changed |= ui.selectable_value(badge_position, *pos, pos.label()).changed();
                                                }
                                            });
                                        ui.end_row();

                                        ui.label("Offset X (Horizontal):");
                                        ui.horizontal(|ui| {
                                            any_changed |= ui.add(egui::Slider::new(badge_offset_x, -400.0..=400.0).suffix(" px")).changed();
                                            if ui.small_button("0").on_hover_text("Reset X offset to 0").clicked() {
                                                *badge_offset_x = 0.0;
                                                any_changed = true;
                                            }
                                        });
                                        ui.end_row();

                                        ui.label("Offset Y (Vertical):");
                                        ui.horizontal(|ui| {
                                            any_changed |= ui.add(egui::Slider::new(badge_offset_y, -250.0..=350.0).suffix(" px")).changed();
                                            if ui.small_button("0").on_hover_text("Reset Y offset to 0").clicked() {
                                                *badge_offset_y = 0.0;
                                                any_changed = true;
                                            }
                                        });
                                        ui.end_row();
                                    }

                                    ui.label("Font Family:");
                                    egui::ComboBox::from_id_salt("prop_time_font_family")
                                        .selected_text(font_family.label())
                                        .width(PROP_FIELD_W)
                                        .show_ui(ui, |ui| {
                                            for ff in TimeFontFamily::all() {
                                                any_changed |= ui.selectable_value(font_family, *ff, ff.label()).changed();
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Font Size & Style:");
                                    ui.horizontal(|ui| {
                                        any_changed |= ui.add(egui::Slider::new(font_size, 16.0..=140.0).suffix(" px")).changed();
                                        any_changed |= ui.checkbox(bold_font, "Bold").changed();
                                    });
                                    ui.end_row();

                                    ui.label("Time Format:");
                                    ui.horizontal(|ui| {
                                        any_changed |= ui.checkbox(show_hours, "Show Hours").changed();
                                        if *mode == TimeSourceMode::RealTimeClock {
                                            any_changed |= ui.checkbox(use_24h_format, "24-Hour").changed();
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Text Color:");
                                    ui.horizontal(|ui| {
                                        any_changed |= ui.color_edit_button_srgba_unmultiplied(text_color_rgba).changed();
                                        let [r, g, b, _] = *text_color_rgba;
                                        ui.label(format!("#{:02X}{:02X}{:02X}", r, g, b));
                                    });
                                    ui.end_row();

                                    ui.label("Background Color:");
                                    ui.horizontal(|ui| {
                                        any_changed |= ui.color_edit_button_srgba_unmultiplied(bg_color_rgba).changed();
                                        let [r, g, b, a] = *bg_color_rgba;
                                        ui.label(format!("#{:02X}{:02X}{:02X} (Alpha: {})", r, g, b, a));
                                    });
                                    ui.end_row();

                                    ui.label("Accent / Border Color:");
                                    ui.horizontal(|ui| {
                                        any_changed |= ui.color_edit_button_srgba_unmultiplied(accent_color_rgba).changed();
                                        let [r, g, b, _] = *accent_color_rgba;
                                        ui.label(format!("#{:02X}{:02X}{:02X}", r, g, b));
                                    });
                                    ui.end_row();

                                    ui.label("Border Width:");
                                    ui.horizontal(|ui| {
                                        any_changed |= ui.add(egui::Slider::new(border_thickness, 0.0..=12.0).suffix(" px")).changed();
                                        if *border_thickness <= 0.0 {
                                            ui.label(RichText::new("(No Border)").small().color(p.text_muted));
                                        }
                                    });
                                    ui.end_row();
                                }
                            }
                        });

                    if let Some(tpl) = pending_tv_template {
                        src.apply_tv_text_template(tpl);
                        any_changed = true;
                    }

                    ui.separator();
                    ui.label(RichText::new("Transform & Layout Preset").strong());
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(5.0, 4.0);
                        ui.spacing_mut().button_padding = egui::vec2(6.0, 3.0);
                        let preset_btn = |ui: &mut egui::Ui, label: &str| -> bool {
                            ui.add(
                                egui::Button::new(RichText::new(label).size(11.5))
                                    .min_size(Vec2::new(0.0, 22.0)),
                            )
                            .clicked()
                        };
                        if preset_btn(ui, "Fit to Screen") {
                            src.transform.fit_to_screen();
                            any_changed = true;
                        }
                        if preset_btn(ui, "Stretch") {
                            src.transform.stretch_to_screen();
                            any_changed = true;
                        }
                        if preset_btn(ui, "Center") {
                            src.transform.center_to_screen();
                            any_changed = true;
                        }
                        if preset_btn(ui, "Rotate 90° CW") {
                            src.transform.rotate_cw_90();
                            any_changed = true;
                        }
                        if preset_btn(ui, "Rotate 90° CCW") {
                            src.transform.rotate_ccw_90();
                            any_changed = true;
                        }
                        if preset_btn(ui, "Rotate 180°") {
                            src.transform.rotate_180();
                            any_changed = true;
                        }
                        if preset_btn(ui, "Flip H") {
                            src.transform.flip_horizontal();
                            any_changed = true;
                        }
                        if preset_btn(ui, "Flip V") {
                            src.transform.flip_vertical();
                            any_changed = true;
                        }
                        if preset_btn(ui, "PiP Bottom-Right") {
                            src.transform.preset_pip_bottom_right();
                            any_changed = true;
                        }
                        if preset_btn(ui, "PiP Bottom-Left") {
                            src.transform.preset_pip_bottom_left();
                            any_changed = true;
                        }
                        if preset_btn(ui, "PiP Top-Right") {
                            src.transform.preset_pip_top_right();
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
                        if src.transform.has_crop()
                            && ui
                                .add(
                                    egui::Button::new(RichText::new("Reset Crop").size(11.5))
                                        .min_size(Vec2::new(0.0, 22.0)),
                                )
                                .clicked()
                        {
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
                    ui.add_space(6.0);
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            let ok_btn = egui::Button::new(
                                RichText::new("OK").size(12.5),
                            )
                            .min_size(Vec2::new(76.0, 24.0));
                            if ui.add(ok_btn).clicked() {
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

        if open_slideshow_manager {
            app.show_slideshow_manager_dialog = true;
        }
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
            app.studio.sync_if_linked();
            app.save_all_state();
        }
        if !open || close_clicked {
            app.show_source_properties_dialog = false;
        }
    }

    // 4b. Manage Slide Show Images Modal Window (with medium thumbnails)
    if app.show_slideshow_manager_dialog {
        let p = ThemePalette::for_mode(app.settings.theme);
        let mut open = app.show_slideshow_manager_dialog;
        let mut done_clicked = false;
        let mut manager_changed = false;

        let current_files: Vec<String> = app
            .studio
            .preview_scene()
            .selected_source()
            .and_then(|s| match &s.kind {
                SourceKind::ImageSlideShow { files, .. } => Some(files.clone()),
                _ => None,
            })
            .unwrap_or_default();

        // Pre-fetch textures for thumbnails so we don't borrow `app.studio` and `app.capture_engine` simultaneously
        let thumb_textures: Vec<Option<egui::TextureHandle>> = current_files
            .iter()
            .map(|path_str| {
                let key = format!("IMG:{}", path_str);
                app.capture_engine.get_or_update_texture(ctx, &key)
            })
            .collect();

        egui::Window::new(format!("{} Manage Slide Show Images", icons::PHOTO))
            .id(egui::Id::new("nexora_slideshow_manager_modal_win"))
            .order(egui::Order::Foreground)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .auto_sized()
            .default_width(530.0)
            .max_width(530.0)
            .show(ctx, |ui| {
                block_window_body_drag(ui);
                ui.set_width(510.0);
                ui.set_max_width(510.0);

                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                    if let SourceKind::ImageSlideShow { files, .. } = &mut src.kind {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{} photo(s) in Slide Show", files.len()))
                                    .strong()
                                    .color(p.text_primary),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if !files.is_empty()
                                        && ui
                                            .add(
                                                egui::Button::new(format!(
                                                    "{} Clear All",
                                                    icons::TRASH
                                                ))
                                                .min_size(Vec2::new(88.0, 26.0)),
                                            )
                                            .on_hover_text("Remove all photos from slide show")
                                            .clicked()
                                    {
                                        files.clear();
                                        manager_changed = true;
                                    }
                                    if ui
                                        .add(
                                            egui::Button::new(format!(
                                                "{} Add Images (Multi-Select)...",
                                                icons::PLUS
                                            ))
                                            .min_size(Vec2::new(205.0, 26.0)),
                                        )
                                        .on_hover_text(
                                            "Hold Ctrl or Shift in Explorer to select multiple photos at once",
                                        )
                                        .clicked()
                                    {
                                        let picked_list =
                                            crate::platform::pick_multiple_image_files_dialog();
                                        if !picked_list.is_empty() {
                                            for path in picked_list {
                                                let s = path.to_string_lossy().to_string();
                                                if !s.trim().is_empty() {
                                                    files.push(s);
                                                }
                                            }
                                            manager_changed = true;
                                        }
                                    }
                                },
                            );
                        });

                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        if files.is_empty() {
                            egui::Frame::new()
                                .fill(p.bg_elevated)
                                .corner_radius(egui::CornerRadius::same(6))
                                .stroke(Stroke::new(1.0_f32, p.border_subtle))
                                .inner_margin(egui::Margin::symmetric(16, 28))
                                .show(ui, |ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.label(
                                            RichText::new(format!(
                                                "{} No photos added yet",
                                                icons::PHOTO
                                            ))
                                            .strong()
                                            .size(13.0)
                                            .color(p.text_secondary),
                                        );
                                        ui.add_space(4.0);
                                        ui.label(
                                            RichText::new(
                                                "Click '+ Add Images (Multi-Select)...' above and use Ctrl+Click to pick multiple images.",
                                            )
                                            .small()
                                            .color(p.text_muted),
                                        );
                                    });
                                });
                        } else {
                            let mut remove_idx = None;
                            let mut swap_pair = None;
                            let total_len = files.len();

                            egui::ScrollArea::vertical()
                                .id_salt("slideshow_manager_cards_scroll")
                                .max_height(330.0)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    for (i, path_str) in files.iter().enumerate() {
                                        let path_obj = std::path::Path::new(path_str);
                                        let fname = path_obj
                                            .file_name()
                                            .and_then(|s| s.to_str())
                                            .unwrap_or(path_str.as_str());
                                        let parent_dir = path_obj
                                            .parent()
                                            .and_then(|p| p.to_str())
                                            .unwrap_or("");

                                        egui::Frame::new()
                                            .fill(p.bg_elevated)
                                            .corner_radius(egui::CornerRadius::same(6))
                                            .stroke(Stroke::new(1.0_f32, p.border_subtle))
                                            .inner_margin(egui::Margin::symmetric(10, 6))
                                            .show(ui, |ui| {
                                                ui.set_width(476.0);
                                                ui.horizontal(|ui| {
                                                    // 1. Slide index badge
                                                    ui.label(
                                                        RichText::new(format!("#{:02}", i + 1))
                                                            .monospace()
                                                            .strong()
                                                            .size(12.0)
                                                            .color(p.text_secondary),
                                                    );
                                                    ui.add_space(4.0);

                                                    // 2. Medium Thumbnail (84x48 px, 16:9 box)
                                                    let thumb_size = Vec2::new(84.0, 48.0);
                                                    let (thumb_rect, _) = ui.allocate_exact_size(
                                                        thumb_size,
                                                        egui::Sense::hover(),
                                                    );
                                                    ui.painter().rect_filled(
                                                        thumb_rect,
                                                        egui::CornerRadius::same(4),
                                                        egui::Color32::from_rgb(18, 22, 29),
                                                    );
                                                    if let Some(Some(tex)) = thumb_textures.get(i)
                                                    {
                                                        let tsize = tex.size_vec2();
                                                        let fit_rect = if tsize.x > 0.0
                                                            && tsize.y > 0.0
                                                        {
                                                            let scale = (thumb_rect.width()
                                                                / tsize.x)
                                                                .min(thumb_rect.height() / tsize.y);
                                                            let draw_w = tsize.x * scale;
                                                            let draw_h = tsize.y * scale;
                                                            egui::Rect::from_center_size(
                                                                thumb_rect.center(),
                                                                Vec2::new(draw_w, draw_h),
                                                            )
                                                        } else {
                                                            thumb_rect
                                                        };
                                                        let uv = egui::Rect::from_min_max(
                                                            egui::Pos2::ZERO,
                                                            egui::Pos2::new(1.0, 1.0),
                                                        );
                                                        ui.painter().image(
                                                            tex.id(),
                                                            fit_rect,
                                                            uv,
                                                            egui::Color32::WHITE,
                                                        );
                                                    } else {
                                                        ui.painter().text(
                                                            thumb_rect.center(),
                                                            egui::Align2::CENTER_CENTER,
                                                            icons::PHOTO,
                                                            egui::FontId::proportional(18.0),
                                                            p.text_muted,
                                                        );
                                                    }
                                                    ui.painter().rect_stroke(
                                                        thumb_rect,
                                                        egui::CornerRadius::same(4),
                                                        Stroke::new(1.0_f32, p.border_subtle),
                                                        egui::StrokeKind::Inside,
                                                    );

                                                    ui.add_space(6.0);

                                                    // 3. Filename & folder info
                                                    ui.vertical(|ui| {
                                                        ui.set_max_width(220.0);
                                                        ui.add_space(4.0);
                                                        ui.add(
                                                            egui::Label::new(
                                                                RichText::new(fname)
                                                                    .strong()
                                                                    .size(12.0)
                                                                    .color(p.text_primary),
                                                            )
                                                            .truncate(),
                                                        )
                                                        .on_hover_text(path_str);
                                                        if !parent_dir.is_empty() {
                                                            ui.add(
                                                                egui::Label::new(
                                                                    RichText::new(parent_dir)
                                                                        .size(10.0)
                                                                        .color(p.text_muted),
                                                                )
                                                                .truncate(),
                                                            )
                                                            .on_hover_text(path_str);
                                                        }
                                                    });

                                                    // 4. Reorder & Delete action buttons (well-spaced)
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            if ui
                                                                .add(
                                                                    egui::Button::new(
                                                                        RichText::new(icons::TRASH)
                                                                            .size(13.0),
                                                                    )
                                                                    .min_size(Vec2::new(
                                                                        30.0, 26.0,
                                                                    )),
                                                                )
                                                                .on_hover_text("Delete photo")
                                                                .clicked()
                                                            {
                                                                remove_idx = Some(i);
                                                            }
                                                            let can_down = i + 1 < total_len;
                                                            if ui
                                                                .add_enabled(
                                                                    can_down,
                                                                    egui::Button::new(
                                                                        RichText::new(
                                                                            icons::ARROW_DOWN,
                                                                        )
                                                                        .size(13.0),
                                                                    )
                                                                    .min_size(Vec2::new(
                                                                        30.0, 26.0,
                                                                    )),
                                                                )
                                                                .on_hover_text("Move photo down")
                                                                .clicked()
                                                            {
                                                                swap_pair = Some((i, i + 1));
                                                            }
                                                            let can_up = i > 0;
                                                            if ui
                                                                .add_enabled(
                                                                    can_up,
                                                                    egui::Button::new(
                                                                        RichText::new(
                                                                            icons::ARROW_UP,
                                                                        )
                                                                        .size(13.0),
                                                                    )
                                                                    .min_size(Vec2::new(
                                                                        30.0, 26.0,
                                                                    )),
                                                                )
                                                                .on_hover_text("Move photo up")
                                                                .clicked()
                                                            {
                                                                swap_pair = Some((i, i - 1));
                                                            }
                                                        },
                                                    );
                                                });
                                            });
                                        ui.add_space(4.0);
                                    }
                                });

                            if let Some((a, b)) = swap_pair {
                                files.swap(a, b);
                                manager_changed = true;
                            }
                            if let Some(idx) = remove_idx {
                                files.remove(idx);
                                manager_changed = true;
                            }
                        }

                        ui.add_space(4.0);
                        ui.separator();
                        ui.allocate_ui_with_layout(
                            Vec2::new(500.0, 28.0),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui
                                    .add(egui::Button::new("Done").min_size(Vec2::new(78.0, 26.0)))
                                    .clicked()
                                {
                                    done_clicked = true;
                                }
                            },
                        );
                    } else {
                        ui.label("Selected source is not an Image Slide Show.");
                        if ui.button("Close").clicked() {
                            done_clicked = true;
                        }
                    }
                } else {
                    ui.label("No Image Slide Show source selected.");
                    if ui.button("Close").clicked() {
                        done_clicked = true;
                    }
                }
            });

        if manager_changed {
            app.studio.sync_if_linked();
            app.save_all_state();
        }
        if !open || done_clicked {
            app.show_slideshow_manager_dialog = false;
        }
    }

    // 4c. Video Filters Modal Window (Chroma Key, Color Key, Luma Key)
    if app.show_source_filters_dialog {
        let p = ThemePalette::for_mode(app.settings.theme);
        let mut open = app.show_source_filters_dialog;
        let mut done_clicked = false;
        let mut filters_changed = false;

        let src_name = app
            .studio
            .preview_scene()
            .selected_source()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "Source".to_string());

        egui::Window::new(format!("{} Filters for '{}'", icons::FILTER, src_name))
            .id(egui::Id::new("nexora_source_filters_modal_win"))
            .order(egui::Order::Foreground)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .auto_sized()
            .default_width(510.0)
            .max_width(510.0)
            .show(ctx, |ui| {
                block_window_body_drag(ui);
                const FLT_FIELD_W: f32 = 270.0;
                ui.set_width(490.0);
                ui.set_max_width(490.0);

                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Keying / Transparency Filter:").strong().color(p.text_primary));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            filters_changed |= ui.checkbox(&mut src.filter.enabled, "Enable Filter").changed();
                        });
                    });

                    ui.add_space(4.0);

                    // Filter selector buttons row
                    ui.horizontal_wrapped(|ui| {
                        for fk in crate::studio::source::KeyFilterKind::all() {
                            let is_active = src.filter.kind == *fk;
                            let btn_text = RichText::new(fk.short_name()).size(12.0);
                            let resp = ui
                                .add(
                                    egui::Button::new(btn_text)
                                        .min_size(Vec2::new(108.0, 26.0))
                                        .fill(if is_active { p.accent_preview } else { p.bg_elevated }),
                                )
                                .on_hover_text(fk.hover_explanation());
                            if resp.clicked() {
                                src.filter.kind = *fk;
                                if *fk != crate::studio::source::KeyFilterKind::None {
                                    src.filter.enabled = true;
                                }
                                filters_changed = true;
                            }
                        }
                    });

                    ui.add_space(6.0);

                    // Explanatory Info Box
                    egui::Frame::new()
                        .fill(p.bg_elevated)
                        .corner_radius(egui::CornerRadius::same(5))
                        .stroke(Stroke::new(1.0_f32, p.border_subtle))
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(src.filter.kind.hover_explanation())
                                    .small()
                                    .color(p.text_secondary),
                            );
                        });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(4.0);

                    match src.filter.kind {
                        crate::studio::source::KeyFilterKind::None => {
                            ui.vertical_centered(|ui| {
                                ui.add_space(12.0);
                                ui.label(
                                    RichText::new(format!("{} Original Source (No Transparency Filter)", icons::PHOTO))
                                        .size(13.0)
                                        .color(p.text_muted),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("Select 'Chroma Key', 'Color Key', or 'Luma Key' above to remove background.")
                                        .small()
                                        .color(p.text_muted),
                                );
                                ui.add_space(12.0);
                            });
                        }
                        crate::studio::source::KeyFilterKind::ChromaKey => {
                            egui::Grid::new("flt_chroma_grid")
                                .num_columns(2)
                                .spacing([12.0, 8.0])
                                .show(ui, |ui| {
                                    ui.label("Key Color Type:").on_hover_text("Select green or blue screen color to remove");
                                    egui::ComboBox::from_id_salt("flt_chroma_preset_combo")
                                        .selected_text(src.filter.key_color_preset.label())
                                        .width(FLT_FIELD_W)
                                        .truncate()
                                        .show_ui(ui, |ui| {
                                            for preset in crate::studio::source::ChromaKeyPresetColor::all() {
                                                if ui
                                                    .selectable_label(src.filter.key_color_preset == *preset, preset.label())
                                                    .clicked()
                                                {
                                                    src.filter.key_color_preset = *preset;
                                                    if *preset != crate::studio::source::ChromaKeyPresetColor::Custom {
                                                        src.filter.key_color_rgb = preset.rgb();
                                                    }
                                                    filters_changed = true;
                                                }
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Key Color:").on_hover_text("Target color to remove as transparent");
                                    ui.horizontal(|ui| {
                                        let mut col = egui::Color32::from_rgb(
                                            src.filter.key_color_rgb[0],
                                            src.filter.key_color_rgb[1],
                                            src.filter.key_color_rgb[2],
                                        );
                                        if ui.color_edit_button_srgba(&mut col).changed() {
                                            src.filter.key_color_rgb = [col.r(), col.g(), col.b()];
                                            src.filter.key_color_preset = crate::studio::source::ChromaKeyPresetColor::Custom;
                                            filters_changed = true;
                                        }
                                        ui.label(format!("#{:02X}{:02X}{:02X}", col.r(), col.g(), col.b())).on_hover_text("Hex Code RGB");

                                        let picker_btn = ui.button(format!("{} Screen Eyedropper", icons::COLOR_PICKER))
                                            .on_hover_text("Screen Eyedropper: Click then hover and click anywhere on canvas or screen to sample a color");
                                        if picker_btn.clicked() {
                                            app.eyedropper_active = true;
                                            app.eyedropper_armed = false;
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Similarity:").on_hover_text("Color similarity threshold. Higher values remove a wider range of key color.");
                                    filters_changed |= ui.add(egui::Slider::new(&mut src.filter.similarity, 1..=1000)).changed();
                                    ui.end_row();

                                    ui.label("Smoothness:").on_hover_text("Kehalusan tepian objek agar tidak bergerigi atau terpotong kasar.");
                                    filters_changed |= ui.add(egui::Slider::new(&mut src.filter.smoothness, 1..=1000)).changed();
                                    ui.end_row();

                                    ui.label("Spill Reduction:").on_hover_text("Cleans up green/blue reflections on hair and edges.");
                                    filters_changed |= ui.add(egui::Slider::new(&mut src.filter.spill_reduction, 0..=1000)).changed();
                                    ui.end_row();

                                    ui.label("Opacity:").on_hover_text("Overall layer transparency level");
                                    filters_changed |= ui.add(egui::Slider::new(&mut src.filter.opacity, 0.0..=1.0).fixed_decimals(2)).changed();
                                    ui.end_row();
                                });
                        }
                        crate::studio::source::KeyFilterKind::ColorKey => {
                            egui::Grid::new("flt_color_grid")
                                .num_columns(2)
                                .spacing([12.0, 8.0])
                                .show(ui, |ui| {
                                    ui.label("Key Color:").on_hover_text("Select a specific solid color to remove from the image or video");
                                    ui.horizontal(|ui| {
                                        let mut col = egui::Color32::from_rgb(
                                            src.filter.key_color_rgb[0],
                                            src.filter.key_color_rgb[1],
                                            src.filter.key_color_rgb[2],
                                        );
                                        if ui.color_edit_button_srgba(&mut col).changed() {
                                            src.filter.key_color_rgb = [col.r(), col.g(), col.b()];
                                            filters_changed = true;
                                        }
                                        ui.label(format!("#{:02X}{:02X}{:02X}", col.r(), col.g(), col.b()));

                                        let picker_btn = ui.button(format!("{} Screen Eyedropper", icons::COLOR_PICKER))
                                            .on_hover_text("Screen Eyedropper: Click then hover and click anywhere on canvas or screen to sample a color");
                                        if picker_btn.clicked() {
                                            app.eyedropper_active = true;
                                            app.eyedropper_armed = false;
                                        }

                                        // Quick Presets
                                        if ui.small_button("Black").on_hover_text("Set to Black").clicked() {
                                            src.filter.key_color_rgb = [0, 0, 0];
                                            filters_changed = true;
                                        }
                                        if ui.small_button("White").on_hover_text("Set to White").clicked() {
                                            src.filter.key_color_rgb = [255, 255, 255];
                                            filters_changed = true;
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Similarity:").on_hover_text("Target color similarity tolerance");
                                    filters_changed |= ui.add(egui::Slider::new(&mut src.filter.similarity, 1..=1000)).changed();
                                    ui.end_row();

                                    ui.label("Smoothness:").on_hover_text("Edge transparency falloff smoothness");
                                    filters_changed |= ui.add(egui::Slider::new(&mut src.filter.smoothness, 1..=1000)).changed();
                                    ui.end_row();

                                    ui.label("Opacity:").on_hover_text("Overall layer transparency level");
                                    filters_changed |= ui.add(egui::Slider::new(&mut src.filter.opacity, 0.0..=1.0).fixed_decimals(2)).changed();
                                    ui.end_row();
                                });
                        }
                        crate::studio::source::KeyFilterKind::LumaKey => {
                            egui::Grid::new("flt_luma_grid")
                                .num_columns(2)
                                .spacing([12.0, 8.0])
                                .show(ui, |ui| {
                                    ui.label("Quick Presets:").on_hover_text("Select dark or bright background removal preset");
                                    ui.horizontal(|ui| {
                                        if ui.add(egui::Button::new("Remove Black (Fire/Particles)").min_size(Vec2::new(170.0, 22.0))).clicked() {
                                            src.filter.luma_min = 0.08;
                                            src.filter.luma_min_smooth = 0.06;
                                            src.filter.luma_max = 1.00;
                                            src.filter.luma_max_smooth = 0.04;
                                            filters_changed = true;
                                        }
                                        if ui.add(egui::Button::new("Remove White (White Logo)").min_size(Vec2::new(160.0, 22.0))).clicked() {
                                            src.filter.luma_min = 0.00;
                                            src.filter.luma_min_smooth = 0.02;
                                            src.filter.luma_max = 0.88;
                                            src.filter.luma_max_smooth = 0.06;
                                            filters_changed = true;
                                        }
                                    });
                                    ui.end_row();

                                    ui.label("Luma Min (Cut Black):").on_hover_text("Removes dark areas below this brightness threshold");
                                    filters_changed |= ui.add(egui::Slider::new(&mut src.filter.luma_min, 0.0..=1.0).fixed_decimals(2)).changed();
                                    ui.end_row();

                                    ui.label("Min Smoothness:").on_hover_text("Smoothness of removed dark edges");
                                    filters_changed |= ui.add(egui::Slider::new(&mut src.filter.luma_min_smooth, 0.0..=0.5).fixed_decimals(2)).changed();
                                    ui.end_row();

                                    ui.label("Luma Max (Cut White):").on_hover_text("Removes bright areas above this brightness threshold (lower to remove white background)");
                                    filters_changed |= ui.add(egui::Slider::new(&mut src.filter.luma_max, 0.0..=1.0).fixed_decimals(2)).changed();
                                    ui.end_row();

                                    ui.label("Max Smoothness:").on_hover_text("Smoothness of removed bright edges");
                                    filters_changed |= ui.add(egui::Slider::new(&mut src.filter.luma_max_smooth, 0.0..=0.5).fixed_decimals(2)).changed();
                                    ui.end_row();

                                    ui.label("Opacity:").on_hover_text("Overall layer transparency level");
                                    filters_changed |= ui.add(egui::Slider::new(&mut src.filter.opacity, 0.0..=1.0).fixed_decimals(2)).changed();
                                    ui.end_row();
                                });
                        }
                    }

                    ui.add_space(6.0);
                    ui.separator();
                    ui.allocate_ui_with_layout(
                        Vec2::new(480.0, 26.0),
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if ui.add(egui::Button::new("Done").min_size(Vec2::new(76.0, 24.0))).clicked() {
                                done_clicked = true;
                            }
                            if ui.add(egui::Button::new(format!("{} Reset Default", icons::REFRESH)).min_size(Vec2::new(115.0, 24.0))).clicked() {
                                let k = src.filter.kind;
                                src.filter = crate::studio::source::SourceFilterConfig::default();
                                src.filter.kind = k;
                                filters_changed = true;
                            }
                        },
                    );
                } else {
                    ui.label("No source is currently selected.");
                    if ui.button("Close").clicked() {
                        done_clicked = true;
                    }
                }
            });

        if filters_changed {
            app.studio.sync_if_linked();
            app.save_all_state();
        }
        if !open || done_clicked {
            app.show_source_filters_dialog = false;
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
        .id(egui::Id::new("nexora_save_preset_modal_win"))
        .order(egui::Order::Foreground)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .auto_sized()
        .default_width(360.0)
        .show(ctx, |ui| {
            block_window_body_drag(ui);
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

    // 8. OBS-Style Modern Add Source Modal Dialog
    render_add_source_dialog(ctx, app);

    // 9. OBS-Style Virtual Camera Configuration Modal Dialog
    render_virtual_camera_dialog(ctx, app);

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

                    let time_sec = ui.input(|i| i.time);
                    // Draw from bottom of Source list (back layer) to top of Source list (front layer)
                    for src in scene_to_render.sources.iter().rev() {
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

                        crate::ui::workspace::render_source_item_on_canvas(
                            &proj_painter,
                            &egui_ctx,
                            app,
                            src,
                            layer_rect,
                            time_sec,
                            1.0,
                        );
                    }

                    // Presenter pointer click navigation on projector screen
                    if response.clicked() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            let rel_x = (pos.x - rect.min.x) / rect.width().max(1.0);
                            if rel_x < 0.20 {
                                app.advance_presentation_slide(-1);
                            } else {
                                app.advance_presentation_slide(1);
                            }
                        }
                    }

                    // Keyboard slide navigation on projector screen (Wireless Presenter Remote)
                    let proj_next = vp_ctx.input(|i| {
                        !i.modifiers.ctrl && !i.modifiers.alt && (
                            i.key_pressed(egui::Key::ArrowRight)
                                || i.key_pressed(egui::Key::PageDown)
                                || i.key_pressed(egui::Key::Space)
                                || i.key_pressed(egui::Key::Enter)
                        )
                    });
                    let proj_prev = vp_ctx.input(|i| {
                        !i.modifiers.ctrl && !i.modifiers.alt && (
                            i.key_pressed(egui::Key::ArrowLeft)
                                || i.key_pressed(egui::Key::PageUp)
                                || i.key_pressed(egui::Key::Backspace)
                        )
                    });
                    if proj_next {
                        app.advance_presentation_slide(1);
                    } else if proj_prev {
                        app.advance_presentation_slide(-1);
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

pub fn render_add_source_dialog(ctx: &egui::Context, app: &mut NexoraApp) {
    if !app.show_add_source_dialog {
        return;
    }

    let mut open = app.show_add_source_dialog;
    let p = ThemePalette::for_mode(app.settings.theme);

    egui::Window::new(format!("{} Add Source", icons::PLUS))
        .id(egui::Id::new("nexora_add_source_modal_win"))
        .order(egui::Order::Foreground)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .auto_sized()
        .default_width(650.0)
        .max_width(650.0)
        .show(ctx, |ui| {
            block_window_body_drag(ui);
            ui.set_width(630.0);
            ui.set_max_width(630.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Select source type and configure the layer to add to scene:")
                        .small()
                        .color(p.text_secondary),
                );
            });
            ui.add_space(6.0);

            let cur_selected = app.add_source_selected_type;
            let mut newly_selected = None;

            ui.horizontal(|ui| {
                // Left Column: Source Selection List
                ui.allocate_ui_with_layout(
                    Vec2::new(260.0, 370.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(260.0);
                        ui.label(
                            RichText::new("SOURCE TYPES (LAYER TYPE)")
                                .small()
                                .strong()
                                .color(p.text_muted),
                        );
                        ui.add_space(4.0);

                        egui::ScrollArea::vertical()
                            .id_salt("add_source_types_scroll")
                            .max_height(340.0)
                            .show(ui, |ui| {
                                for st in crate::studio::source::AddSourceType::all() {
                                    let is_active = cur_selected == *st;
                                    let (badge, is_ready) = st.status_tag();

                                    let (rect, resp) = ui.allocate_exact_size(
                                        Vec2::new(ui.available_width() - 4.0, 44.0),
                                        egui::Sense::click(),
                                    );

                                    let bg = if is_active {
                                        p.accent_primary.gamma_multiply(0.20)
                                    } else if resp.hovered() {
                                        p.bg_elevated
                                    } else {
                                        p.bg_panel
                                    };
                                    let stroke = if is_active {
                                        Stroke::new(1.5_f32, p.accent_primary)
                                    } else {
                                        Stroke::new(1.0_f32, p.border_subtle)
                                    };

                                    ui.painter().rect(rect, 4.0, bg, stroke, egui::StrokeKind::Inside);

                                    // Icon in left column
                                    ui.painter().text(
                                        Pos2::new(rect.min.x + 10.0, rect.min.y + 22.0),
                                        egui::Align2::LEFT_CENTER,
                                        st.icon(),
                                        egui::FontId::proportional(16.0),
                                        if is_active { p.accent_primary } else { p.text_primary },
                                    );

                                    // Title
                                    ui.painter().text(
                                        Pos2::new(rect.min.x + 36.0, rect.min.y + 14.0),
                                        egui::Align2::LEFT_CENTER,
                                        st.title(),
                                        egui::FontId::proportional(12.0),
                                        p.text_primary,
                                    );

                                    // Status Badge on the right
                                    let badge_color = if is_ready { p.meter_ok } else { p.pause_amber };
                                    ui.painter().text(
                                        Pos2::new(rect.max.x - 8.0, rect.min.y + 14.0),
                                        egui::Align2::RIGHT_CENTER,
                                        badge,
                                        egui::FontId::proportional(9.5),
                                        badge_color,
                                    );

                                    // Brief single-line summary
                                    let desc_short: String = st.description().chars().take(34).collect();
                                    ui.painter().text(
                                        Pos2::new(rect.min.x + 36.0, rect.min.y + 29.0),
                                        egui::Align2::LEFT_CENTER,
                                        format!("{}...", desc_short),
                                        egui::FontId::proportional(10.0),
                                        p.text_muted,
                                    );

                                    if resp.clicked() {
                                        newly_selected = Some(*st);
                                    }
                                }
                            });
                    },
                );

                ui.separator();

                // Right Column: Configuration & Properties for Selected Type
                ui.allocate_ui_with_layout(
                    Vec2::new(350.0, 370.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(350.0);
                        ui.label(
                            RichText::new("SOURCE SETTINGS")
                                .small()
                                .strong()
                                .color(p.text_muted),
                        );
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label(RichText::new(cur_selected.icon()).size(20.0).color(p.accent_primary));
                            ui.heading(cur_selected.title());
                        });

                        ui.label(RichText::new(cur_selected.description()).small().color(p.text_secondary));
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // Source Name Input
                        ui.label(RichText::new("Source Name:").strong().size(12.0));
                        ui.add_sized(
                            Vec2::new(340.0, 24.0),
                            standard_singleline_text(&mut app.add_source_name_buffer, "Source name..."),
                        );
                        ui.add_space(6.0);

                        // Contextual settings per source type
                        match cur_selected {
                            crate::studio::source::AddSourceType::DisplayCapture => {
                                ui.label("Select Display Monitor:");
                                let displays = app.capture_catalog.displays.clone();
                                if displays.is_empty() {
                                    ui.label(RichText::new("No monitor detected. Using Primary Display.").small().color(p.text_muted));
                                } else {
                                    let selected_id = if app.add_source_file_path.is_empty() {
                                        displays.first().map(|d| d.id.clone()).unwrap_or_default()
                                    } else {
                                        app.add_source_file_path.clone()
                                    };
                                    let current_name = displays.iter().find(|d| d.id == selected_id).map(|d| d.name.as_str()).unwrap_or("Select Monitor");
                                    egui::ComboBox::from_id_salt("add_disp_combo")
                                        .selected_text(current_name)
                                        .width(330.0)
                                        .show_ui(ui, |ui| {
                                            for d in &displays {
                                                if ui.selectable_value(&mut app.add_source_file_path, d.id.clone(), format!("{} ({}x{})", d.name, d.width, d.height)).clicked() {
                                                    // updated
                                                }
                                            }
                                        });
                                }
                            }
                            crate::studio::source::AddSourceType::WindowCapture => {
                                ui.label("Select Application Window:");
                                let windows = app.capture_catalog.windows.clone();
                                if windows.is_empty() {
                                    ui.label(RichText::new("No open application windows detected.").small().color(p.text_muted));
                                } else {
                                    let selected_hwnd: u64 = app.add_source_file_path.parse().unwrap_or(0);
                                    let current_title = windows.iter().find(|w| w.hwnd == selected_hwnd).map(|w| w.title.as_str()).unwrap_or_else(|| {
                                        windows.first().map(|w| w.title.as_str()).unwrap_or("Select Window")
                                    });
                                    egui::ComboBox::from_id_salt("add_win_combo")
                                        .selected_text(current_title.chars().take(40).collect::<String>())
                                        .width(330.0)
                                        .show_ui(ui, |ui| {
                                            for w in &windows {
                                                let mut cur_val = selected_hwnd;
                                                if ui.selectable_value(&mut cur_val, w.hwnd, format!("[{}] {}", w.executable, w.title)).clicked() {
                                                    app.add_source_file_path = w.hwnd.to_string();
                                                }
                                            }
                                        });
                                }
                            }
                            crate::studio::source::AddSourceType::Image => {
                                ui.label("Select Image File:");
                                ui.horizontal(|ui| {
                                    ui.add_sized(Vec2::new(225.0, 24.0), standard_singleline_text(&mut app.add_source_file_path, "Image file path..."));
                                    if ui.button(format!("{} Browse...", icons::FOLDER_OPEN)).clicked() {
                                        if let Some(path) = crate::platform::pick_image_file_dialog() {
                                            app.add_source_file_path = path.to_string_lossy().to_string();
                                            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                                app.add_source_name_buffer = stem.to_string();
                                            }
                                            app.add_source_error_message = None;
                                        }
                                    }
                                });
                                ui.label(RichText::new("Supported formats: .png, .jpg, .jpeg, .bmp, .webp, .gif").small().color(p.text_muted));
                            }
                            crate::studio::source::AddSourceType::MediaSource => {
                                ui.label("Select Video or Audio File:");
                                ui.horizontal(|ui| {
                                    ui.add_sized(Vec2::new(225.0, 24.0), standard_singleline_text(&mut app.add_source_file_path, "Video/audio file path..."));
                                    if ui.button(format!("{} Browse...", icons::FOLDER_OPEN)).clicked() {
                                        if let Some(path) = crate::platform::pick_media_file_dialog() {
                                            app.add_source_file_path = path.to_string_lossy().to_string();
                                            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                                app.add_source_name_buffer = stem.to_string();
                                            }
                                            app.add_source_error_message = None;
                                        }
                                    }
                                });
                                ui.label(RichText::new("Supported formats: .mp4, .mkv, .mov, .avi, .webm, .mp3, .wav").small().color(p.text_muted));
                            }
                            crate::studio::source::AddSourceType::TextGdiPlus => {
                                ui.label("Text Content:");
                                ui.add_sized(Vec2::new(340.0, 24.0), standard_singleline_text(&mut app.add_source_text_content, "Enter text to display..."));
                                ui.add_space(4.0);
                                ui.label(RichText::new("Scrolling marquee and TV show templates can be customized in Properties.").small().color(p.text_muted));
                            }
                            crate::studio::source::AddSourceType::ColorSource => {
                                ui.label("Solid Background Color:");
                                ui.horizontal(|ui| {
                                    ui.color_edit_button_srgba_unmultiplied(&mut app.add_source_color_rgba);
                                    let [r, g, b, _] = app.add_source_color_rgba;
                                    ui.label(RichText::new(format!("#{:02X}{:02X}{:02X}", r, g, b)).strong());
                                });
                                ui.add_space(4.0);
                                ui.label(RichText::new("Default canvas dimension 1920×1080 (16:9 fullscreen).").small().color(p.text_muted));
                            }
                            crate::studio::source::AddSourceType::PowerPoint => {
                                ui.label(RichText::new("Option 1: Capture PowerPoint Slide Show Window").strong().small());
                                let windows = app.capture_catalog.windows.clone();
                                let ppt_window = windows.iter().find(|w| w.title.to_lowercase().contains("powerpoint") || w.executable.to_lowercase().contains("powerpnt"));
                                if let Some(w) = ppt_window {
                                    ui.label(RichText::new(format!("{} Terdeteksi: [{}] {}", icons::CHECK, w.executable, w.title)).small().color(p.meter_ok));
                                    if app.add_source_file_path.is_empty() {
                                        app.add_source_file_path = w.hwnd.to_string();
                                    }
                                } else {
                                    ui.label(RichText::new("Start PowerPoint in Slide Show mode for interactive capture.").small().color(p.text_muted));
                                }

                                ui.add_space(4.0);
                                ui.label(RichText::new("Option 2: Or Select Presentation File (.pptx / .ppt):").strong().small());
                                ui.horizontal(|ui| {
                                    let mut display_p = if app.add_source_file_path.parse::<u64>().is_ok() { String::new() } else { app.add_source_file_path.clone() };
                                    if ui.add_sized(Vec2::new(225.0, 24.0), egui::TextEdit::singleline(&mut display_p)).changed() {
                                        app.add_source_file_path = display_p;
                                    }
                                    if ui.button(format!("{} Browse...", icons::FOLDER_OPEN)).clicked() {
                                        if let Some(path) = crate::platform::pick_powerpoint_file_dialog() {
                                            app.add_source_file_path = path.to_string_lossy().to_string();
                                            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                                app.add_source_name_buffer = format!("PPT - {}", stem);
                                            }
                                        }
                                    }
                                });
                            }
                            crate::studio::source::AddSourceType::Pdf => {
                                ui.label(RichText::new("PDF Document / Presentation File (.pdf):").strong().size(12.0));
                                ui.horizontal(|ui| {
                                    let btn_w = 84.0;
                                    let input_w = (340.0 - btn_w - ui.spacing().item_spacing.x).max(120.0);
                                    let mut display_p = app.add_source_file_path.clone();
                                    if ui.add_sized(Vec2::new(input_w, 24.0), standard_singleline_text(&mut display_p, "Select PDF file...")).changed() {
                                        app.add_source_file_path = display_p;
                                    }
                                    if ui.add(egui::Button::new(format!("{} Browse", icons::FOLDER_OPEN)).min_size(Vec2::new(btn_w, 24.0))).clicked() {
                                        if let Some(path) = crate::platform::pick_pdf_file_dialog() {
                                            app.add_source_file_path = path.to_string_lossy().to_string();
                                            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                                app.add_source_name_buffer = format!("PDF - {}", stem);
                                            }
                                        }
                                    }
                                });
                                ui.add_space(4.0);
                                ui.label(RichText::new("PDF is opened in 16:9 Fullscreen presentation mode with page navigation.").small().color(p.text_muted));
                            }
                            crate::studio::source::AddSourceType::BrowserSource => {
                                ui.label("Web URL:");
                                ui.add_sized(Vec2::new(340.0, 24.0), egui::TextEdit::singleline(&mut app.add_source_browser_url));
                                ui.add_space(4.0);
                                ui.label(RichText::new("Provides web URL and rendering dimensions for broadcast overlays.").small().color(p.text_muted));
                            }
                            crate::studio::source::AddSourceType::WebcamDevice => {
                                ui.label("Select Camera Device:");
                                let cameras = app.capture_catalog.cameras.clone();
                                if cameras.is_empty() {
                                    ui.label(RichText::new("No online webcams detected.").small().color(p.text_muted));
                                } else {
                                    let selected_id = if app.add_source_file_path.is_empty() {
                                        cameras.first().map(|c| c.id.clone()).unwrap_or_default()
                                    } else {
                                        app.add_source_file_path.clone()
                                    };
                                    let current_name = cameras.iter().find(|c| c.id == selected_id).map(|c| c.name.as_str()).unwrap_or("Select Camera");
                                    egui::ComboBox::from_id_salt("add_cam_combo")
                                        .selected_text(current_name)
                                        .width(330.0)
                                        .show_ui(ui, |ui| {
                                            for c in &cameras {
                                                if ui.selectable_value(&mut app.add_source_file_path, c.id.clone(), c.name.clone()).clicked() {
                                                    // updated
                                                }
                                            }
                                        });
                                }
                            }
                            crate::studio::source::AddSourceType::ImageSlideShow => {
                                ui.label("Image Slide Show");
                                ui.label(RichText::new("Image slide show created with fade transitions. Photos can be added and reordered via 'Manage Slide Show Images' in Properties.").small().color(p.text_muted));
                            }
                            crate::studio::source::AddSourceType::TimeSource => {
                                ui.label(RichText::new("Time Mode & Style Options:").strong().small());
                                ui.label(RichText::new("• Real-Time Clock (Synchronized with Windows system time)").small());
                                ui.label(RichText::new("• Stopwatch / Count-Up (Hours, minutes, seconds)").small());
                                ui.label(RichText::new("• Countdown Timer (Presentations, events, exams)").small());
                                ui.add_space(4.0);
                                ui.label(RichText::new("Available in 4 visual styles: Digital Neon, TV Broadcast Badge, Clean Minimalist, and Glassmorphism Card.").small().color(p.text_muted));
                                ui.label(RichText::new("Font family, scalable font size, colors, and advance/rewind controls can be adjusted in Properties.").small().color(p.text_muted));
                            }
                        }

                        ui.add_space(8.0);
                        ui.checkbox(&mut app.add_source_open_properties, "Open advanced settings (Properties) after adding");

                        // Error message
                        if let Some(err) = &app.add_source_error_message {
                            ui.add_space(4.0);
                            ui.colored_label(p.rec_red, format!("⚠ {}", err));
                        }

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // Action buttons (aligned right with precision)
                        ui.allocate_ui_with_layout(
                            Vec2::new(ui.available_width(), 28.0),
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                let add_btn = egui::Button::new(
                                    RichText::new(format!("{} Add Source", icons::PLUS))
                                        .strong()
                                        .color(egui::Color32::WHITE),
                                )
                                .fill(p.accent_primary)
                                .min_size(Vec2::new(125.0, 28.0));

                                if ui.add(add_btn).clicked() {
                                    if let Some(item) = validate_and_create_source(app) {
                                        app.studio.preview_scene_mut().add_source(item);
                                        app.studio.sync_if_linked();
                                        if app.add_source_open_properties {
                                            app.show_source_properties_dialog = true;
                                        }
                                        app.show_add_source_dialog = false;
                                    }
                                }

                                if ui
                                    .add(egui::Button::new("Cancel").min_size(Vec2::new(80.0, 28.0)))
                                    .clicked()
                                {
                                    app.show_add_source_dialog = false;
                                }
                            },
                        );
                    },
                );
            });

            if let Some(new_st) = newly_selected {
                app.add_source_selected_type = new_st;
                let prefix = new_st.default_name_prefix();
                app.add_source_name_buffer = app.studio.preview_scene().generate_unique_source_name(prefix);
                app.add_source_error_message = None;
            }
        });

    if !open {
        app.show_add_source_dialog = false;
    }
}

fn validate_and_create_source(app: &mut NexoraApp) -> Option<crate::studio::source::SourceItem> {
    let name = app.add_source_name_buffer.trim().to_string();
    if name.is_empty() {
        app.add_source_error_message = Some("Source name cannot be empty.".to_string());
        return None;
    }

    match app.add_source_selected_type {
        crate::studio::source::AddSourceType::DisplayCapture => {
            let displays = &app.capture_catalog.displays;
            let (mon_id, mon_name, res) = if let Some(d) = displays.iter().find(|d| d.id == app.add_source_file_path).or_else(|| displays.first()) {
                (d.id.clone(), d.name.clone(), (d.width, d.height))
            } else {
                ("MONITOR_0".to_string(), "Primary Monitor".to_string(), (1920, 1080))
            };
            Some(SourceItem::new(
                name,
                SourceKind::DisplayCapture {
                    monitor_id: mon_id,
                    monitor_name: mon_name,
                    resolution: res,
                    capture_cursor: true,
                },
            ))
        }
        crate::studio::source::AddSourceType::WindowCapture => {
            let windows = &app.capture_catalog.windows;
            let target_hwnd: u64 = app.add_source_file_path.parse().unwrap_or(0);
            let (hwnd, title, exe) = if let Some(w) = windows.iter().find(|w| w.hwnd == target_hwnd).or_else(|| windows.first()) {
                (w.hwnd, w.title.clone(), w.executable.clone())
            } else {
                (0x0001_0420, "Window Application".to_string(), "App.exe".to_string())
            };
            Some(SourceItem::new(
                name,
                SourceKind::WindowCapture {
                    window_hwnd: hwnd,
                    window_title: title,
                    executable_name: exe,
                    capture_cursor: true,
                },
            ))
        }
        crate::studio::source::AddSourceType::Image => {
            let trimmed_path = app.add_source_file_path.trim();
            if trimmed_path.is_empty() {
                app.add_source_error_message = Some("Please select an image file first.".to_string());
                return None;
            }
            if !std::path::Path::new(trimmed_path).exists() {
                app.add_source_error_message = Some("Image file not found at the specified path.".to_string());
                return None;
            }
            Some(SourceItem::new(
                name,
                SourceKind::ImageSource {
                    file_path: trimmed_path.to_string(),
                    opacity: 1.0,
                },
            ))
        }
        crate::studio::source::AddSourceType::MediaSource => {
            let trimmed_path = app.add_source_file_path.trim();
            if trimmed_path.is_empty() {
                app.add_source_error_message = Some("Please select a media/video file first.".to_string());
                return None;
            }
            if !std::path::Path::new(trimmed_path).exists() {
                app.add_source_error_message = Some("Media file not found at the specified path.".to_string());
                return None;
            }
            Some(SourceItem::new(
                name,
                SourceKind::MediaSource {
                    is_local_file: true,
                    file_path: trimmed_path.to_string(),
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
            ))
        }
        crate::studio::source::AddSourceType::TextGdiPlus => {
            let text = if app.add_source_text_content.trim().is_empty() {
                "NEXORA Studio Live".to_string()
            } else {
                app.add_source_text_content.clone()
            };
            Some(SourceItem::new(
                name,
                SourceKind::TextGdiPlus {
                    text,
                    font_size: 32.0,
                    color_rgba: [255, 255, 255, 255],
                    effect: crate::studio::source::TextAnimationEffect::Static,
                    scroll_speed: 95.0,
                    loop_animation: true,
                    loop_gap_px: 80.0,
                    banner_style: crate::studio::source::TvBannerStyle::PlainText,
                    bg_color_rgba: [15, 23, 42, 225],
                    accent_color_rgba: [220, 38, 38, 255],
                    badge_text: String::new(),
                    text_shadow: true,
                },
            ))
        }
        crate::studio::source::AddSourceType::ColorSource => {
            Some(SourceItem::new(
                name,
                SourceKind::ColorSource {
                    color_rgba: app.add_source_color_rgba,
                    width: 1920,
                    height: 1080,
                },
            ))
        }
        crate::studio::source::AddSourceType::PowerPoint => {
            let path_or_hwnd = app.add_source_file_path.trim();
            let hwnd: u64 = path_or_hwnd.parse().unwrap_or(0);
            let (file_path, window_hwnd, pres_name) = if hwnd != 0 {
                let win_title = app.capture_catalog.windows.iter().find(|w| w.hwnd == hwnd).map(|w| w.title.clone()).unwrap_or_else(|| "PowerPoint Slide Show".to_string());
                (String::new(), hwnd, win_title)
            } else if !path_or_hwnd.is_empty() {
                (path_or_hwnd.to_string(), 0, name.clone())
            } else {
                (String::new(), 0, "PowerPoint Slide Show".to_string())
            };

            let mut slides = Vec::new();
            if window_hwnd == 0 && !file_path.is_empty() {
                slides = crate::platform::get_or_export_ppt_slides(&file_path);
            }
            let slide_count = slides.len();

            Some(SourceItem::new(
                name,
                SourceKind::PowerPointSource {
                    presentation_name: pres_name,
                    file_path,
                    window_hwnd,
                    slide_index: 0,
                    slide_count,
                    slides,
                    load_attempted: true,
                },
            ))
        }
        crate::studio::source::AddSourceType::Pdf => {
            let trimmed_path = app.add_source_file_path.trim();
            if trimmed_path.is_empty() {
                app.add_source_error_message = Some("Please select a PDF file first.".to_string());
                return None;
            }
            if !std::path::Path::new(trimmed_path).exists() {
                app.add_source_error_message = Some("PDF file not found at the specified path.".to_string());
                return None;
            }
            let doc_name = std::path::Path::new(trimmed_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&name)
                .to_string();
            let pages = crate::platform::get_or_export_pdf_pages(trimmed_path);
            let page_count = pages.len();
            let mut item = SourceItem::new(
                name,
                SourceKind::PdfSource {
                    document_name: doc_name,
                    file_path: trimmed_path.to_string(),
                    page_index: 0,
                    page_count,
                    pages,
                    load_attempted: true,
                },
            );
            item.transform.stretch_to_screen();
            Some(item)
        }
        crate::studio::source::AddSourceType::BrowserSource => {
            let url = if app.add_source_browser_url.trim().is_empty() {
                "https://obsproject.com/browser-source".to_string()
            } else {
                app.add_source_browser_url.trim().to_string()
            };
            Some(SourceItem::new(
                name,
                SourceKind::BrowserSource {
                    url,
                    width: 800,
                    height: 600,
                    fps: 30,
                },
            ))
        }
        crate::studio::source::AddSourceType::WebcamDevice => {
            let cameras = &app.capture_catalog.cameras;
            let (cid, cname) = if let Some(c) = cameras.iter().find(|c| c.id == app.add_source_file_path).or_else(|| cameras.first()) {
                (c.id.clone(), c.name.clone())
            } else {
                ("CAM_0".to_string(), "Integrated Camera".to_string())
            };
            Some(SourceItem::new(
                name,
                SourceKind::WebcamDevice {
                    device_id: cid,
                    device_name: cname,
                    resolution: (1280, 720),
                    corner_radius: 0.0,
                    border_thickness: 0.0,
                    border_color_rgba: [255, 255, 255, 255],
                },
            ))
        }
        crate::studio::source::AddSourceType::ImageSlideShow => {
            Some(SourceItem::new(
                name,
                SourceKind::ImageSlideShow {
                    files: Vec::new(),
                    slide_time_ms: 4500,
                    transition_duration_ms: 600,
                    loop_slides: true,
                    transition_effect: crate::studio::source::SlideTransitionEffect::CrossFade,
                },
            ))
        }
        crate::studio::source::AddSourceType::TimeSource => {
            let mut item = SourceItem::new(
                name,
                SourceKind::TimeSource {
                    mode: crate::studio::source::TimeSourceMode::RealTimeClock,
                    target_duration_sec: 300,
                    elapsed_sec: 0.0,
                    is_running: true,
                    effect: crate::studio::source::TimeVisualEffect::DigitalNeon,
                    font_size: 52.0,
                    font_family: crate::studio::source::TimeFontFamily::Monospace,
                    bold_font: true,
                    border_thickness: 2.0,
                    show_badge: true,
                    text_color_rgba: [255, 255, 255, 255],
                    bg_color_rgba: [15, 23, 42, 220],
                    accent_color_rgba: [0, 210, 255, 255],
                    show_hours: true,
                    blink_colon: false,
                    use_24h_format: true,
                    badge_text: String::new(),
                    badge_font_size: 14.0,
                    badge_position: crate::studio::source::TimeBadgePosition::TopLeft,
                    badge_offset_x: 0.0,
                    badge_offset_y: 0.0,
                    finish_text: String::new(),
                },
            );
            item.transform.pos_x = 0.04;
            item.transform.pos_y = 0.04;
            item.transform.scale_x = 0.30;
            item.transform.scale_y = 0.15;
            Some(item)
        }
    }
}

pub fn render_eyedropper_overlay(ctx: &egui::Context, app: &mut NexoraApp) {
    if !app.eyedropper_active {
        return;
    }

    let ptr = ctx.input(|i| i.pointer.latest_pos());

    // Sample color at cursor: directly from canvas/sources, fallback to OS screen
    if let Some(p) = ptr {
        if let Some(rgb) = sample_canvas_pixel_at(app, p) {
            app.eyedropper_color = Some(rgb);
        }
    }

    // Force crosshair cursor while sampling
    ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
    ctx.request_repaint();

    // Cancellation: Escape key or right click
    if ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.pointer.secondary_clicked()) {
        app.eyedropper_active = false;
        app.eyedropper_armed = false;
        return;
    }

    // Arm eyedropper only after mouse button has been released from clicking the "Pipet Layar" button
    if !app.eyedropper_armed {
        if !ctx.input(|i| i.pointer.primary_down()) {
            app.eyedropper_armed = true;
        }
    } else {
        // Armed! Click anywhere on canvas or window to commit selected color
        if ctx.input(|i| i.pointer.primary_clicked() || i.pointer.primary_down()) {
            if let Some([r, g, b]) = app.eyedropper_color {
                if let Some(src) = app.studio.preview_scene_mut().selected_source_mut() {
                    src.filter.key_color_rgb = [r, g, b];
                    src.filter.key_color_preset = crate::studio::source::ChromaKeyPresetColor::Custom;
                    if let SourceKind::ColorSource { color_rgba, .. } = &mut src.kind {
                        *color_rgba = [r, g, b, 255];
                    }
                }
                app.studio.sync_if_linked();
            }
            app.eyedropper_active = false;
            app.eyedropper_armed = false;
            return;
        }
    }

    // Floating magnifying/preview pill next to mouse cursor
    if let Some(p) = ptr {
        let p_theme = ThemePalette::for_mode(app.settings.theme);
        let tooltip_pos = p + egui::vec2(16.0, 16.0);
        egui::Area::new(egui::Id::new("nexora_eyedropper_floating_pill"))
            .order(egui::Order::Tooltip)
            .fixed_pos(tooltip_pos)
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(p_theme.bg_panel)
                    .stroke(egui::Stroke::new(1.5_f32, p_theme.accent_primary))
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .corner_radius(6)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if let Some([r, g, b]) = app.eyedropper_color {
                                let (rect, _) =
                                    ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(r, g, b));
                                ui.painter().rect_stroke(
                                    rect,
                                    4.0,
                                    egui::Stroke::new(1.0_f32, egui::Color32::WHITE),
                                    egui::StrokeKind::Inside,
                                );
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(format!("#{:02X}{:02X}{:02X}", r, g, b))
                                            .strong()
                                            .color(p_theme.text_primary),
                                    );
                                    ui.label(
                                        RichText::new("Click canvas: Pick Color • Esc: Cancel")
                                            .small()
                                            .color(p_theme.text_secondary),
                                    );
                                });
                            } else {
                                ui.label(
                                    RichText::new("Hover over canvas...")
                                        .small()
                                        .color(p_theme.text_secondary),
                                );
                            }
                        });
                    });
            });
    }
}

/// Samples the exact RGB pixel value at a given position from Preview/Program canvas sources
fn sample_canvas_pixel_at(app: &NexoraApp, pos: egui::Pos2) -> Option<[u8; 3]> {
    // 1. Try Preview canvas first
    if let Some(canvas_rect) = app.preview_canvas_rect {
        if canvas_rect.contains(pos) {
            return sample_scene_sources_at(app, app.studio.preview_scene(), canvas_rect, pos);
        }
    }
    // 2. Try Program canvas next
    if let Some(canvas_rect) = app.program_canvas_rect {
        if canvas_rect.contains(pos) {
            return sample_scene_sources_at(app, app.studio.program_scene(), canvas_rect, pos);
        }
    }
    // 3. Fallback: OS level screen pixel sampler
    crate::platform::sample_screen_pixel_at_cursor()
}

fn sample_scene_sources_at(
    app: &NexoraApp,
    scene: &crate::studio::scene::Scene,
    canvas_rect: egui::Rect,
    pos: egui::Pos2,
) -> Option<[u8; 3]> {
    let norm_x = ((pos.x - canvas_rect.min.x) / canvas_rect.width()).clamp(0.0, 1.0);
    let norm_y = ((pos.y - canvas_rect.min.y) / canvas_rect.height()).clamp(0.0, 1.0);

    // Hit test sources in OBS order: top to bottom (index 0 is topmost foreground)
    for src in scene.sources.iter() {
        if !src.visible {
            continue;
        }
        let (vx, vy, vw, vh) = src.transform.visible_norm_rect();
        if norm_x >= vx && norm_x <= vx + vw && norm_y >= vy && norm_y <= vy + vh {
            let u = if vw > 0.0001 { ((norm_x - vx) / vw).clamp(0.0, 1.0) } else { 0.0 };
            let v = if vh > 0.0001 { ((norm_y - vy) / vh).clamp(0.0, 1.0) } else { 0.0 };

            match &src.kind {
                SourceKind::ColorSource { color_rgba, .. } => {
                    return Some([color_rgba[0], color_rgba[1], color_rgba[2]]);
                }
                SourceKind::ImageSource { file_path, .. } => {
                    let key = format!("IMG:{}", file_path);
                    if let Some(rgb) = app.capture_engine.sample_latest_frame_pixel(&key, u, v) {
                        return Some(rgb);
                    }
                }
                SourceKind::ImageSlideShow { files, .. } => {
                    let valid_files: Vec<&String> = files.iter().filter(|f| !f.trim().is_empty()).collect();
                    for f in valid_files {
                        let key = format!("IMG:{}", f);
                        if let Some(rgb) = app.capture_engine.sample_latest_frame_pixel(&key, u, v) {
                            return Some(rgb);
                        }
                    }
                }
                SourceKind::MediaSource { file_path, .. } => {
                    let key = format!("MEDIA:{}", file_path);
                    if let Some(rgb) = app.capture_engine.sample_latest_frame_pixel(&key, u, v) {
                        return Some(rgb);
                    }
                }
                SourceKind::WebcamDevice { device_id, resolution, .. } => {
                    let key = format!("CAM:{}:{}x{}", device_id, resolution.0, resolution.1);
                    if let Some(rgb) = app.capture_engine.sample_latest_frame_pixel(&key, u, v) {
                        return Some(rgb);
                    }
                }
                SourceKind::DisplayCapture { monitor_id, capture_cursor, .. } => {
                    let resolved_id = app
                        .capture_catalog
                        .find_display(monitor_id)
                        .map(|d| d.id.as_str())
                        .unwrap_or(monitor_id.as_str());
                    let key = format!("DISP:{}:{}", resolved_id, capture_cursor);
                    if let Some(rgb) = app.capture_engine.sample_latest_frame_pixel(&key, u, v) {
                        return Some(rgb);
                    }
                }
                SourceKind::WindowCapture { window_hwnd, capture_cursor, .. }
                | SourceKind::GameCapture { window_hwnd, capture_cursor, .. } => {
                    let key = format!("WIN:{}:{}", window_hwnd, capture_cursor);
                    if let Some(rgb) = app.capture_engine.sample_latest_frame_pixel(&key, u, v) {
                        return Some(rgb);
                    }
                }
                SourceKind::TextGdiPlus { color_rgba, .. } => {
                    return Some([color_rgba[0], color_rgba[1], color_rgba[2]]);
                }
                _ => {}
            }
        }
    }

    // Default stage surface color
    Some([16, 20, 28])
}

pub fn render_virtual_camera_dialog(ctx: &egui::Context, app: &mut NexoraApp) {
    if !app.show_virtual_camera_dialog {
        return;
    }

    let p = ThemePalette::for_mode(app.settings.theme);
    let mut is_open = app.show_virtual_camera_dialog;

    egui::Window::new(format!("{} Virtual Camera", icons::CAMERA))
        .id(egui::Id::new("nexora_vcam_modal_win_v2"))
        .order(egui::Order::Foreground)
        .open(&mut is_open)
        .collapsible(false)
        .resizable(false)
        .auto_sized()
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .default_width(300.0)
        .max_width(310.0)
        .max_height(210.0)
        .show(ctx, |ui| {
            block_window_body_drag(ui);
            const VCAM_FIELD_W: f32 = 280.0;
            ui.set_width(280.0);
            ui.set_max_width(280.0);

            ui.add_space(4.0);

            // Output Type
            ui.label(RichText::new("Output Type").size(12.0).strong().color(p.text_primary));
            ui.add_space(2.0);
            egui::ComboBox::from_id_salt("vcam_output_type_combo")
                .width(VCAM_FIELD_W)
                .selected_text(app.virtual_camera_dialog_draft_type.label())
                .show_ui(ui, |ui| {
                    ui.set_width(VCAM_FIELD_W);
                    for ot in VirtualCameraOutputType::all() {
                        let is_sel = app.virtual_camera_dialog_draft_type == *ot;
                        if ui.selectable_label(is_sel, ot.label()).clicked() {
                            app.virtual_camera_dialog_draft_type = *ot;
                        }
                    }
                });

            ui.add_space(6.0);

            // Output Selection
            ui.label(RichText::new("Output Selection").size(12.0).strong().color(p.text_primary));
            ui.add_space(2.0);
            let is_scene_mode = app.virtual_camera_dialog_draft_type == VirtualCameraOutputType::Scene;
            if is_scene_mode {
                let current_scene_name = app.studio.scenes.iter()
                    .find(|s| Some(s.id) == app.virtual_camera_dialog_draft_scene_id)
                    .map(|s| s.name.clone())
                    .or_else(|| app.studio.scenes.first().map(|s| s.name.clone()))
                    .unwrap_or_else(|| "Select Scene...".to_string());

                egui::ComboBox::from_id_salt("vcam_scene_selection_combo")
                    .width(VCAM_FIELD_W)
                    .selected_text(current_scene_name)
                    .show_ui(ui, |ui| {
                        ui.set_width(VCAM_FIELD_W);
                        for sc in &app.studio.scenes {
                            let is_sel = app.virtual_camera_dialog_draft_scene_id == Some(sc.id);
                            if ui.selectable_label(is_sel, &sc.name).clicked() {
                                app.virtual_camera_dialog_draft_scene_id = Some(sc.id);
                            }
                        }
                    });
            } else {
                ui.add_enabled_ui(false, |ui| {
                    egui::ComboBox::from_id_salt("vcam_no_selection_combo")
                        .width(VCAM_FIELD_W)
                        .selected_text("No selection for this output type")
                        .show_ui(ui, |_ui| {});
                });
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Driver:").size(11.0).color(p.text_muted));
                ui.label(
                    RichText::new("NEXORA Virtual Camera (Terdaftar)")
                        .size(11.0)
                        .strong()
                        .color(p.meter_ok),
                );
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // OK and Cancel buttons
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(
                    egui::Button::new(RichText::new("Cancel").size(12.0))
                        .min_size(Vec2::new(72.0, 24.0))
                ).clicked() {
                    app.show_virtual_camera_dialog = false;
                }

                if ui.add(
                    egui::Button::new(RichText::new("OK").strong().size(12.0).color(egui::Color32::WHITE))
                        .fill(p.accent_preview)
                        .min_size(Vec2::new(72.0, 24.0))
                ).clicked() {
                    app.virtual_camera_output_type = app.virtual_camera_dialog_draft_type;
                    app.virtual_camera_scene_id = app.virtual_camera_dialog_draft_scene_id;
                    app.show_virtual_camera_dialog = false;
                    if app.virtual_camera_active {
                        let out_label = app.virtual_camera_target_label();
                        app.recording.last_message = format!("Virtual Camera: LIVE ({})", out_label);
                    }
                }
            });
        });

    if !is_open {
        app.show_virtual_camera_dialog = false;
    }
}
