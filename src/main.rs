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

    let mut wgpu_options = eframe::egui_wgpu::WgpuConfiguration::default();
    wgpu_options.present_mode = eframe::egui_wgpu::wgpu::PresentMode::AutoVsync;
    wgpu_options.desired_maximum_frame_latency = Some(1);

    // Pin the renderer to Vulkan / DirectX 12 and keep the OpenGL backend out of the adapter list.
    //
    // `egui_wgpu`'s default is `Backends::PRIMARY | Backends::GL`, so on a laptop with an Intel
    // iGPU wgpu may happily pick the *OpenGL* adapter. That adapter lives in `igxelpicd64.dll`,
    // whose driver faults with an access violation (0xc0000005) as soon as a second window — the
    // Projector — asks for its own GL surface, which force-closes the whole studio. Vulkan and
    // DirectX 12 share no code with that driver and every Windows 10/11 machine has one of them.
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = &mut wgpu_options.wgpu_setup {
        setup.instance_descriptor.backends =
            eframe::egui_wgpu::wgpu::Backends::VULKAN | eframe::egui_wgpu::wgpu::Backends::DX12;
        // Prefer the discrete GPU: it keeps the projector smooth and stays away from the iGPU.
        setup.power_preference = eframe::egui_wgpu::wgpu::PowerPreference::HighPerformance;
    }

    let native_options = eframe::NativeOptions {
        viewport,
        vsync: true,
        renderer: eframe::Renderer::Wgpu,
        wgpu_options,
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

        // Verify native feel: static labels are non-selectable across both themes
        let ctx = egui::Context::default();
        crate::ui::theme::apply_studio_theme(&ctx, ThemeMode::Dark);
        assert!(!ctx.style().interaction.selectable_labels, "Labels must be non-selectable for native feel");
        assert!(!ctx.style().interaction.multi_widget_text_select);

        crate::ui::theme::apply_studio_theme(&ctx, ThemeMode::Light);
        assert!(!ctx.style().interaction.selectable_labels, "Labels must be non-selectable in Light mode");
        assert!(!ctx.style().interaction.multi_widget_text_select);
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
            snapping_enabled: true,
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
        assert!(parse_version("v0.6.0") > parse_version("0.5.0"));
        assert!(parse_version("0.3.1") > parse_version("0.3.0"));
        assert!(!(parse_version("0.3.0") > parse_version("0.3.0")));

        let updater = UpdaterState::default();
        assert_eq!(updater.status, UpdateStatus::Idle);
        assert_eq!(updater.current_version, "0.7.0");
    }

    #[test]
    fn test_reset_transform_to_native_dimensions_and_aspect_ratio() {
        use crate::studio::source::SourceTransform;

        let mut transform = SourceTransform::default();
        // Modify transform (rotated, cropped, flipped, stretched)
        transform.rotate_cw_90();
        transform.flip_horizontal();
        transform.crop_left = 0.2;
        transform.scale_x = 1.0;
        transform.scale_y = 1.0;

        // Native square image (500x500) on 1920x1080 canvas
        let canvas_w = 1920.0_f32;
        let canvas_h = 1080.0_f32;
        let native_w = 500.0_f32;
        let native_h = 500.0_f32;
        let norm_w = native_w / canvas_w;
        let norm_h = native_h / canvas_h;

        // Reset to native size
        transform.reset_to_native_size(norm_w, norm_h);

        assert_eq!(transform.rotation_deg, 0);
        assert!(!transform.flip_h);
        assert!(!transform.flip_v);
        assert!(!transform.has_crop());
        assert!(transform.lock_aspect);
        assert_eq!(transform.pos_x, 0.0);
        assert_eq!(transform.pos_y, 0.0);
        assert!((transform.scale_x - norm_w).abs() < 1e-4);
        assert!((transform.scale_y - norm_h).abs() < 1e-4);

        // Aspect ratio on canvas: (scale_x * canvas_w) / (scale_y * canvas_h) == 1.0 (square 1:1)
        let rendered_px_w = transform.effective_width_norm() * canvas_w;
        let rendered_px_h = transform.effective_height_norm() * canvas_h;
        assert!((rendered_px_w - 500.0).abs() < 0.1);
        assert!((rendered_px_h - 500.0).abs() < 0.1);

        // Fit to screen with native size
        transform.fit_to_screen_with_native_size(norm_w, norm_h);
        assert!(transform.lock_aspect);
        // Height should fit canvas (1080), width should be 1080 (500:500 aspect ratio)
        let fit_px_w = transform.effective_width_norm() * canvas_w;
        let fit_px_h = transform.effective_height_norm() * canvas_h;
        assert!((fit_px_h - 1080.0).abs() < 0.1);
        assert!((fit_px_w - 1080.0).abs() < 0.1);
    }

    #[test]
    fn source_bus_device_name_matching_is_tolerant() {
        use crate::audio::endpoints::{is_default_device_entry, normalize_device_name, score_device_name};

        // Exact and case-insensitive matches win
        assert_eq!(score_device_name("Speakers", "Speakers"), 1_000);
        assert_eq!(score_device_name("Speakers", "speakers"), 1_000);

        // Punctuation / spacing differences are folded away (the real UI truncates with "…")
        let winmm = "Speakers (Realtek(R) Audio)";
        let mmdevice = "Speakers (Realtek(R) Audio)";
        assert!(score_device_name(winmm, mmdevice) >= 900);

        let truncated = "Microphone Array (2- Intel® Smart Sound Technology for Di…";
        let full = "Microphone Array (Intel® Smart Sound Technology for Digital Microphones)";
        assert!(
            score_device_name(truncated, full) > 0,
            "a UI-truncated endpoint name must still resolve to its device"
        );

        // Unrelated devices must not match at all
        assert_eq!(score_device_name("Speakers", "Headphones"), 0);

        // Empty input never matches, so the caller falls back to the default endpoint
        assert_eq!(score_device_name("", "Speakers"), 0);
        assert_eq!(score_device_name("Speakers", "   "), 0);

        // Normalization strips vendor punctuation and lowercases
        assert_eq!(normalize_device_name("Speakers (2- Realtek®)"), "speakers2realtek");

        // Synthetic "use the Windows default" entries
        assert!(is_default_device_entry("Default System Output (WASAPI)"));
        assert!(is_default_device_entry("Default Microphone (WASAPI)"));
        assert!(is_default_device_entry(""));
        assert!(!is_default_device_entry("Speakers (Realtek(R) Audio)"));
    }

    #[test]
    fn source_bus_gain_is_isolated_from_the_global_mixer() {
        use crate::audio::bus::{buses_signature, SourceBusKind, SourceBusSpec};

        let bus = |bus_id: &str, volume: f32, muted: bool, enabled: bool| SourceBusSpec {
            bus_id: bus_id.to_string(),
            label: bus_id.to_string(),
            kind: SourceBusKind::CaptureInput,
            device_name: "Microphone Array".to_string(),
            volume,
            muted,
            enabled,
            filters: Vec::new(),
        };

        // A live, unmuted source bus keeps its gain — it does not read any global mixer state,
        // which is the whole point of Option A (muting `Mic / Aux` cannot silence it).
        let live = bus("nx_src_a", 1.0, false, true);
        assert!((live.effective_gain() - 1.0).abs() < f32::EPSILON);

        // Muting the *source* bus zeroes only that bus
        assert_eq!(bus("nx_src_a", 1.0, true, true).effective_gain(), 0.0);

        // A source that is not live on Program (Preview standby / hidden) is silenced
        assert_eq!(bus("nx_src_a", 1.0, false, false).effective_gain(), 0.0);

        // Gain is clamped so a fader can never amplify past +3.5 dB
        assert!((bus("nx_src_a", 9.0, false, true).effective_gain() - 1.5).abs() < f32::EPSILON);

        // Routing signatures ignore fader / mute changes, so no device is needlessly re-opened
        let a = bus("nx_src_a", 1.0, false, true);
        let b = bus("nx_src_a", 0.2, true, false);
        assert_eq!(a.routing_signature(), b.routing_signature());

        // ...but the full signature does change, so the encoder still receives the new gain
        assert_ne!(a.spec_signature(), b.spec_signature());

        // A list-level signature changes when a source is added or removed
        let one = buses_signature(&[a.clone()]);
        let two = buses_signature(&[a.clone(), bus("nx_src_b", 1.0, false, true)]);
        assert_ne!(one, two);
        assert_eq!(one, buses_signature(&[a]));
    }

    #[test]
    fn audio_mixer_serde_keeps_undo_hash_stable() {
        use crate::audio::AudioMixerState;

        // The per-source meter registry is runtime-only; it must not leak into the serialized
        // form, otherwise every frame would look like an undoable change.
        let mut mixer = AudioMixerState::default();
        let before = serde_json::to_string(&mixer).expect("mixer must serialize");

        mixer.source_levels.insert(
            "nx_src_test".to_string(),
            crate::audio::SourceLevel {
                current_peak: 0.8,
                peak_hold: 0.9,
                peak_hold_timer: 1.0,
            },
        );
        let after = serde_json::to_string(&mixer).expect("mixer must still serialize");

        assert_eq!(
            before, after,
            "live meter values must be #[serde(skip)] so the undo hash stays stable"
        );
    }

    #[test]
    fn source_meter_ballistics_decay_to_silence() {
        use crate::audio::SourceLevel;

        let mut level = SourceLevel::default();
        // Loud input drives the meter and the peak-hold pill up
        for _ in 0..40 {
            level.update(1.0, 1.0, 0.016);
        }
        assert!(level.current_peak > 0.5, "meter must respond to signal");
        assert!(level.peak_hold > 0.5, "peak hold must capture the peak");

        // Silence then releases the meter smoothly towards zero
        for _ in 0..200 {
            level.update(0.0, 1.0, 0.016);
        }
        assert!(level.current_peak < 0.01, "meter must fall back to silence");
        assert!(level.peak_hold < 0.01, "peak hold must glide back down");

        // An explicit silence() zeroes everything immediately (mute / disabled bus)
        level.silence();
        assert_eq!(level.current_peak, 0.0);
        assert_eq!(level.peak_hold, 0.0);
        assert_eq!(level.peak_hold_timer, 0.0);
    }

    /// 100 Hz sine at 48 kHz — a stand-in for speech that survives the 12 kHz hiss rolloff.
    fn audio_test_tone(n: usize, amp: f32) -> f32 {
        amp * (2.0 * std::f32::consts::PI * 100.0 * n as f32 / 48_000.0).sin()
    }

    #[test]
    fn audio_gain_filter_scales_the_signal() {
        use crate::audio::filters::{AudioFilter, AudioFilterChainRuntime, AudioFilterKind};

        let mut f = AudioFilter::new(AudioFilterKind::Gain);
        f.gain_db = 6.0;
        let mut chain = AudioFilterChainRuntime::from_chain(&[f]);

        // +6 dB is a linear factor of ~1.995, applied from the very first sample
        let (l, r) = chain.process(0.1, -0.1);
        assert!(
            (l - 0.1995).abs() < 0.002,
            "+6 dB must roughly double the level, got {l}"
        );
        assert!((r + 0.1995).abs() < 0.002, "gain must apply to both channels");

        // A bypassed filter is never instantiated, so it costs nothing on the audio thread
        let mut off = AudioFilter::new(AudioFilterKind::Gain);
        off.gain_db = 30.0;
        off.enabled = false;
        assert!(
            AudioFilterChainRuntime::from_chain(&[off]).is_transparent(),
            "a bypassed filter must not run"
        );
    }

    #[test]
    fn audio_noise_gate_closes_on_silence_after_opening() {
        use crate::audio::filters::{AudioFilter, AudioFilterChainRuntime, AudioFilterKind};

        let mut f = AudioFilter::new(AudioFilterKind::NoiseGate);
        f.gate_threshold_db = -45.0;
        let mut chain = AudioFilterChainRuntime::from_chain(&[f]);

        // Speech-level input must open the gate and pass almost intact
        let mut out = (0.0, 0.0);
        for n in 0..9_600 {
            let s = if n % 2 == 0 { 0.5 } else { -0.5 };
            out = chain.process(s, s);
        }
        assert!(
            out.0.abs() > 0.45,
            "speech-level input must pass the gate, got {}",
            out.0
        );

        // Sub-threshold room tone must then be driven fully to silence
        for n in 0..48_000 {
            let s = if n % 2 == 0 { 0.0002 } else { -0.0002 };
            out = chain.process(s, s);
        }
        assert!(
            out.0.abs() < 1.0e-3,
            "sub-threshold noise must be gated away, got {}",
            out.0
        );
    }

    #[test]
    fn audio_noise_suppression_lowers_hiss_but_keeps_speech() {
        use crate::audio::filters::{AudioFilter, AudioFilterChainRuntime, AudioFilterKind};

        let mut f = AudioFilter::new(AudioFilterKind::NoiseSuppression);
        f.ns_reduction_db = 12.0;
        f.ns_sensitivity = 0.5;

        // Steady hiss below the detection threshold is pushed down, not chopped off
        let mut hiss_chain = AudioFilterChainRuntime::from_chain(&[f.clone()]);
        let mut hiss_peak = 0.0_f32;
        for n in 0..48_000 {
            let s = audio_test_tone(n, 0.001);
            let (l, _) = hiss_chain.process(s, s);
            if n >= 47_520 {
                hiss_peak = hiss_peak.max(l.abs());
            }
        }
        assert!(
            hiss_peak < 0.0006,
            "hiss must be attenuated, got {hiss_peak}"
        );

        // Speech sits above the threshold and comes through essentially untouched
        let mut speech_chain = AudioFilterChainRuntime::from_chain(&[f]);
        let mut speech_peak = 0.0_f32;
        for n in 0..48_000 {
            let s = audio_test_tone(n, 0.5);
            let (l, _) = speech_chain.process(s, s);
            if n >= 47_520 {
                speech_peak = speech_peak.max(l.abs());
            }
        }
        assert!(
            speech_peak > 0.4,
            "speech must survive noise suppression, got {speech_peak}"
        );
    }

    #[test]
    fn audio_filter_chain_supports_cleaning_mic_and_controls_params() {
        use crate::audio::filters::{AudioFilter, AudioFilterChainRuntime, AudioFilterKind};

        // `Clean Mic` installs the documented two-stage chain for a microphone and is a
        // usable starting point on its own: it must remove hiss yet let speech through.
        let mut chain = AudioFilterChainRuntime::from_chain(&crate::audio::filters::clean_mic_preset());
        assert!(!chain.is_transparent(), "Clean Mic preset must do something");

        // Speech must survive the whole chain
        let mut peak = 0.0_f32;
        for n in 0..24_000 {
            let s = audio_test_tone(n, 0.5);
            let (l, _) = chain.process(s, s);
            if n >= 23_520 {
                peak = peak.max(l.abs());
            }
        }
        assert!(peak > 0.4, "Clean Mic must keep speech intact, got {peak}");

        // Filters must be individually bypassable without disturbing the rest
        let mut bypassed = AudioFilter::new(AudioFilterKind::NoiseGate);
        bypassed.enabled = false;
        let mut chain = AudioFilterChainRuntime::from_chain(&[bypassed]);
        assert!(chain.is_transparent(), "a disabled chain must be transparent");
        assert_eq!(chain.process(0.25, -0.25), (0.25, -0.25));
    }

    #[test]
    fn audio_filter_signature_tracks_order_and_parameters() {
        use crate::audio::filters::{filters_signature, AudioFilter, AudioFilterKind};

        assert_eq!(filters_signature(&[]), "none", "an empty chain is a no-op");

        let gain = AudioFilter::new(AudioFilterKind::Gain);
        let gate = AudioFilter::new(AudioFilterKind::NoiseGate);
        let ns = AudioFilter::new(AudioFilterKind::NoiseSuppression);

        // Order is significant: a gate before a gain is not the same as a gain before a gate
        assert_ne!(
            filters_signature(&[gain.clone(), gate.clone()]),
            filters_signature(&[gate.clone(), gain.clone()]),
            "chain order must be part of the signature"
        );

        // Parameter edits are visible to the encoder, so the new value is actually delivered
        let mut louder = gain.clone();
        louder.gain_db = 12.0;
        assert_ne!(filters_signature(&[gain.clone()]), filters_signature(&[louder]));

        // A bypass toggle changes the rendered audio, so it must change the signature too
        let mut off = ns.clone();
        off.enabled = false;
        assert_ne!(filters_signature(&[ns.clone()]), filters_signature(&[off]));

        // The identity is deliberately excluded: it is a UI handle, not an audio parameter
        let mut renamed = ns.clone();
        renamed.id = uuid::Uuid::new_v4();
        assert_eq!(filters_signature(&[ns]), filters_signature(&[renamed]));
    }

    #[test]
    fn test_slider_standardization_constants() {
        use crate::ui::dialogs::{
            AUDIO_FILTER_CARD_CONTENT_WIDTH, AUDIO_FILTER_CARD_WIDTH, SLIDER_PARAM_BOX_HEIGHT,
            SLIDER_PARAM_BOX_WIDTH, SLIDER_PARAM_FONT_SIZE, SLIDER_PARAM_LABEL_WIDTH,
            SLIDER_PARAM_RAIL_WIDTH, SLIDER_PARAM_RESET_WIDTH,
        };

        // Guaranteed dimensions for consistent 4-column studio slider parameter forms
        assert_eq!(AUDIO_FILTER_CARD_WIDTH, 496.0);
        assert_eq!(AUDIO_FILTER_CARD_CONTENT_WIDTH, 472.0);
        assert_eq!(SLIDER_PARAM_LABEL_WIDTH, 130.0);
        assert_eq!(SLIDER_PARAM_RAIL_WIDTH, 185.0);
        assert_eq!(SLIDER_PARAM_BOX_WIDTH, 64.0);
        assert_eq!(SLIDER_PARAM_BOX_HEIGHT, 22.0);
        assert_eq!(SLIDER_PARAM_RESET_WIDTH, 64.0);
        assert_eq!(SLIDER_PARAM_FONT_SIZE, 11.5);
    }
}
