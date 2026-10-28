pub mod scene;
pub mod source;

use crate::studio::scene::Scene;
use crate::studio::source::{SourceItem, SourceKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioState {
    pub scenes: Vec<Scene>,
    /// Scene currently edited in the left Preview viewport
    pub preview_scene_id: Uuid,
    /// Active scene routed to the right Program viewport and Recording Encoder
    pub program_scene_id: Uuid,
    /// Snapshot of the Program scene when operating in unlinked Studio Mode
    pub program_scene_snapshot: Scene,
    /// When true, edits in Preview immediately mirror to Program; when false, user clicks "Transition / Apply to Program"
    pub auto_sync_program: bool,
    pub transition_count: u64,
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

    /// Applies the current Preview scene composition to the Program output
    pub fn transition_preview_to_program(&mut self) {
        let preview_clone = self.preview_scene().clone();
        self.program_scene_id = preview_clone.id;
        self.program_scene_snapshot = preview_clone;
        self.transition_count += 1;
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

