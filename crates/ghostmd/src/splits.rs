#![allow(dead_code)]

use std::path::PathBuf;

/// Orientation of a split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// A pane in the split layout, holding the path of its active editor.
#[derive(Debug, Clone)]
pub struct Pane {
    pub active_path: Option<PathBuf>,
    pub id: usize,
}

/// Layout manager for split editor panes.
pub struct SplitLayout {
    panes: Vec<Pane>,
    next_id: usize,
}

impl SplitLayout {
    /// Creates a new layout with a single empty pane.
    pub fn new() -> Self {
        SplitLayout {
            panes: vec![Pane {
                active_path: None,
                id: 0,
            }],
            next_id: 1,
        }
    }

    /// Splits the currently focused pane to the right, returning the new pane's id.
    pub fn split_right(&mut self, focused: usize) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        let insert_at = focused + 1;
        let pane = Pane {
            active_path: None,
            id,
        };
        if insert_at >= self.panes.len() {
            self.panes.push(pane);
        } else {
            self.panes.insert(insert_at, pane);
        }
        id
    }

    /// Splits the currently focused pane downward, returning the new pane's id.
    pub fn split_down(&mut self, focused: usize) -> usize {
        // For now, split_down behaves the same as split_right in the flat list.
        // A tree-based layout will be implemented with GPUI rendering.
        self.split_right(focused)
    }

    /// Returns the number of panes.
    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    /// Returns a reference to the panes.
    pub fn panes(&self) -> &[Pane] {
        &self.panes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_layout_has_one_pane() {
        let layout = SplitLayout::new();
        assert_eq!(layout.pane_count(), 1);
    }

    #[test]
    fn split_right_adds_pane() {
        let mut layout = SplitLayout::new();
        layout.split_right(0);
        assert_eq!(layout.pane_count(), 2);
    }

    #[test]
    fn split_down_adds_pane() {
        let mut layout = SplitLayout::new();
        layout.split_down(0);
        assert_eq!(layout.pane_count(), 2);
    }

    #[test]
    fn multiple_splits() {
        let mut layout = SplitLayout::new();
        layout.split_right(0);
        layout.split_right(1);
        layout.split_down(0);
        assert_eq!(layout.pane_count(), 4);
    }

    #[test]
    fn split_returns_unique_ids() {
        let mut layout = SplitLayout::new();
        let id1 = layout.split_right(0);
        let id2 = layout.split_right(1);
        assert_ne!(id1, id2);
    }

    #[test]
    fn deeply_nested_splits_five_times() {
        let mut layout = SplitLayout::new();
        // Split 5 times, always at the last pane index
        for i in 0..5 {
            layout.split_right(i);
        }
        assert_eq!(layout.pane_count(), 6); // 1 original + 5 splits

        // All pane ids should be unique
        let ids: Vec<usize> = layout.panes().iter().map(|p| p.id).collect();
        let mut unique_ids = ids.clone();
        unique_ids.sort();
        unique_ids.dedup();
        assert_eq!(ids.len(), unique_ids.len());
    }

    #[test]
    fn split_preserves_existing_content() {
        let mut layout = SplitLayout::new();

        // Set active path on pane 0
        layout.panes[0].active_path = Some(PathBuf::from("existing.md"));

        // Split right from pane 0
        layout.split_right(0);

        // Original pane should still have its path
        assert_eq!(
            layout.panes()[0].active_path,
            Some(PathBuf::from("existing.md"))
        );
        // New pane should have no active path
        assert_eq!(layout.panes()[1].active_path, None);
    }

    #[test]
    fn split_inserts_at_correct_position() {
        let mut layout = SplitLayout::new();

        // Pane 0 (id=0)
        layout.panes[0].active_path = Some(PathBuf::from("first.md"));

        // Split right from pane 0 -> inserts at index 1
        layout.split_right(0);
        layout.panes[1].active_path = Some(PathBuf::from("second.md"));

        // Split right from pane 0 again -> inserts at index 1, pushing "second.md" to index 2
        layout.split_right(0);
        layout.panes[1].active_path = Some(PathBuf::from("middle.md"));

        assert_eq!(layout.pane_count(), 3);
        assert_eq!(layout.panes()[0].active_path, Some(PathBuf::from("first.md")));
        assert_eq!(layout.panes()[1].active_path, Some(PathBuf::from("middle.md")));
        assert_eq!(layout.panes()[2].active_path, Some(PathBuf::from("second.md")));
    }
}
