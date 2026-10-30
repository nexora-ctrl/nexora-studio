#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod app;
pub mod audio;
pub mod capture;
pub mod encoder;
pub mod media_library;
pub mod platform;
pub mod project;
pub mod recording;
pub mod settings;
pub mod studio;
pub mod ui;
pub mod virtual_camera;

use app::NexoraApp;
use egui::ViewportBuilder;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("info,wgpu_hal=error,wgpu_core=error")
            }),
        )
        .init();

    crate::platform::updater::cleanup_old_executable_if_needed();

    let initial_settings = settings::AppSettings::load_or_default();
    let win_w = initial_settings.window_size.0.clamp(1280.0, 3840.0);
    let win_h = initial_settings.window_size.1.clamp(720.0, 2160.0);

    let mut viewport = ViewportBuilder::default()
        .with_title("NEXORA Studio - Your Easy Studio.")
        .with_inner_size([win_w, win_h])
        .with_min_inner_size([1024.0, 620.0])
        .with_maximized(true);

    if let Some(app_icon) = ui::icons::load_app_window_icon() {
        viewport = viewport.with_icon(app_icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "NEXORA Studio",
        native_options,
        Box::new(|cc| Ok(Box::new(NexoraApp::new(cc)))),
    )
}

#[cfg(test)]
mod tests {
    
    use crate::project::NexoraProject;
    use crate::settings::ThemeMode;
    use crate::studio::StudioState;

    #[test]
    fn studio_mode_preview_program_separation_works() {
        let mut studio = StudioState::default();
        assert!(!studio.has_pending_preview_changes());

        // Add a new scene in Preview (sources must be empty by default)
        let new_scene_id = studio.add_scene("Scene 3 - Demo".to_string());
        assert_eq!(studio.preview_scene_id, new_scene_id);
        assert_ne!(studio.program_scene_id, new_scene_id);
        assert!(studio.preview_scene().sources.is_empty(), "New scene must have empty sources by default");
        assert!(studio.has_pending_preview_changes());

        // Apply Preview -> Program transition
        studio.transition_preview_to_program();
        assert_eq!(studio.program_scene_id, new_scene_id);
        assert!(!studio.has_pending_preview_changes());

        // Modifying a source in Preview must NOT bleed into Program snapshot before Transition
        let time_source = crate::studio::source::SourceItem::new(
            "Clock 1".to_string(),
            crate::studio::source::SourceKind::TimeSource {
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
        studio.preview_scene_mut().sources.push(time_source);
        studio.sync_media_transport_to_program();
        assert!(studio.program_scene().sources.is_empty(), "Program scene must not contain Preview source changes before transition");
        assert!(studio.has_pending_preview_changes());

        // Transition applies changes to Program
        studio.transition_preview_to_program();
        assert_eq!(studio.program_scene().sources.len(), 1);
        assert!(!studio.has_pending_preview_changes());

        // Modifying visual property in Preview (e.g. font size) does not change Program
        if let crate::studio::source::SourceKind::TimeSource { font_size, .. } = &mut studio.preview_scene_mut().sources[0].kind {
            *font_size = 80.0;
        }
        studio.sync_media_transport_to_program();
        if let crate::studio::source::SourceKind::TimeSource { font_size, .. } = &studio.program_scene().sources[0].kind {
            assert_ne!(*font_size, 80.0, "Program font_size must remain at original value before Transition");
        }
        studio.transition_preview_to_program();
        if let crate::studio::source::SourceKind::TimeSource { font_size, .. } = &studio.program_scene().sources[0].kind {
            assert_eq!(*font_size, 80.0, "Program font_size must update after Transition");
        }
    }

    #[test]
    fn theme_toggle_roundtrip() {
        let mut mode = ThemeMode::Dark;
        mode.toggle();
        assert_eq!(mode, ThemeMode::Light);
        mode.toggle();
        assert_eq!(mode, ThemeMode::Dark);
    }

    #[test]
    fn project_serialization_roundtrip() {
        let project = NexoraProject::default();
        let json = serde_json::to_string_pretty(&project).expect("serialize project");
        let restored: NexoraProject = serde_json::from_str(&json).expect("deserialize project");
        assert_eq!(restored.project_name, project.project_name);
        assert_eq!(restored.studio.scenes.len(), project.studio.scenes.len());
    }

    #[test]
    fn settings_and_screen_layout_preset_roundtrip() {
        use crate::audio::AudioMixerState;
        use crate::settings::{AppSettings, ScreenLayoutPreset};

        let mut settings = AppSettings::default();
        settings.window_size = (1600.0, 900.0);
        settings.dock_height = 280.0;
        settings.active_layout_name = "Setup Presentasi".to_string();
        settings.saved_layout_presets.push(ScreenLayoutPreset {
            name: "Setup Presentasi".to_string(),
            updated_at: "2026-10-28 15:40".to_string(),
            panels: settings.panels.clone(),
            dock_height: 280.0,
            preview_fit_to_window: true,
            preview_zoom: 1.0,
            show_safe_margins: true,
            video_preset: settings.video_preset,
            bitrate_preset: settings.bitrate_preset,
            studio: StudioState::default(),
            audio_mixer: AudioMixerState::default(),
        });

        let json = serde_json::to_string_pretty(&settings).expect("serialize settings");
        let restored: AppSettings = serde_json::from_str(&json).expect("deserialize settings");
        assert_eq!(restored.window_size, (1600.0, 900.0));
        assert_eq!(restored.dock_height, 280.0);
        assert_eq!(restored.active_layout_name, "Setup Presentasi");
        assert_eq!(restored.saved_layout_presets.len(), 1);
    }

    #[test]
    fn scene_naming_and_empty_sources_by_default() {
        let mut studio = StudioState::default();
        let name1 = studio.generate_unique_scene_name();
        assert_eq!(name1, "Scene 3");

        let id = studio.add_scene(name1);
        let scene = studio.scenes.iter().find(|s| s.id == id).expect("scene exists");
        assert_eq!(scene.name, "Scene 3");
        assert!(scene.sources.is_empty(), "New scene sources must be empty");

        let name2 = studio.generate_unique_scene_name();
        assert_eq!(name2, "Scene 4");
    }

    #[test]
    fn app_icon_ico_decodes_properly() {
        let icon = crate::ui::icons::load_app_window_icon().expect("decode icon.ico");
        assert!(icon.width >= 32 && icon.height >= 32);
        assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
    }

    #[test]
    fn test_copy_and_paste_source_across_scenes() {
        let mut studio = StudioState::default();
        let scene1 = &studio.scenes[0];
        let original_source = scene1.sources[0].clone();

        let new_scene_id = studio.add_scene("Scene 3".to_string());
        let target_scene = studio.scenes.iter_mut().find(|s| s.id == new_scene_id).unwrap();
        assert!(target_scene.sources.is_empty());

        let pasted_id = target_scene.paste_source(&original_source);
        assert_eq!(target_scene.sources.len(), 1);
        assert_eq!(target_scene.sources[0].id, pasted_id);
        assert_ne!(pasted_id, original_source.id);
        assert_eq!(target_scene.sources[0].name, original_source.name);

        // Pasting again into same scene appends (Copy)
        let second_pasted_id = target_scene.paste_source(&original_source);
        assert_eq!(target_scene.sources.len(), 2);
        assert_eq!(target_scene.sources[0].id, second_pasted_id);
        assert_eq!(target_scene.sources[0].name, format!("{} (Copy)", original_source.name));
    }

    #[test]
    fn test_virtual_camera_default_off_and_outputs() {
        use crate::app::VirtualCameraOutputType;
        assert_eq!(VirtualCameraOutputType::default(), VirtualCameraOutputType::Program);
        assert_eq!(VirtualCameraOutputType::Program.label(), "Program (Default)");
        assert_eq!(VirtualCameraOutputType::Preview.label(), "Preview");
        assert_eq!(VirtualCameraOutputType::Scene.label(), "Scene");
    }

    #[test]
    fn test_font_rubik_and_exo_installation() {
        let ctx = egui::Context::default();
        crate::ui::icons::install_tabler_icons(&ctx);
        let _ = ctx.run(Default::default(), |_| {});
        // Verify submenu arrow '\u{23f5}' is resolved cleanly without fallback '?'
        let galley = ctx.fonts(|f| {
            f.layout_no_wrap(
                "\u{23f5}".to_owned(),
                egui::FontId::new(12.0, crate::ui::icons::rubik_light_family()),
                egui::Color32::WHITE,
            )
        });
        assert_eq!(galley.rect.width(), 11.0);

        // Verify Roboto Mono renders monospace numbers cleanly
        let mono_galley = ctx.fonts(|f| {
            f.layout_no_wrap(
                "0123456789:".to_owned(),
                egui::FontId::monospace(14.0),
                egui::Color32::WHITE,
            )
        });
        assert!(mono_galley.rect.width() > 0.0);
    }

    #[test]
    fn test_updater_semver_and_state() {
        use crate::platform::updater::{parse_version, UpdaterState, UpdateStatus};
        assert_eq!(parse_version("0.3.0"), (0, 3, 0));
        assert_eq!(parse_version("v0.4.0"), (0, 4, 0));
        assert_eq!(parse_version("v1.0.2"), (1, 0, 2));
        assert!(parse_version("v0.4.0") > parse_version("0.3.0"));
        assert!(parse_version("0.3.1") > parse_version("0.3.0"));
        assert!(!(parse_version("0.3.0") > parse_version("0.3.0")));

        let updater = UpdaterState::default();
        assert_eq!(updater.status, UpdateStatus::Idle);
        assert_eq!(updater.current_version, "0.3.0");
    }
}

