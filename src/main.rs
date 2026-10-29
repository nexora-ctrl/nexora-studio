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

    let initial_settings = settings::AppSettings::load_or_default();
    let win_w = initial_settings.window_size.0.clamp(1024.0, 3840.0);
    let win_h = initial_settings.window_size.1.clamp(620.0, 2160.0);

    let mut viewport = ViewportBuilder::default()
        .with_title("NEXORA Studio — Desktop Recording Studio")
        .with_inner_size([win_w, win_h])
        .with_min_inner_size([1024.0, 620.0])
        .with_maximized(initial_settings.window_maximized);

    if let Some(app_icon) = ui::icons::load_app_window_icon() {
        viewport = viewport.with_icon(app_icon);
    }

    if let Some((pos_x, pos_y)) = initial_settings.window_pos {
        if pos_x > -3000.0 && pos_x < 6000.0 && pos_y > -3000.0 && pos_y < 4000.0 {
            viewport = viewport.with_position([pos_x, pos_y]);
        }
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
}

