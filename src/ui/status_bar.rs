use crate::app::NexoraApp;
use crate::ui::icons;
use crate::ui::theme::ThemePalette;
use egui::{RichText, Stroke};

pub fn render_status_bar(ctx: &egui::Context, app: &NexoraApp) {
    let p = ThemePalette::for_mode(app.settings.theme);

    egui::TopBottomPanel::bottom("nexora_status_bar")
        .exact_height(24.0)
        .frame(
            egui::Frame::new()
                .fill(p.bg_panel)
                .inner_margin(egui::Margin::symmetric(10, 3))
                .stroke(Stroke::new(1.0_f32, p.border_subtle)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                // Capture Status
                ui.label(
                    RichText::new(format!("Capture: {}", app.capture_catalog.status_label))
                        .small()
                        .color(p.text_secondary),
                );
                ui.separator();

                // Resolution & FPS
                let (w, h) = app.settings.video_preset.dimensions();
                let fps = app.settings.video_preset.fps();
                ui.label(
                    RichText::new(format!(
                        "Video: {}×{} @ {} FPS • {} kbps (.{})",
                        w,
                        h,
                        fps,
                        app.settings.bitrate_preset.kbps(),
                        app.settings.container_format
                    ))
                    .small()
                    .monospace()
                    .color(p.text_primary),
                );
                ui.separator();

                // Audio Status
                let desk_state = if app.audio_mixer.desktop_audio.is_disabled() {
                    "Off".to_string()
                } else if app.audio_mixer.desktop_audio.muted {
                    "Muted".to_string()
                } else {
                    format!("{}%", app.audio_mixer.desktop_audio.volume_percent())
                };
                let mic_state = if app.audio_mixer.microphone.is_disabled() {
                    "Off".to_string()
                } else if app.audio_mixer.microphone.muted {
                    "Muted".to_string()
                } else {
                    format!("{}%", app.audio_mixer.microphone.volume_percent())
                };
                ui.label(
                    RichText::new(format!(
                        "Audio: Desktop [{}] | Mic [{}]",
                        desk_state, mic_state
                    ))
                    .small()
                    .color(p.text_secondary),
                );

                if app.virtual_camera_active {
                    ui.separator();
                    ui.label(
                        RichText::new(format!("{} VCam: LIVE ({})", icons::CAMERA, app.virtual_camera_target_label()))
                            .small()
                            .strong()
                            .color(p.accent_preview),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some((msg, inst)) = &app.screenshot_notify {
                        if inst.elapsed().as_secs() < 5 {
                            ui.label(
                                RichText::new(format!("📸 {}", msg))
                                    .small()
                                    .strong()
                                    .color(p.meter_ok),
                            );
                        } else {
                            ui.label(
                                RichText::new(&app.recording.last_message)
                                    .small()
                                    .color(p.text_muted),
                            );
                        }
                    } else {
                        ui.label(
                            RichText::new(&app.recording.last_message)
                                .small()
                                .color(p.text_muted),
                        );
                    }
                });
            });
        });
}

