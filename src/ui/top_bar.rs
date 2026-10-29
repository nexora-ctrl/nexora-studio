use crate::app::{NexoraApp, QuickScreenTemplate};
use crate::recording::RecordingStatus;
use crate::settings::ThemeMode;
use crate::ui::icons;
use crate::ui::theme::{apply_studio_theme, ThemePalette};
use egui::{Color32, CornerRadius, RichText, Stroke};

pub fn render_top_bar(ctx: &egui::Context, app: &mut NexoraApp) {
    let p = ThemePalette::for_mode(app.settings.theme);

    egui::TopBottomPanel::top("nexora_top_bar")
        .exact_height(34.0)
        .frame(
            egui::Frame::new()
                .fill(p.bg_panel)
                .inner_margin(egui::Margin::symmetric(10, 4))
                .stroke(Stroke::new(1.0_f32, p.border_subtle)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                // Brand Icon (`assets/icons/icon.ico`) & Title
                if let Some(ref icon_tex) = app.brand_icon_texture {
                    ui.add(
                        egui::Image::from_texture(icon_tex)
                            .fit_to_exact_size(egui::Vec2::new(20.0, 20.0))
                            .corner_radius(CornerRadius::same(4)),
                    );
                } else {
                    let brand_badge = egui::Frame::new()
                        .fill(p.accent_primary)
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(egui::Margin::symmetric(6, 2));
                    brand_badge.show(ui, |ui| {
                        ui.label(
                            RichText::new("NX")
                                .strong()
                                .size(11.5)
                                .color(Color32::WHITE),
                        );
                    });
                }

                ui.label(
                    RichText::new("NEXORA Studio")
                        .strong()
                        .size(13.5)
                        .color(p.text_primary),
                );
                ui.label(
                    RichText::new("v0.3")
                        .size(11.0)
                        .color(p.text_muted),
                );

                ui.separator();

                // Undo / Redo Quick Buttons
                let can_undo = app.can_undo();
                let undo_steps = app.undo_count();
                if ui
                    .add_enabled(
                        can_undo,
                        egui::Button::new(RichText::new(format!("{} Undo", icons::UNDO)).size(11.5)),
                    )
                    .on_hover_text(format!(
                        "Undo last change (Ctrl+Z) — {} step(s) available",
                        undo_steps
                    ))
                    .clicked()
                {
                    app.undo();
                }

                let can_redo = app.can_redo();
                let redo_steps = app.redo_count();
                if ui
                    .add_enabled(
                        can_redo,
                        egui::Button::new(RichText::new(format!("{} Redo", icons::REDO)).size(11.5)),
                    )
                    .on_hover_text(format!(
                        "Redo change (Ctrl+Y / Ctrl+Shift+Z) — {} step(s) available",
                        redo_steps
                    ))
                    .clicked()
                {
                    app.redo();
                }

                ui.separator();

                // Project / File menu
                ui.menu_button(format!("{} Project", icons::FOLDER), |ui| {
                    ui.set_min_width(205.0);
                    if ui
                        .button(format!("{}  New Studio Project", icons::FILE))
                        .on_hover_text("Reset scenes and sources to a clean Studio project")
                        .clicked()
                    {
                        app.new_project();
                        ui.close_menu();
                    }
                    if ui
                        .button(format!(
                            "{}  Save All Settings & Layout (Ctrl+S)",
                            icons::DEVICE_FLOPPY
                        ))
                        .on_hover_text(
                            "Save all scenes, sources, screen/camera layout, audio mixer, and app settings to disk",
                        )
                        .clicked()
                    {
                        app.quick_save_project();
                        ui.close_menu();
                    }
                    if ui
                        .button(format!(
                            "{}  Save As Screen & Layout Preset...",
                            icons::PLUS
                        ))
                        .on_hover_text("Save current screen and layout setup as a named preset")
                        .clicked()
                    {
                        app.save_layout_name_buffer = app.settings.active_layout_name.clone();
                        app.show_save_layout_dialog = true;
                        ui.close_menu();
                    }
                    if ui
                        .button(format!("{}  Load Saved Snapshot", icons::FOLDER_OPEN))
                        .on_hover_text("Reload the saved project configuration from disk")
                        .clicked()
                    {
                        app.quick_load_project();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .button(format!("{}  Open Output Folder", icons::FOLDER_OPEN))
                        .clicked()
                    {
                        let _ = crate::platform::open_folder_in_explorer(
                            &app.settings.output_directory,
                        );
                        ui.close_menu();
                    }
                });

                // Screen & Layout Presets menu (Save / Load / Quick Templates)
                ui.menu_button(
                    format!(
                        "{} Screen & Layout: {}",
                        icons::TRANSFORM,
                        app.settings.active_layout_name
                    ),
                    |ui| {
                        ui.set_min_width(275.0);
                        ui.label(
                            RichText::new("Screen & Layout Saving (Auto-Saved on Exit)")
                                .small()
                                .color(p.text_muted),
                        );

                        if ui
                            .button(format!(
                                "{}  Update '{}' (Ctrl+S)",
                                icons::DEVICE_FLOPPY,
                                app.settings.active_layout_name
                            ))
                            .on_hover_text(
                                "Overwrite the active layout preset with your current screen, camera, audio, and panel setup",
                            )
                            .clicked()
                        {
                            let current = app.settings.active_layout_name.clone();
                            app.save_layout_preset(&current);
                            ui.close_menu();
                        }

                        if ui
                            .button(format!(
                                "{}  Save New Screen & Layout Preset...",
                                icons::PLUS
                            ))
                            .on_hover_text(
                                "Save current screen, sources, and panel layout under a new preset name",
                            )
                            .clicked()
                        {
                            app.save_layout_name_buffer = format!(
                                "Studio Layout {}",
                                app.settings.saved_layout_presets.len() + 1
                            );
                            app.show_save_layout_dialog = true;
                            ui.close_menu();
                        }

                        ui.separator();
                        ui.label(
                            RichText::new("Saved Screen & Layout Presets")
                                .small()
                                .color(p.text_muted),
                        );

                        let presets_snapshot: Vec<(usize, String, String)> = app
                            .settings
                            .saved_layout_presets
                            .iter()
                            .enumerate()
                            .map(|(idx, pr)| (idx, pr.name.clone(), pr.updated_at.clone()))
                            .collect();
                        let can_delete = presets_snapshot.len() > 1;
                        let mut load_idx = None;
                        let mut save_over_name = None;
                        let mut delete_idx = None;

                        for (idx, name, updated_at) in presets_snapshot {
                            let is_active = app.settings.active_layout_name == name;
                            ui.horizontal(|ui| {
                                let label_text = if is_active {
                                    format!("● {}", name)
                                } else {
                                    format!("  {}", name)
                                };
                                if ui
                                    .selectable_label(is_active, RichText::new(label_text).strong())
                                    .on_hover_text(format!(
                                        "Click to load preset '{}' (Saved: {})",
                                        name, updated_at
                                    ))
                                    .clicked()
                                {
                                    load_idx = Some(idx);
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if can_delete
                                            && ui
                                                .small_button(icons::TRASH)
                                                .on_hover_text("Delete this layout preset")
                                                .clicked()
                                        {
                                            delete_idx = Some(idx);
                                        }
                                        if ui
                                            .small_button(icons::DEVICE_FLOPPY)
                                            .on_hover_text(
                                                "Save current screen & layout into this preset",
                                            )
                                            .clicked()
                                        {
                                            save_over_name = Some(name.clone());
                                        }
                                    },
                                );
                            });
                        }

                        if let Some(idx) = load_idx {
                            app.load_layout_preset(idx);
                            ui.close_menu();
                        }
                        if let Some(name) = save_over_name {
                            app.save_layout_preset(&name);
                            ui.close_menu();
                        }
                        if let Some(idx) = delete_idx {
                            app.delete_layout_preset(idx);
                        }

                        ui.separator();
                        ui.label(
                            RichText::new("Quick Screen & Camera Templates")
                                .small()
                                .color(p.text_muted),
                        );

                        if ui
                            .button(format!(
                                "{}  Full Screen + Cam Bottom-Right (PiP)",
                                icons::DEVICE_DESKTOP
                            ))
                            .clicked()
                        {
                            app.apply_quick_screen_template(
                                QuickScreenTemplate::FullScreenCamBottomRight,
                            );
                            ui.close_menu();
                        }
                        if ui
                            .button(format!(
                                "{}  Full Screen + Cam Bottom-Left (PiP)",
                                icons::DEVICE_DESKTOP
                            ))
                            .clicked()
                        {
                            app.apply_quick_screen_template(
                                QuickScreenTemplate::FullScreenCamBottomLeft,
                            );
                            ui.close_menu();
                        }
                        if ui
                            .button(format!(
                                "{}  Full Screen + Cam Top-Right (PiP)",
                                icons::DEVICE_DESKTOP
                            ))
                            .clicked()
                        {
                            app.apply_quick_screen_template(
                                QuickScreenTemplate::FullScreenCamTopRight,
                            );
                            ui.close_menu();
                        }
                        if ui
                            .button(format!(
                                "{}  Side-by-Side Split (Screen + Camera)",
                                icons::LAYOUT_COLUMNS
                            ))
                            .clicked()
                        {
                            app.apply_quick_screen_template(QuickScreenTemplate::SideBySideSplit);
                            ui.close_menu();
                        }
                        if ui
                            .button(format!(
                                "{}  Full Screen Only (Hide Camera)",
                                icons::MAXIMIZE
                            ))
                            .clicked()
                        {
                            app.apply_quick_screen_template(QuickScreenTemplate::FullScreenOnly);
                            ui.close_menu();
                        }
                    },
                );

                // Workspace / View menu for panel minimization
                ui.menu_button(format!("{} Panels", icons::LAYOUT_COLUMNS), |ui| {
                    ui.set_min_width(195.0);
                    ui.label(RichText::new("Workspace Mode").small().color(p.text_muted));
                    if ui
                        .checkbox(
                            &mut app.settings.panels.studio_mode,
                            "Studio Mode (Dual Preview)",
                        )
                        .changed()
                    {
                        let _ = app.settings.save();
                    }
                    ui.separator();
                    ui.label(RichText::new("Dock Panels").small().color(p.text_muted));
                    let mut changed = false;
                    changed |= ui
                        .checkbox(&mut app.settings.panels.show_scenes, "Scenes Panel")
                        .changed();
                    changed |= ui
                        .checkbox(&mut app.settings.panels.show_sources, "Source Panel")
                        .changed();
                    changed |= ui
                        .checkbox(&mut app.settings.panels.show_audio_mixer, "Audio Mixer")
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut app.settings.panels.show_recording_controls,
                            "Controls Panel",
                        )
                        .changed();
                    changed |= ui
                        .checkbox(
                            &mut app.settings.panels.show_library_drawer,
                            "Recording Library Drawer",
                        )
                        .changed();
                    if changed {
                        let _ = app.settings.save();
                    }
                });

                // Studio Mode quick pill
                let studio_btn_text = if app.settings.panels.studio_mode {
                    format!("{} Studio Mode: ON", icons::LAYOUT_COLUMNS)
                } else {
                    format!("{} Studio Mode: Single", icons::LAYOUT_COLUMNS)
                };
                if ui
                    .selectable_label(app.settings.panels.studio_mode, studio_btn_text)
                    .on_hover_text("Toggle Dual Preview + Program Studio workspace")
                    .clicked()
                {
                    app.settings.panels.studio_mode = !app.settings.panels.studio_mode;
                    let _ = app.settings.save();
                }

                // Right-aligned indicators and controls
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Settings Dialog trigger
                    if ui
                        .button(format!("{} Settings", icons::SETTINGS))
                        .on_hover_text("Configure video resolution, FPS, bitrate, and output path")
                        .clicked()
                    {
                        app.show_settings_dialog = !app.show_settings_dialog;
                    }

                    // Recording Library Drawer toggle
                    if ui
                        .selectable_label(
                            app.settings.panels.show_library_drawer,
                            format!("{} Library", icons::MOVIE),
                        )
                        .on_hover_text("Toggle Recording Library side drawer")
                        .clicked()
                    {
                        app.settings.panels.show_library_drawer =
                            !app.settings.panels.show_library_drawer;
                        if app.settings.panels.show_library_drawer {
                            app.media_library
                                .scan_directory(&app.settings.output_directory);
                        }
                        let _ = app.settings.save();
                    }

                    // Theme toggle (Dark / Light)
                    let theme_label = match app.settings.theme {
                        ThemeMode::Dark => format!("{} Light Mode", icons::SUN),
                        ThemeMode::Light => format!("{} Dark Mode", icons::MOON),
                    };
                    if ui
                        .button(theme_label)
                        .on_hover_text("Switch between Dark Mode and Light Mode across all panels")
                        .clicked()
                    {
                        app.settings.theme.toggle();
                        apply_studio_theme(ctx, app.settings.theme);
                        let _ = app.settings.save();
                    }

                    ui.separator();

                    // Live Recording Status Indicator Pill
                    let (badge_color, status_text) = match app.recording.status {
                        RecordingStatus::Idle => (
                            p.border_strong,
                            format!("{} STANDBY", icons::PLAYER_RECORD),
                        ),
                        RecordingStatus::Recording => (
                            p.rec_red,
                            format!("{} REC", icons::PLAYER_RECORD),
                        ),
                        RecordingStatus::Paused => (
                            p.pause_amber,
                            format!("{} PAUSED", icons::PLAYER_PAUSE),
                        ),
                    };

                    egui::Frame::new()
                        .fill(badge_color.gamma_multiply(0.18))
                        .stroke(Stroke::new(1.0_f32, badge_color))
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(egui::Margin::symmetric(8, 2))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(status_text)
                                        .strong()
                                        .size(11.0)
                                        .color(if app.recording.status == RecordingStatus::Idle {
                                            p.text_secondary
                                        } else {
                                            badge_color
                                        }),
                                );
                                ui.label(
                                    RichText::new(app.recording.formatted_timer())
                                        .monospace()
                                        .strong()
                                        .size(11.5)
                                        .color(p.text_primary),
                                );
                            });
                        });
                });
            });
        });
}

