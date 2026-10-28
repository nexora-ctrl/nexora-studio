pub mod scene;
pub mod source;

use crate::studio::scene::Scene;
use crate::studio::source::{SourceItem, SourceKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TransitionEffect {
    #[default]
    Cut,
    Fade,
}

impl TransitionEffect {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Cut => "Cut",
            Self::Fade => "Fade",
        }
    }
}

fn default_transition_duration_ms() -> u32 {
    350
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioState {
    pub scenes: Vec<Scene>,
    /// Scene currently edited in the left Preview viewport
    pub preview_scene_id: Uuid,
    /// Active scene routed to the right Program viewport and Recording Encoder
    pub program_scene_id: Uuid,
    /// Snapshot of the Program scene when operating in unlinked Studio Mode
    pub program_scene_snapshot: Scene,
    /// When true, edits in Preview immediately mirror to Program; when false, user clicks Transition
    pub auto_sync_program: bool,
    pub transition_count: u64,
    #[serde(default)]
    pub transition_effect: TransitionEffect,
    #[serde(default = "default_transition_duration_ms")]
    pub transition_duration_ms: u32,
    #[serde(skip)]
    pub transition_from_snapshot: Option<Scene>,
    #[serde(skip)]
    pub transition_start_instant: Option<std::time::Instant>,
}

impl Default for StudioState {
    fn default() -> Self {
        let main_scene = Scene::with_default_display("Scene 1");

        let mut coding_scene = Scene::new("Scene 2");
        coding_scene.add_source(SourceItem::new(
            "IDE Window",
            SourceKind::WindowCapture {
                window_hwnd: 0x0001_0420,
                window_title: "NEXORA Studio - Visual Studio Code".to_string(),
                executable_name: "Code.exe".to_string(),
                capture_cursor: true,
            },
        ));
        coding_scene.add_source(SourceItem::new(
            "Facecam Overlay",
            SourceKind::WebcamDevice {
                device_id: "CAM_0".to_string(),
                device_name: "Integrated Studio Camera".to_string(),
                resolution: (1280, 720),
            },
        ));

        let main_id = main_scene.id;
        let program_snapshot = main_scene.clone();

        Self {
            scenes: vec![main_scene, coding_scene],
            preview_scene_id: main_id,
            program_scene_id: main_id,
            program_scene_snapshot: program_snapshot,
            auto_sync_program: false,
            transition_count: 1,
            transition_effect: TransitionEffect::Cut,
            transition_duration_ms: 350,
            transition_from_snapshot: None,
            transition_start_instant: None,
        }
    }
}

impl StudioState {
    pub fn preview_scene_index(&self) -> usize {
        self.scenes
            .iter()
            .position(|s| s.id == self.preview_scene_id)
            .map(|i| i + 1)
            .unwrap_or(1)
    }

    pub fn program_scene_index(&self) -> usize {
        self.scenes
            .iter()
            .position(|s| s.id == self.program_scene_id)
            .map(|i| i + 1)
            .unwrap_or(1)
    }

    pub fn preview_top_label(&self) -> String {
        self.preview_scene()
            .top_display_label(self.preview_scene_index())
    }

    pub fn program_top_label(&self) -> String {
        self.program_scene()
            .top_display_label(self.program_scene_index())
    }

    pub fn preview_scene(&self) -> &Scene {
        self.scenes
            .iter()
            .find(|s| s.id == self.preview_scene_id)
            .unwrap_or(&self.scenes[0])
    }

    pub fn preview_scene_mut(&mut self) -> &mut Scene {
        let preview_id = self.preview_scene_id;
        if let Some(pos) = self.scenes.iter().position(|s| s.id == preview_id) {
            &mut self.scenes[pos]
        } else {
            &mut self.scenes[0]
        }
    }

    pub fn program_scene(&self) -> &Scene {
        if self.auto_sync_program {
            self.scenes
                .iter()
                .find(|s| s.id == self.program_scene_id)
                .unwrap_or(&self.scenes[0])
        } else {
            &self.program_scene_snapshot
        }
    }

    /// Applies the current Preview scene composition to the Program output using Cut or Fade transition
    pub fn transition_preview_to_program(&mut self) {
        let prev_program = self.program_scene().clone();
        if self.transition_effect == TransitionEffect::Fade {
            self.transition_from_snapshot = Some(prev_program.clone());
            self.transition_start_instant = Some(std::time::Instant::now());
        } else {
            self.transition_from_snapshot = None;
            self.transition_start_instant = None;
        }

        // For any MediaSource newly becoming active/visible in Program (and with restart_on_activate),
        // bump its restart_nonce so video and audio start together from 00:00 at the moment of transition.
        let preview_id = self.preview_scene_id;
        if let Some(pos) = self.scenes.iter().position(|s| s.id == preview_id) {
            for src in &mut self.scenes[pos].sources {
                if !src.visible {
                    continue;
                }
                let was_active_in_program = prev_program.sources.iter().any(|old| {
                    old.id == src.id
                        && old.visible
                        && match (&old.kind, &src.kind) {
                            (
                                SourceKind::MediaSource {
                                    file_path: old_p,
                                    is_playing: old_play,
                                    ..
                                },
                                SourceKind::MediaSource {
                                    file_path: new_p, ..
                                },
                            ) => old_p == new_p && *old_play,
                            _ => false,
                        }
                });

                if !was_active_in_program {
                    if let SourceKind::MediaSource {
                        restart_on_activate,
                        is_playing,
                        restart_nonce,
                        ..
                    } = &mut src.kind
                    {
                        *is_playing = true;
                        if *restart_on_activate {
                            *restart_nonce = restart_nonce.wrapping_add(1);
                        }
                    }
                }
            }
        }

        let preview_clone = self.preview_scene().clone();
        self.program_scene_id = preview_clone.id;
        self.program_scene_snapshot = preview_clone;
        self.transition_count += 1;
    }

    /// Synchronizes live MediaSource audio/transport controls (volume, mute, play/pause, loop, restart)
    /// from the scenes list into `program_scene_snapshot` if that source is already on Program.
    pub fn sync_media_transport_to_program(&mut self) {
        if self.auto_sync_program {
            self.sync_if_linked();
            return;
        }
        for scene in &self.scenes {
            for src in &scene.sources {
                if let SourceKind::MediaSource {
                    loop_playback,
                    volume,
                    muted,
                    is_playing,
                    restart_nonce,
                    ..
                } = &src.kind
                {
                    if let Some(prog_src) = self
                        .program_scene_snapshot
                        .sources
                        .iter_mut()
                        .find(|ps| ps.id == src.id)
                    {
                        if let SourceKind::MediaSource {
                            loop_playback: p_loop,
                            volume: p_vol,
                            muted: p_muted,
                            is_playing: p_playing,
                            restart_nonce: p_nonce,
                            ..
                        } = &mut prog_src.kind
                        {
                            *p_loop = *loop_playback;
                            *p_vol = *volume;
                            *p_muted = *muted;
                            *p_playing = *is_playing;
                            *p_nonce = *restart_nonce;
                        }
                    }
                }
            }
        }
    }

    /// Returns `Some(progress)` in `0.0..1.0` if a Fade transition is currently animating
    pub fn active_fade_progress(&mut self) -> Option<f32> {
        if self.transition_effect != TransitionEffect::Fade {
            self.transition_from_snapshot = None;
            self.transition_start_instant = None;
            return None;
        }
        if let Some(start) = self.transition_start_instant {
            let dur = self.transition_duration_ms.clamp(100, 2000) as f32 / 1000.0;
            let p = start.elapsed().as_secs_f32() / dur;
            if p < 1.0 {
                return Some(p.clamp(0.0, 1.0));
            }
            self.transition_from_snapshot = None;
            self.transition_start_instant = None;
        }
        None
    }

    /// Synchronize snapshot if auto_sync_program is enabled
    pub fn sync_if_linked(&mut self) {
        if self.auto_sync_program {
            let preview_clone = self.preview_scene().clone();
            self.program_scene_id = preview_clone.id;
            self.program_scene_snapshot = preview_clone;
        }
    }

    pub fn add_scene(&mut self, name: String) -> Uuid {
        let scene = Scene::with_default_display(name);
        let id = scene.id;
        self.scenes.push(scene);
        self.preview_scene_id = id;
        self.sync_if_linked();
        id
    }

    pub fn duplicate_preview_scene(&mut self) -> Uuid {
        let base = self.preview_scene().clone();
        let copy_name = format!("{} (Copy)", base.name);
        let duplicated = base.duplicate(copy_name);
        let id = duplicated.id;
        self.scenes.push(duplicated);
        self.preview_scene_id = id;
        self.sync_if_linked();
        id
    }

    pub fn delete_scene(&mut self, scene_id: Uuid) -> bool {
        if self.scenes.len() <= 1 {
            return false; // Always keep at least one scene
        }
        if let Some(idx) = self.scenes.iter().position(|s| s.id == scene_id) {
            self.scenes.remove(idx);
            if self.preview_scene_id == scene_id {
                self.preview_scene_id = self.scenes[0].id;
            }
            if self.program_scene_id == scene_id {
                self.program_scene_id = self.scenes[0].id;
                self.program_scene_snapshot = self.scenes[0].clone();
            }
            return true;
        }
        false
    }

    pub fn has_pending_preview_changes(&self) -> bool {
        if self.auto_sync_program {
            return false;
        }
        let prev = self.preview_scene();
        let prog = &self.program_scene_snapshot;
        if prev.id != prog.id || prev.name != prog.name || prev.sources.len() != prog.sources.len()
        {
            return true;
        }
        for (a, b) in prev.sources.iter().zip(prog.sources.iter()) {
            if a.id != b.id
                || a.name != b.name
                || a.visible != b.visible
                || a.kind != b.kind
                || a.transform != b.transform
            {
                return true;
            }
        }
        false
    }
}

