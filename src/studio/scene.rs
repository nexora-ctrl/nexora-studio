use crate::studio::source::{SourceItem, SourceKind};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: Uuid,
    pub name: String,
    pub sources: Vec<SourceItem>,
    pub selected_source_id: Option<Uuid>,
}

impl Scene {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            sources: Vec::new(),
            selected_source_id: None,
        }
    }

    pub fn with_default_display(name: impl Into<String>) -> Self {
        let mut scene = Self::new(name);
        let default_display = SourceItem::new(
            "Primary Display",
            SourceKind::DisplayCapture {
                monitor_id: "MONITOR_0".to_string(),
                monitor_name: "Primary Monitor (Display 1)".to_string(),
                resolution: (1920, 1080),
                capture_cursor: true,
            },
        );
        scene.selected_source_id = Some(default_display.id);
        scene.sources.push(default_display);
        scene
    }

    /// Generates an automatic descriptive label based on the scene's active sources
    pub fn auto_source_descriptor(&self) -> &'static str {
        let visible: Vec<&SourceItem> = self.sources.iter().filter(|s| s.visible).collect();
        if visible.is_empty() {
            return "Empty Scene";
        }
        let has_display = visible
            .iter()
            .any(|s| matches!(s.kind, SourceKind::DisplayCapture { .. }));
        let has_window = visible
            .iter()
            .any(|s| matches!(s.kind, SourceKind::WindowCapture { .. }));
        let has_cam = visible
            .iter()
            .any(|s| matches!(s.kind, SourceKind::WebcamDevice { .. }));
        let has_img = visible
            .iter()
            .any(|s| matches!(s.kind, SourceKind::ImageSource { .. }));

        match (has_display, has_window, has_cam, has_img) {
            (true, false, true, _) => "Desktop + Cam PiP",
            (true, false, false, _) => "Desktop Full",
            (false, true, true, _) => "App + Cam PiP",
            (false, true, false, _) => "Window Capture",
            (false, false, true, _) => "Camera Full",
            (false, false, false, true) => "Image Overlay",
            _ => "Multi-Source",
        }
    }

    /// Returns the automatic top label (e.g. "Scene 1 - Desktop Full")
    pub fn top_display_label(&self, index_one_based: usize) -> String {
        let trimmed = self.name.trim();
        let default_prefix = format!("Scene {}", index_one_based);
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case(&default_prefix) {
            format!("Scene {} - {}", index_one_based, self.auto_source_descriptor())
        } else if trimmed.contains(" - ") {
            trimmed.to_string()
        } else {
            format!("{} - {}", trimmed, self.auto_source_descriptor())
        }
    }

    pub fn duplicate(&self, new_name: String) -> Self {
        let mut cloned_sources = Vec::with_capacity(self.sources.len());
        let mut selected = None;
        for src in &self.sources {
            let mut copy = src.clone();
            copy.id = Uuid::new_v4();
            if self.selected_source_id == Some(src.id) {
                selected = Some(copy.id);
            }
            cloned_sources.push(copy);
        }
        Self {
            id: Uuid::new_v4(),
            name: new_name,
            sources: cloned_sources,
            selected_source_id: selected,
        }
    }

    pub fn selected_source(&self) -> Option<&SourceItem> {
        let id = self.selected_source_id?;
        self.sources.iter().find(|s| s.id == id)
    }

    pub fn selected_source_mut(&mut self) -> Option<&mut SourceItem> {
        let id = self.selected_source_id?;
        self.sources.iter_mut().find(|s| s.id == id)
    }

    pub fn generate_unique_source_name(&self, prefix: &str) -> String {
        let existing: std::collections::HashSet<&str> = self.sources.iter().map(|s| s.name.as_str()).collect();
        if !existing.contains(prefix) {
            return prefix.to_string();
        }
        let mut i = 2;
        loop {
            let candidate = format!("{} {}", prefix, i);
            if !existing.contains(candidate.as_str()) {
                return candidate;
            }
            i += 1;
        }
    }

    pub fn add_source(&mut self, item: SourceItem) {
        self.selected_source_id = Some(item.id);
        // Insert at index 0 (top of the Source panel = front/foreground layer on canvas)
        self.sources.insert(0, item);
    }

    pub fn remove_selected_source(&mut self) -> bool {
        if let Some(id) = self.selected_source_id {
            if let Some(idx) = self.sources.iter().position(|s| s.id == id) {
                self.sources.remove(idx);
                self.selected_source_id = self.sources.first().map(|s| s.id);
                return true;
            }
        }
        false
    }

    pub fn move_selected_source_up(&mut self) -> bool {
        if let Some(id) = self.selected_source_id {
            if let Some(idx) = self.sources.iter().position(|s| s.id == id) {
                if idx > 0 {
                    self.sources.swap(idx, idx - 1);
                    return true;
                }
            }
        }
        false
    }

    /// Moves a source to an explicit slot of the layer stack.
    ///
    /// The list order *is* the Z-order: index 0 is the front-most layer and the last index sits
    /// behind everything else. `target_slot` is counted **without** the moved source, which is
    /// exactly what a drag-and-drop insertion gap means (dropping it on its own slot is a no-op).
    pub fn move_source_to_slot(&mut self, id: Uuid, target_slot: usize) -> bool {
        let Some(idx) = self.sources.iter().position(|s| s.id == id) else {
            return false;
        };
        // Clamp *before* removing, so a drop below the last row really means "bottom-most".
        let insert_at = target_slot.min(self.sources.len().saturating_sub(1));
        if insert_at == idx {
            return false;
        }
        let item = self.sources.remove(idx);
        self.sources.insert(insert_at, item);
        true
    }

    pub fn move_selected_source_down(&mut self) -> bool {
        if let Some(id) = self.selected_source_id {
            if let Some(idx) = self.sources.iter().position(|s| s.id == id) {
                if idx + 1 < self.sources.len() {
                    self.sources.swap(idx, idx + 1);
                    return true;
                }
            }
        }
        false
    }

    pub fn move_selected_source_to_top(&mut self) -> bool {
        if let Some(id) = self.selected_source_id {
            if let Some(idx) = self.sources.iter().position(|s| s.id == id) {
                if idx > 0 {
                    let item = self.sources.remove(idx);
                    self.sources.insert(0, item);
                    return true;
                }
            }
        }
        false
    }

    pub fn move_selected_source_to_bottom(&mut self) -> bool {
        if let Some(id) = self.selected_source_id {
            if let Some(idx) = self.sources.iter().position(|s| s.id == id) {
                if idx + 1 < self.sources.len() {
                    let item = self.sources.remove(idx);
                    self.sources.push(item);
                    return true;
                }
            }
        }
        false
    }

    pub fn duplicate_selected_source(&mut self) -> Option<Uuid> {
        let id = self.selected_source_id?;
        let idx = self.sources.iter().position(|s| s.id == id)?;
        let mut copy = self.sources[idx].clone();
        copy.id = Uuid::new_v4();
        copy.name = format!("{} (Copy)", copy.name);
        let new_id = copy.id;
        self.sources.insert(idx, copy);
        self.selected_source_id = Some(new_id);
        Some(new_id)
    }

    pub fn paste_source(&mut self, source: &SourceItem) -> Uuid {
        let mut copy = source.clone();
        copy.id = Uuid::new_v4();

        let name_exists = self.sources.iter().any(|s| s.name.eq_ignore_ascii_case(&copy.name));
        if name_exists {
            let mut counter = 2;
            let base_name = copy.name.clone();
            let mut candidate = format!("{} (Copy)", base_name);
            while self.sources.iter().any(|s| s.name.eq_ignore_ascii_case(&candidate)) {
                candidate = format!("{} (Copy {})", base_name, counter);
                counter += 1;
            }
            copy.name = candidate;
            copy.transform.pos_x = (copy.transform.pos_x + 0.02).clamp(0.0, 0.90);
            copy.transform.pos_y = (copy.transform.pos_y + 0.02).clamp(0.0, 0.90);
        }

        let new_id = copy.id;
        self.sources.insert(0, copy);
        self.selected_source_id = Some(new_id);
        new_id
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn scene_with(names: &[&str]) -> Scene {
        let mut scene = Scene::new("Test Scene");
        for name in names {
            let item = SourceItem::new(
                *name,
                SourceKind::DisplayCapture {
                    monitor_id: "MONITOR_0".to_string(),
                    monitor_name: "Primary Monitor (Display 1)".to_string(),
                    resolution: (1920, 1080),
                    capture_cursor: true,
                },
            );
            scene.sources.push(item);
        }
        scene
    }

    fn order(scene: &Scene) -> Vec<&str> {
        scene.sources.iter().map(|s| s.name.as_str()).collect()
    }

    #[test]
    fn move_source_to_slot_reorders_layer_stack() {
        let mut scene = scene_with(&["A", "B", "C", "D"]);
        let b_id = scene.sources[1].id;

        // Drag B below D: the slot is counted without the dragged source.
        assert!(scene.move_source_to_slot(b_id, 3));
        assert_eq!(order(&scene), vec!["A", "C", "D", "B"]);

        // Drag B back to the front-most layer (index 0 = foreground).
        assert!(scene.move_source_to_slot(b_id, 0));
        assert_eq!(order(&scene), vec!["B", "A", "C", "D"]);
    }

    #[test]
    fn move_source_to_same_slot_is_a_noop() {
        let mut scene = scene_with(&["A", "B", "C"]);
        let c_id = scene.sources[2].id;
        // Dropping a row into its own gap must not change the order.
        assert!(!scene.move_source_to_slot(c_id, 2));
        assert_eq!(order(&scene), vec!["A", "B", "C"]);
        assert!(!scene.move_source_to_slot(c_id, 99));
        assert_eq!(order(&scene), vec!["A", "B", "C"]);
    }

    #[test]
    fn move_unknown_source_is_rejected() {
        let mut scene = scene_with(&["A", "B"]);
        assert!(!scene.move_source_to_slot(Uuid::new_v4(), 0));
        assert_eq!(order(&scene), vec!["A", "B"]);
    }
}
