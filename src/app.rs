use crate::audio::AudioMixerState;
use crate::capture::{CaptureCatalog, CaptureRequestTarget, LiveCaptureEngine};
use crate::encoder::EncoderConfig;
use crate::media_library::MediaLibrary;
use crate::project::{NexoraProject, ProjectManager};
use crate::recording::RecordingController;
use crate::settings::{AppSettings, ScreenLayoutPreset};
use crate::studio::source::{SourceItem, SourceKind};
use crate::studio::StudioState;
use crate::ui::theme::{apply_studio_theme, ThemePalette};
use directories::ProjectDirs;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickScreenTemplate {
    FullScreenCamBottomRight,
    FullScreenCamBottomLeft,
    FullScreenCamTopRight,
    SideBySideSplit,
    FullScreenOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreviewDragMode {
    #[default]
    None,
    Move,
    ResizeTopLeft,
    ResizeTopRight,
    ResizeBottomLeft,
    ResizeBottomRight,
    ResizeTop,
    ResizeBottom,
    ResizeLeft,
    ResizeRight,
    CropTop,
    CropBottom,
    CropLeft,
    CropRight,
}

#[derive(Clone)]
pub struct UndoSnapshot {
    pub studio: StudioState,
    pub audio_mixer: AudioMixerState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectorTarget {
    pub is_program: bool,
    pub is_fullscreen: bool,
    pub label: String,
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub snap_frames_remaining: u8,
}

pub struct NexoraApp {
    pub settings: AppSettings,
    pub studio: StudioState,
    pub capture_catalog: CaptureCatalog,
    pub capture_engine: LiveCaptureEngine,
    pub audio_mixer: AudioMixerState,
    pub recording: RecordingController,
    pub encoder_config: EncoderConfig,
    pub project_manager: ProjectManager,
    pub media_library: MediaLibrary,
    pub show_settings_dialog: bool,
    pub show_rename_scene_dialog: bool,
    pub rename_scene_buffer: String,
    pub show_source_properties_dialog: bool,
    pub show_rename_source_dialog: bool,
    pub rename_source_buffer: String,
    pub show_save_layout_dialog: bool,
    pub save_layout_name_buffer: String,
    pub show_projector_window: bool,
    pub projector_target_label: String,
    pub projector_target: Option<ProjectorTarget>,
    pub screenshot_notify: Option<(String, std::time::Instant)>,
    pub preview_drag_mode: PreviewDragMode,
    pub preview_crop_mode: bool,
    pub media_audio_engine: crate::platform::MediaAudioEngine,
    pub brand_icon_texture: Option<egui::TextureHandle>,
    undo_stack: Vec<UndoSnapshot>,
    redo_stack: Vec<UndoSnapshot>,
    committed_snapshot: UndoSnapshot,
    committed_hash: u64,
    last_live_hash: u64,
    last_change_time: f64,
    last_autosave_hash: u64,
    last_autosave_check: f64,
}

impl NexoraApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::ui::icons::install_tabler_icons(&cc.egui_ctx);
        let mut settings = AppSettings::load_or_default();
        apply_studio_theme(&cc.egui_ctx, settings.theme);

        let mut media_library = MediaLibrary::default();
        media_library.scan_directory(&settings.output_directory);

        let capture_catalog = CaptureCatalog::default();
        let mut studio = StudioState::default();
        let mut audio_mixer = AudioMixerState::default();
        let mut encoder_config = EncoderConfig::default();
        let mut project_manager = ProjectManager::new();

        // Try to restore the last saved studio session (scenes, sources, screen/camera layout, audio mixer)
        let snapshot_path = Self::default_project_snapshot_path();
        let loaded_from_disk = if let Ok(proj) = NexoraProject::load_from_file(&snapshot_path) {
            project_manager.project_name = proj.project_name;
            project_manager.current_project_path = Some(snapshot_path.clone());
            studio = proj.studio;
            audio_mixer = proj.audio_mixer;
            encoder_config = proj.encoder_config;
            true
        } else {
            false
        };

        Self::reconcile_sources_with_catalog(&mut studio, &capture_catalog, !loaded_from_disk);
        audio_mixer.refresh_devices_from_system();

        // Reconcile selected GPU / VGA adapter with detected system GPUs
        if !capture_catalog.gpus.is_empty() {
            if !settings.preferred_gpu_name.is_empty() {
                if let Some(matched) = capture_catalog
                    .gpus
                    .iter()
                    .find(|g| g.name.eq_ignore_ascii_case(&settings.preferred_gpu_name))
                {
                    settings.preferred_gpu_index = Some(matched.index);
                    settings.preferred_gpu_name = matched.name.clone();
                }
            }
            let idx_valid = settings
                .preferred_gpu_index
                .map(|idx| capture_catalog.gpus.iter().any(|g| g.index == idx))
                .unwrap_or(false);
            if !idx_valid {
                if let Some(first_gpu) = capture_catalog.gpus.first() {
                    settings.preferred_gpu_index = Some(first_gpu.index);
                    settings.preferred_gpu_name = first_gpu.name.clone();
                }
            } else if settings.preferred_gpu_name.is_empty() {
                if let Some(idx) = settings.preferred_gpu_index {
                    if let Some(g) = capture_catalog.gpus.iter().find(|g| g.index == idx) {
                        settings.preferred_gpu_name = g.name.clone();
                    }
                }
            }
        }

        // Ensure Default Studio Layout preset exists if user has no presets yet
        if settings.saved_layout_presets.is_empty() {
            settings.saved_layout_presets.push(ScreenLayoutPreset {
                name: "Default Studio Layout".to_string(),
                updated_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
                panels: settings.panels.clone(),
                dock_height: settings.dock_height,
                preview_fit_to_window: settings.preview_fit_to_window,
                preview_zoom: settings.preview_zoom,
                show_safe_margins: settings.show_safe_margins,
                video_preset: settings.video_preset,
                bitrate_preset: settings.bitrate_preset,
                studio: studio.clone(),
                audio_mixer: audio_mixer.clone(),
            });
        }

        let brand_icon_texture =
            crate::ui::icons::decode_embedded_ico_rgba(48).map(|(w, h, rgba)| {
                let color_image =
                    egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                cc.egui_ctx.load_texture(
                    "nexora_brand_icon_ico",
                    color_image,
                    egui::TextureOptions::LINEAR,
                )
            });

        let initial_snapshot = UndoSnapshot {
            studio: studio.clone(),
            audio_mixer: audio_mixer.clone(),
        };

        let mut app = Self {
            settings,
            studio,
            capture_catalog,
            capture_engine: LiveCaptureEngine::new(),
            audio_mixer,
            recording: RecordingController::default(),
            encoder_config,
            project_manager,
            media_library,
            show_settings_dialog: false,
            show_rename_scene_dialog: false,
            rename_scene_buffer: String::new(),
            show_source_properties_dialog: false,
            show_rename_source_dialog: false,
            rename_source_buffer: String::new(),
            show_save_layout_dialog: false,
            save_layout_name_buffer: String::new(),
            show_projector_window: false,
            projector_target_label: String::new(),
            projector_target: None,
            screenshot_notify: None,
            preview_drag_mode: PreviewDragMode::None,
            preview_crop_mode: false,
            media_audio_engine: crate::platform::MediaAudioEngine::default(),
            brand_icon_texture,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            committed_snapshot: initial_snapshot,
            committed_hash: 0,
            last_live_hash: 0,
            last_change_time: 0.0,
            last_autosave_hash: 0,
            last_autosave_check: 0.0,
        };

        if loaded_from_disk {
            app.recording.last_message =
                "Restored last saved Screen, Layout & Studio settings".to_string();
        } else {
            app.save_all_state();
        }
        let u_hash = app.compute_undoable_hash();
        app.committed_hash = u_hash;
        app.last_live_hash = u_hash;
        app.last_autosave_hash = app.compute_state_hash();
        app
    }

    /// Reconciles saved or default sources with live connected monitors, windows, and cameras
    pub fn reconcile_sources_with_catalog(
        studio: &mut StudioState,
        capture_catalog: &CaptureCatalog,
        is_fresh_default: bool,
    ) {
        let primary_disp = capture_catalog.displays.first();
        let first_win = capture_catalog.windows.first();
        let first_cam = capture_catalog
            .cameras
            .iter()
            .find(|c| c.is_online)
            .or_else(|| capture_catalog.cameras.first());

        for scene in &mut studio.scenes {
            for src in &mut scene.sources {
                match &mut src.kind {
                    SourceKind::DisplayCapture {
                        monitor_id,
                        monitor_name,
                        resolution,
                        ..
                    } => {
                        if let Some(matched) = capture_catalog.find_display(monitor_id).or_else(|| {
                            capture_catalog
                                .displays
                                .iter()
                                .find(|d| d.name == *monitor_name)
                        }) {
                            *monitor_id = matched.id.clone();
                            *monitor_name = matched.name.clone();
                            *resolution = (matched.width, matched.height);
                        } else if let Some(primary) = primary_disp {
                            *monitor_id = primary.id.clone();
                            *monitor_name = primary.name.clone();
                            *resolution = (primary.width, primary.height);
                        }
                    }
                    SourceKind::WindowCapture {
                        window_hwnd,
                        window_title,
                        executable_name,
                        ..
                    } => {
                        let hwnd_alive = capture_catalog
                            .windows
                            .iter()
                            .any(|w| w.hwnd == *window_hwnd);
                        if !hwnd_alive {
                            // Try matching by executable + title, or by executable name alone
                            let matched = capture_catalog
                                .windows
                                .iter()
                                .find(|w| {
                                    w.executable.eq_ignore_ascii_case(executable_name)
                                        && w.title == *window_title
                                })
                                .or_else(|| {
                                    capture_catalog
                                        .windows
                                        .iter()
                                        .find(|w| w.executable.eq_ignore_ascii_case(executable_name))
                                });

                            if let Some(w) = matched {
                                *window_hwnd = w.hwnd;
                                *window_title = w.title.clone();
                                *executable_name = w.executable.clone();
                            } else if is_fresh_default {
                                if let Some(w) = first_win {
                                    *window_hwnd = w.hwnd;
                                    *window_title = w.title.clone();
                                    *executable_name = w.executable.clone();
                                }
                            }
                        }
                    }
                    SourceKind::WebcamDevice {
                        device_id,
                        device_name,
                        ..
                    } => {
                        let matched = capture_catalog
                            .cameras
                            .iter()
                            .find(|c| c.id == *device_id)
                            .or_else(|| {
                                capture_catalog
                                    .cameras
                                    .iter()
                                    .find(|c| c.name == *device_name)
                            });
                        if let Some(cam) = matched {
                            *device_id = cam.id.clone();
                            *device_name = cam.name.clone();
                        } else if let Some(cam) = first_cam {
                            *device_id = cam.id.clone();
                            *device_name = cam.name.clone();
                        }
                    }
                    _ => {}
                }
            }
        }

        if let Some(prog_scene) = studio
            .scenes
            .iter()
            .find(|s| s.id == studio.program_scene_id)
            .cloned()
        {
            studio.program_scene_snapshot = prog_scene;
        } else {
            studio.program_scene_snapshot = studio.preview_scene().clone();
        }
    }

    pub fn sync_active_capture_requests(&mut self) {
        let mut set = HashSet::new();
        let mut audio_specs = Vec::new();
        let mut seen_audio_keys = HashSet::new();

        // Collect active video/image capture targets from both Preview and Program,
        // but ONLY output Media Source audio when the source is active & visible in PROGRAM!
        for (is_program_scene, scene) in [
            (true, self.studio.program_scene()),
            (false, self.studio.preview_scene()),
        ] {
            for src in &scene.sources {
                if !src.visible {
                    continue;
                }
                match &src.kind {
                    SourceKind::DisplayCapture {
                        monitor_id,
                        capture_cursor,
                        ..
                    } => {
                        if let Some(disp) = self.capture_catalog.find_display(monitor_id) {
                            set.insert(CaptureRequestTarget::Display {
                                monitor_id: disp.id.clone(),
                                left: disp.left,
                                top: disp.top,
                                width: disp.width,
                                height: disp.height,
                                capture_cursor: *capture_cursor,
                            });
                        }
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
                        if *window_hwnd != 0 {
                            set.insert(CaptureRequestTarget::Window {
                                hwnd: *window_hwnd,
                                capture_cursor: *capture_cursor,
                            });
                        }
                    }
                    SourceKind::WebcamDevice {
                        device_id,
                        resolution,
                        ..
                    } => {
                        set.insert(CaptureRequestTarget::Camera {
                            device_id: device_id.clone(),
                            resolution: *resolution,
                        });
                    }
                    SourceKind::MediaSource {
                        file_path,
                        loop_playback,
                        use_hardware_decoding,
                        is_playing,
                        volume,
                        muted,
                        restart_nonce,
                        ..
                    } => {
                        if !file_path.trim().is_empty() {
                            if !SourceKind::is_audio_only_media(file_path) {
                                let (max_w, max_h) = self.settings.media_decode_scale.max_dimensions();
                                set.insert(CaptureRequestTarget::MediaFile {
                                    file_path: file_path.clone(),
                                    loop_playback: *loop_playback,
                                    is_playing: *is_playing,
                                    use_hardware_decoding: *use_hardware_decoding,
                                    gpu_index: self.settings.preferred_gpu_index.unwrap_or(0),
                                    max_decode_width: max_w,
                                    max_decode_height: max_h,
                                    restart_nonce: *restart_nonce,
                                });
                            }
                            if is_program_scene {
                                let id_key = format!("nx_med_{}", src.id.simple());
                                if seen_audio_keys.insert(id_key.clone()) {
                                    audio_specs.push(crate::platform::ActiveMediaAudioSpec {
                                        id_key,
                                        file_path: file_path.clone(),
                                        loop_playback: *loop_playback,
                                        is_playing: *is_playing,
                                        volume: *volume,
                                        muted: *muted,
                                        restart_nonce: *restart_nonce,
                                    });
                                }
                            }
                        }
                    }
                    SourceKind::ImageSource { file_path, .. } => {
                        if !file_path.trim().is_empty() {
                            set.insert(CaptureRequestTarget::ImageFile {
                                file_path: file_path.clone(),
                            });
                        }
                    }
                    SourceKind::ImageSlideShow { files, .. } => {
                        for f in files {
                            if !f.trim().is_empty() {
                                set.insert(CaptureRequestTarget::ImageFile {
                                    file_path: f.clone(),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        self.capture_engine
            .set_active_requests(set.into_iter().collect());
        self.media_audio_engine.sync_active_media(&audio_specs);
    }

    pub fn new_project(&mut self) {
        self.studio = StudioState::default();
        Self::reconcile_sources_with_catalog(&mut self.studio, &self.capture_catalog, true);
        self.audio_mixer = AudioMixerState::default();
        self.project_manager.project_name = "Untitled Studio Project".to_string();
        self.save_all_state();
        self.recording.last_message = "Created new Studio project (Auto-saved)".to_string();
    }

    fn default_project_snapshot_path() -> PathBuf {
        if let Some(dirs) = ProjectDirs::from("com", "Nexora", "NexoraStudio") {
            dirs.config_dir().join("default_project.nexora.json")
        } else {
            PathBuf::from("default_project.nexora.json")
        }
    }

    /// Saves all application settings, window/dock layout, scenes, sources, and audio mixer state
    pub fn save_all_state(&mut self) {
        let _ = self.settings.save();
        let path = Self::default_project_snapshot_path();
        let mut proj = NexoraProject {
            schema_version: 1,
            project_name: self.project_manager.project_name.clone(),
            updated_at: chrono::Local::now(),
            video_preset: self.settings.video_preset,
            bitrate_preset: self.settings.bitrate_preset,
            studio: self.studio.clone(),
            audio_mixer: self.audio_mixer.clone(),
            encoder_config: self.encoder_config.clone(),
        };
        if proj.save_to_file(&path).is_ok() {
            self.project_manager.current_project_path = Some(path);
        }
        self.last_autosave_hash = self.compute_state_hash();
    }

    pub fn quick_save_project(&mut self) {
        self.save_all_state();
        let path = Self::default_project_snapshot_path();
        self.recording.last_message =
            format!("All screen, layout & studio settings saved -> {}", path.display());
    }

    pub fn quick_load_project(&mut self) {
        let path = Self::default_project_snapshot_path();
        match NexoraProject::load_from_file(&path) {
            Ok(proj) => {
                self.project_manager.project_name = proj.project_name;
                self.settings.video_preset = proj.video_preset;
                self.settings.bitrate_preset = proj.bitrate_preset;
                self.studio = proj.studio;
                self.audio_mixer = proj.audio_mixer;
                self.encoder_config = proj.encoder_config;
                Self::reconcile_sources_with_catalog(
                    &mut self.studio,
                    &self.capture_catalog,
                    false,
                );
                self.audio_mixer.refresh_devices_from_system();
                let _ = self.settings.save();
                self.last_autosave_hash = self.compute_state_hash();
                self.recording.last_message =
                    format!("Project & layout loaded <- {}", path.display());
            }
            Err(err) => {
                self.recording.last_message = format!("Project load error: {}", err);
            }
        }
    }

    /// Saves or updates a named Screen & Layout Preset
    pub fn save_layout_preset(&mut self, name: &str) {
        let clean_name = name.trim();
        if clean_name.is_empty() {
            return;
        }
        let preset = ScreenLayoutPreset {
            name: clean_name.to_string(),
            updated_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
            panels: self.settings.panels.clone(),
            dock_height: self.settings.dock_height,
            preview_fit_to_window: self.settings.preview_fit_to_window,
            preview_zoom: self.settings.preview_zoom,
            show_safe_margins: self.settings.show_safe_margins,
            video_preset: self.settings.video_preset,
            bitrate_preset: self.settings.bitrate_preset,
            studio: self.studio.clone(),
            audio_mixer: self.audio_mixer.clone(),
        };

        if let Some(existing) = self
            .settings
            .saved_layout_presets
            .iter_mut()
            .find(|p| p.name.eq_ignore_ascii_case(clean_name))
        {
            *existing = preset;
        } else {
            self.settings.saved_layout_presets.push(preset);
        }
        self.settings.active_layout_name = clean_name.to_string();
        self.save_all_state();
        self.recording.last_message =
            format!("Saved Screen & Layout Preset: '{}'", clean_name);
    }

    /// Loads a saved Screen & Layout Preset by index
    pub fn load_layout_preset(&mut self, index: usize) {
        if let Some(preset) = self.settings.saved_layout_presets.get(index).cloned() {
            self.settings.panels = preset.panels;
            self.settings.dock_height = preset.dock_height.clamp(210.0, 380.0);
            self.settings.preview_fit_to_window = preset.preview_fit_to_window;
            self.settings.preview_zoom = preset.preview_zoom;
            self.settings.show_safe_margins = preset.show_safe_margins;
            self.settings.video_preset = preset.video_preset;
            self.settings.bitrate_preset = preset.bitrate_preset;
            self.settings.active_layout_name = preset.name.clone();
            self.studio = preset.studio;
            self.audio_mixer = preset.audio_mixer;

            Self::reconcile_sources_with_catalog(&mut self.studio, &self.capture_catalog, false);
            self.audio_mixer.refresh_devices_from_system();
            self.save_all_state();
            self.recording.last_message =
                format!("Loaded Screen & Layout Preset: '{}'", preset.name);
        }
    }

    /// Deletes a saved Screen & Layout Preset by index
    pub fn delete_layout_preset(&mut self, index: usize) {
        if index < self.settings.saved_layout_presets.len()
            && self.settings.saved_layout_presets.len() > 1
        {
            let removed = self.settings.saved_layout_presets.remove(index);
            if self.settings.active_layout_name == removed.name {
                if let Some(first) = self.settings.saved_layout_presets.first() {
                    self.settings.active_layout_name = first.name.clone();
                }
            }
            self.save_all_state();
            self.recording.last_message =
                format!("Deleted Layout Preset: '{}'", removed.name);
        }
    }

    /// Applies a 1-click Screen + Camera arrangement template to the active Preview scene
    pub fn apply_quick_screen_template(&mut self, template: QuickScreenTemplate) {
        let primary_disp = self.capture_catalog.displays.first().cloned();
        let first_cam = self
            .capture_catalog
            .cameras
            .iter()
            .find(|c| c.is_online)
            .or_else(|| self.capture_catalog.cameras.first())
            .cloned();

        let scene = self.studio.preview_scene_mut();

        // Ensure Display source exists
        let has_disp = scene
            .sources
            .iter()
            .any(|s| matches!(s.kind, SourceKind::DisplayCapture { .. }));
        if !has_disp {
            let (mon_id, mon_name, res) = if let Some(d) = primary_disp {
                (d.id, d.name, (d.width, d.height))
            } else {
                (
                    "MONITOR_0".to_string(),
                    "Primary Monitor (Display 1)".to_string(),
                    (1920, 1080),
                )
            };
            scene.sources.insert(
                0,
                SourceItem::new(
                    "Primary Display",
                    SourceKind::DisplayCapture {
                        monitor_id: mon_id,
                        monitor_name: mon_name,
                        resolution: res,
                        capture_cursor: true,
                    },
                ),
            );
        }

        // Ensure Webcam source exists when needed
        if template != QuickScreenTemplate::FullScreenOnly {
            let has_cam = scene
                .sources
                .iter()
                .any(|s| matches!(s.kind, SourceKind::WebcamDevice { .. }));
            if !has_cam {
                let (cam_id, cam_name) = if let Some(c) = first_cam {
                    (c.id, c.name)
                } else {
                    ("CAM_0".to_string(), "Integrated Studio Camera".to_string())
                };
                scene.add_source(SourceItem::new(
                    "Video Capture Device",
                    SourceKind::WebcamDevice {
                        device_id: cam_id,
                        device_name: cam_name,
                        resolution: (1280, 720),
                    },
                ));
            }
        }

        for src in &mut scene.sources {
            match &src.kind {
                SourceKind::DisplayCapture { .. } | SourceKind::WindowCapture { .. } => {
                    src.visible = true;
                    match template {
                        QuickScreenTemplate::SideBySideSplit => {
                            src.transform.pos_x = 0.01;
                            src.transform.pos_y = 0.12;
                            src.transform.scale = 0.48;
                        }
                        _ => {
                            src.transform.reset_full_canvas();
                        }
                    }
                }
                SourceKind::WebcamDevice { .. } => match template {
                    QuickScreenTemplate::FullScreenCamBottomRight => {
                        src.visible = true;
                        src.transform.preset_pip_bottom_right();
                    }
                    QuickScreenTemplate::FullScreenCamBottomLeft => {
                        src.visible = true;
                        src.transform.preset_pip_bottom_left();
                    }
                    QuickScreenTemplate::FullScreenCamTopRight => {
                        src.visible = true;
                        src.transform.preset_pip_top_right();
                    }
                    QuickScreenTemplate::SideBySideSplit => {
                        src.visible = true;
                        src.transform.pos_x = 0.51;
                        src.transform.pos_y = 0.12;
                        src.transform.scale = 0.48;
                    }
                    QuickScreenTemplate::FullScreenOnly => {
                        src.visible = false;
                    }
                },
                _ => {}
            }
        }

        self.studio.sync_if_linked();
        self.save_all_state();
        self.recording.last_message = "Applied Screen & Camera Layout template (Auto-saved)".to_string();
    }

    fn compute_undoable_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        if let Ok(s) = serde_json::to_string(&self.studio) {
            s.hash(&mut hasher);
        }
        if let Ok(s) = serde_json::to_string(&self.audio_mixer) {
            s.hash(&mut hasher);
        }
        hasher.finish()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty() || self.compute_undoable_hash() != self.committed_hash
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
            + usize::from(self.compute_undoable_hash() != self.committed_hash)
    }

    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Reverts the last change made to Scenes, Sources, Transform/Crop, or Audio Mixer (`Ctrl+Z`)
    pub fn undo(&mut self) {
        let cur_hash = self.compute_undoable_hash();
        let current_snap = UndoSnapshot {
            studio: self.studio.clone(),
            audio_mixer: self.audio_mixer.clone(),
        };

        if cur_hash != self.committed_hash {
            // Revert uncommitted change directly back to `committed_snapshot`
            self.redo_stack.push(current_snap);
            self.studio = self.committed_snapshot.studio.clone();
            self.audio_mixer = self.committed_snapshot.audio_mixer.clone();
            self.audio_mixer.refresh_devices_from_system();
            let h = self.compute_undoable_hash();
            self.committed_hash = h;
            self.last_live_hash = h;
            self.recording.last_message =
                format!("Undo applied ({} step(s) remaining)", self.undo_stack.len());
            return;
        }

        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(current_snap);
            self.studio = prev.studio.clone();
            self.audio_mixer = prev.audio_mixer.clone();
            self.audio_mixer.refresh_devices_from_system();
            self.committed_snapshot = prev;
            let h = self.compute_undoable_hash();
            self.committed_hash = h;
            self.last_live_hash = h;
            self.recording.last_message =
                format!("Undo applied ({} step(s) remaining)", self.undo_stack.len());
        }
    }

    /// Re-applies an undone change (`Ctrl+Y` or `Ctrl+Shift+Z`)
    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            let current_snap = UndoSnapshot {
                studio: self.studio.clone(),
                audio_mixer: self.audio_mixer.clone(),
            };
            if self.undo_stack.len() >= 50 {
                self.undo_stack.remove(0);
            }
            self.undo_stack.push(current_snap);
            self.studio = next.studio.clone();
            self.audio_mixer = next.audio_mixer.clone();
            self.audio_mixer.refresh_devices_from_system();
            self.committed_snapshot = next;
            let h = self.compute_undoable_hash();
            self.committed_hash = h;
            self.last_live_hash = h;
            self.recording.last_message =
                format!("Redo applied ({} step(s) remaining)", self.redo_stack.len());
        }
    }

    fn compute_state_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        if let Ok(s) = serde_json::to_string(&self.settings) {
            s.hash(&mut hasher);
        }
        if let Ok(s) = serde_json::to_string(&self.studio) {
            s.hash(&mut hasher);
        }
        if let Ok(s) = serde_json::to_string(&self.audio_mixer) {
            s.hash(&mut hasher);
        }
        hasher.finish()
    }
}

impl eframe::App for NexoraApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let time_sec = ctx.input(|i| i.time);
        self.audio_mixer.tick_meters(time_sec);
        self.sync_active_capture_requests();

        // Track window size, position, Maximized state, and keyboard shortcuts (Ctrl+S, Ctrl+Z, Ctrl+Y)
        ctx.input(|i| {
            let vp = i.viewport();
            if let Some(max) = vp.maximized {
                self.settings.window_maximized = max;
            }
            let minimized = vp.minimized.unwrap_or(false);
            if !self.settings.window_maximized && !minimized {
                if let Some(inner) = vp.inner_rect {
                    let w = inner.width();
                    let h = inner.height();
                    if w >= 800.0 && h >= 500.0 {
                        self.settings.window_size = (w, h);
                    }
                }
                if let Some(outer) = vp.outer_rect {
                    if outer.min.x > -3000.0 && outer.min.y > -3000.0 {
                        self.settings.window_pos = Some((outer.min.x, outer.min.y));
                    }
                }
            }
        });

        let (ctrl_s, ctrl_z, ctrl_y) = ctx.input(|i| {
            let s = i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::S);
            let z = i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Z);
            let y = (i.modifiers.ctrl && i.key_pressed(egui::Key::Y))
                || (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Z));
            (s, z, y)
        });
        if ctrl_s {
            let active_name = self.settings.active_layout_name.clone();
            self.save_layout_preset(&active_name);
        }
        if ctrl_z {
            self.undo();
        } else if ctrl_y {
            self.redo();
        }

        // 1. Top Bar (Brand, Menus, Undo/Redo, Studio Mode switch, Live Status Pill, Theme Toggle, Settings)
        crate::ui::top_bar::render_top_bar(ctx, self);

        // 2. Bottom Status Bar (Capture, Video Preset, Audio Status, Session Telemetry)
        crate::ui::status_bar::render_status_bar(ctx, self);

        // 3. Resizable Bottom Studio Dock (Scenes | Sources | Audio Mixer | Recording Controls)
        crate::ui::dock::render_bottom_studio_dock(ctx, self);

        // 4. Optional Right Side Drawer for Recording Library
        crate::ui::dialogs::render_library_drawer_if_open(ctx, self);

        // 5. Central Workspace (Dual Preview + Program Studio Viewports)
        let p = ThemePalette::for_mode(self.settings.theme);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(p.bg_app)
                    .inner_margin(egui::Margin::same(8)),
            )
            .show(ctx, |ui| {
                crate::ui::workspace::render_preview_workspace(ui, self, time_sec);
            });

        // 6. Floating Modal Dialogs (Settings, Rename Scene, Save Screen & Layout Preset)
        crate::ui::dialogs::render_dialogs(ctx, self);

        // 7. Automatic Undo History Commit (settled after mouse release) & Background Auto-Save
        let live_u_hash = self.compute_undoable_hash();
        if live_u_hash != self.last_live_hash {
            self.last_live_hash = live_u_hash;
            self.last_change_time = time_sec;
        }
        let pointer_down = ctx.input(|i| i.pointer.any_down());
        if !pointer_down
            && self.preview_drag_mode == PreviewDragMode::None
            && live_u_hash != self.committed_hash
            && (time_sec - self.last_change_time) >= 0.25
        {
            if self.undo_stack.len() >= 50 {
                self.undo_stack.remove(0);
            }
            self.undo_stack.push(self.committed_snapshot.clone());
            self.redo_stack.clear();
            self.committed_snapshot = UndoSnapshot {
                studio: self.studio.clone(),
                audio_mixer: self.audio_mixer.clone(),
            };
            self.committed_hash = live_u_hash;
        }

        if time_sec - self.last_autosave_check >= 1.5 {
            self.last_autosave_check = time_sec;
            let current_hash = self.compute_state_hash();
            if current_hash != self.last_autosave_hash {
                self.save_all_state();
            }
        }

        // Request smooth repaint for live audio meters, timer, and canvas telemetry
        ctx.request_repaint_after(Duration::from_millis(33));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.media_audio_engine.shutdown_all();
        // Update the active layout preset with the final state and save all settings & project files
        let active_name = self.settings.active_layout_name.clone();
        self.save_layout_preset(&active_name);
    }
}

