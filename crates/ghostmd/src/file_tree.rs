use std::path::{Path, PathBuf};

use ghostmd_core::tree::{FileTree, TreeNode};

/// UI state for the sidebar file tree panel.
pub struct FileTreePanel {
    /// Root directory of the vault being displayed.
    pub root: PathBuf,
    /// The underlying file tree data.
    pub tree: FileTree,
    /// Index into the flattened visible items for the current selection.
    pub selected_index: Option<usize>,
    /// Width of the sidebar panel in pixels.
    pub width: f32,
    /// Whether the panel is visible.
    pub visible: bool,
}

impl FileTreePanel {
    /// Creates a new file tree panel for the given root.
    pub fn new(root: PathBuf) -> Self {
        FileTreePanel {
            tree: FileTree::new(root.clone()),
            root,
            selected_index: None,
            width: 240.0,
            visible: true,
        }
    }

    /// Refresh the tree by rescanning the filesystem.
    pub fn refresh(&mut self) -> anyhow::Result<()> {
        self.tree.scan()?;
        // Reset selection if it is now out of bounds
        let count = self.tree.flatten().len();
        if let Some(idx) = self.selected_index {
            if idx >= count {
                self.selected_index = if count > 0 { Some(count - 1) } else { None };
            }
        }
        Ok(())
    }

    /// Get the flattened visible items (depth, node) pairs.
    pub fn visible_items(&self) -> Vec<(usize, &TreeNode)> {
        self.tree.flatten()
    }

    /// Move selection down. Wraps to the beginning at the end.
    pub fn select_next(&mut self) {
        let count = self.visible_items().len();
        if count == 0 {
            return;
        }
        self.selected_index = Some(match self.selected_index {
            None => 0,
            Some(idx) => (idx + 1) % count,
        });
    }

    /// Move selection up. Wraps to the end at the beginning.
    pub fn select_prev(&mut self) {
        let count = self.visible_items().len();
        if count == 0 {
            return;
        }
        self.selected_index = Some(match self.selected_index {
            None => count - 1,
            Some(0) => count - 1,
            Some(idx) => idx - 1,
        });
    }

    /// Get the path of the currently selected item.
    pub fn selected_path(&self) -> Option<PathBuf> {
        let idx = self.selected_index?;
        let items = self.visible_items();
        items.get(idx).map(|(_, node)| node_path(node).to_path_buf())
    }

    /// Toggle expand/collapse if selected item is a directory. No-op for files.
    pub fn toggle_selected(&mut self) {
        if let Some(path) = self.selected_path() {
            self.tree.toggle_dir(&path);
            // After toggling, clamp the selection index if items changed
            let count = self.visible_items().len();
            if let Some(idx) = self.selected_index {
                if idx >= count && count > 0 {
                    self.selected_index = Some(count - 1);
                }
            }
        }
    }

    /// Select a specific path. If the path is not found in visible items, selection is unchanged.
    pub fn select_path(&mut self, path: &Path) {
        let items = self.visible_items();
        for (i, (_, node)) in items.iter().enumerate() {
            if node_path(node) == path {
                self.selected_index = Some(i);
                return;
            }
        }
    }
}

/// Helper to extract the path from a TreeNode.
fn node_path(node: &TreeNode) -> &Path {
    match node {
        TreeNode::Directory { path, .. } => path,
        TreeNode::File { path, .. } => path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_basic_tree(tmp: &TempDir) {
        let root = tmp.path();
        fs::create_dir_all(root.join("notes")).unwrap();
        fs::write(root.join("notes/alpha.md"), "alpha").unwrap();
        fs::write(root.join("notes/beta.md"), "beta").unwrap();
        fs::write(root.join("readme.md"), "readme").unwrap();
    }

    fn make_panel(tmp: &TempDir) -> FileTreePanel {
        let mut panel = FileTreePanel::new(tmp.path().to_path_buf());
        panel.refresh().unwrap();
        panel
    }

    #[test]
    fn new_defaults() {
        let tmp = TempDir::new().unwrap();
        let panel = FileTreePanel::new(tmp.path().to_path_buf());
        assert!(panel.selected_index.is_none());
        assert!((panel.width - 240.0).abs() < f32::EPSILON);
        assert!(panel.visible);
    }

    #[test]
    fn refresh_populates_tree() {
        let tmp = TempDir::new().unwrap();
        setup_basic_tree(&tmp);
        let panel = make_panel(&tmp);
        // "notes" dir, "alpha.md", "beta.md" (inside notes, expanded), "readme.md"
        assert!(!panel.visible_items().is_empty());
        assert!(panel.tree.file_count() > 0);
    }

    #[test]
    fn select_next_from_none_selects_first() {
        let tmp = TempDir::new().unwrap();
        setup_basic_tree(&tmp);
        let mut panel = make_panel(&tmp);
        assert!(panel.selected_index.is_none());
        panel.select_next();
        assert_eq!(panel.selected_index, Some(0));
    }

    #[test]
    fn select_next_advances_through_items() {
        let tmp = TempDir::new().unwrap();
        setup_basic_tree(&tmp);
        let mut panel = make_panel(&tmp);
        panel.select_next(); // 0
        panel.select_next(); // 1
        assert_eq!(panel.selected_index, Some(1));
        panel.select_next(); // 2
        assert_eq!(panel.selected_index, Some(2));
    }

    #[test]
    fn select_next_at_end_wraps_to_zero() {
        let tmp = TempDir::new().unwrap();
        setup_basic_tree(&tmp);
        let mut panel = make_panel(&tmp);
        let count = panel.visible_items().len();
        // First call goes None -> 0, then count more calls to wrap back to 0
        for _ in 0..=count {
            panel.select_next();
        }
        assert_eq!(panel.selected_index, Some(0)); // wrapped
    }

    #[test]
    fn select_prev_from_zero_wraps_to_end() {
        let tmp = TempDir::new().unwrap();
        setup_basic_tree(&tmp);
        let mut panel = make_panel(&tmp);
        let count = panel.visible_items().len();
        panel.selected_index = Some(0);
        panel.select_prev();
        assert_eq!(panel.selected_index, Some(count - 1));
    }

    #[test]
    fn select_prev_from_none_selects_last() {
        let tmp = TempDir::new().unwrap();
        setup_basic_tree(&tmp);
        let mut panel = make_panel(&tmp);
        let count = panel.visible_items().len();
        panel.select_prev();
        assert_eq!(panel.selected_index, Some(count - 1));
    }

    #[test]
    fn selected_path_returns_correct_path() {
        let tmp = TempDir::new().unwrap();
        setup_basic_tree(&tmp);
        let mut panel = make_panel(&tmp);
        panel.select_next(); // select first item
        let path = panel.selected_path().unwrap();
        // First item should be the "notes" directory (dirs sorted before files)
        assert_eq!(path, tmp.path().join("notes"));
    }

    #[test]
    fn toggle_selected_on_directory_collapses_it() {
        let tmp = TempDir::new().unwrap();
        setup_basic_tree(&tmp);
        let mut panel = make_panel(&tmp);

        let before_count = panel.visible_items().len();
        // Select the "notes" directory (first item)
        panel.select_next();
        assert_eq!(panel.selected_path().unwrap(), tmp.path().join("notes"));

        // Toggle it (collapse)
        panel.toggle_selected();

        let after_count = panel.visible_items().len();
        // After collapsing, "notes" children should be hidden
        assert!(after_count < before_count);
    }

    #[test]
    fn toggle_selected_on_file_is_noop() {
        let tmp = TempDir::new().unwrap();
        setup_basic_tree(&tmp);
        let mut panel = make_panel(&tmp);

        let before_count = panel.visible_items().len();
        // Navigate to a file: "notes" (dir), "alpha.md" (file at index 1)
        panel.selected_index = Some(1);
        let path = panel.selected_path().unwrap();
        // It should be a file inside the notes directory
        assert!(path.extension().is_some());

        panel.toggle_selected();
        let after_count = panel.visible_items().len();
        assert_eq!(before_count, after_count);
    }

    #[test]
    fn select_path_finds_and_selects_correct_index() {
        let tmp = TempDir::new().unwrap();
        setup_basic_tree(&tmp);
        let mut panel = make_panel(&tmp);

        let target = tmp.path().join("readme.md");
        panel.select_path(&target);

        assert!(panel.selected_index.is_some());
        assert_eq!(panel.selected_path().unwrap(), target);
    }

    #[test]
    fn select_path_for_nonexistent_leaves_selection_unchanged() {
        let tmp = TempDir::new().unwrap();
        setup_basic_tree(&tmp);
        let mut panel = make_panel(&tmp);
        panel.selected_index = Some(0);

        panel.select_path(Path::new("/nonexistent/path.md"));
        assert_eq!(panel.selected_index, Some(0)); // unchanged
    }

    #[test]
    fn select_next_on_empty_tree_is_noop() {
        let tmp = TempDir::new().unwrap();
        let mut panel = make_panel(&tmp);
        assert!(panel.visible_items().is_empty());
        panel.select_next();
        assert!(panel.selected_index.is_none());
    }

    #[test]
    fn select_prev_on_empty_tree_is_noop() {
        let tmp = TempDir::new().unwrap();
        let mut panel = make_panel(&tmp);
        assert!(panel.visible_items().is_empty());
        panel.select_prev();
        assert!(panel.selected_index.is_none());
    }
}
