use std::path::{Path, PathBuf};

use crate::splits::SplitLayout;
use crate::tabs::{ClosedTab, TabManager};

/// Which overlay is currently displayed on top of the workspace.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    None,
    FileFinder,
    ContentSearch,
    CommandPalette,
}

/// Top-level application state for GhostMD.
pub struct GhostApp {
    /// Root directory of the note vault.
    pub root: PathBuf,
    /// Recently-closed tab stack for restore.
    #[allow(dead_code)]
    pub tab_manager: TabManager,
    /// Split pane layout.
    #[allow(dead_code)]
    pub splits: SplitLayout,
    /// Whether the sidebar file tree is visible.
    pub sidebar_visible: bool,
    /// Currently active overlay.
    #[allow(dead_code)]
    pub overlay: Overlay,
    /// Index of the currently focused split pane.
    #[allow(dead_code)]
    pub focused_split: usize,
    /// Paths of files currently open in tabs (across all splits).
    pub open_files: Vec<PathBuf>,
}

impl GhostApp {
    /// Creates a new GhostApp for the given vault root.
    pub fn new(root: PathBuf) -> Self {
        GhostApp {
            root,
            tab_manager: TabManager::new(20),
            splits: SplitLayout::new(),
            sidebar_visible: true,
            overlay: Overlay::None,
            focused_split: 0,
            open_files: Vec::new(),
        }
    }

    /// Toggles sidebar visibility.
    pub fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
    }

    /// Open an overlay (closes any current one first).
    #[allow(dead_code)]
    pub fn open_overlay(&mut self, overlay: Overlay) {
        self.overlay = overlay;
    }

    /// Close the current overlay.
    #[allow(dead_code)]
    pub fn close_overlay(&mut self) {
        self.overlay = Overlay::None;
    }

    /// Toggle a specific overlay (open if closed, close if open).
    #[allow(dead_code)]
    pub fn toggle_overlay(&mut self, overlay: Overlay) {
        if self.overlay == overlay {
            self.overlay = Overlay::None;
        } else {
            self.overlay = overlay;
        }
    }

    /// Open a file in the active split. Returns true if newly opened, false if already open.
    pub fn open_file(&mut self, path: PathBuf) -> bool {
        if self.open_files.contains(&path) {
            return false;
        }
        self.open_files.push(path);
        true
    }

    /// Close a file, push to TabManager for restore.
    #[allow(dead_code)]
    pub fn close_file(&mut self, path: &Path, cursor_position: usize) {
        if let Some(pos) = self.open_files.iter().position(|p| p == path) {
            self.open_files.remove(pos);
            self.tab_manager.push_closed(ClosedTab {
                path: path.to_path_buf(),
                cursor_position,
            });
        }
    }

    /// Restore most recently closed tab. Returns the path if restored.
    #[allow(dead_code)]
    pub fn restore_tab(&mut self) -> Option<PathBuf> {
        let closed = self.tab_manager.pop_closed()?;
        self.open_files.push(closed.path.clone());
        Some(closed.path)
    }

    /// Check if a file is currently open.
    #[allow(dead_code)]
    pub fn is_file_open(&self, path: &Path) -> bool {
        self.open_files.iter().any(|p| p == path)
    }

    /// Navigate to the next split (wraps around).
    #[allow(dead_code)]
    pub fn focus_next_split(&mut self) {
        let count = self.splits.pane_count();
        if count > 0 {
            self.focused_split = (self.focused_split + 1) % count;
        }
    }

    /// Navigate to the previous split (wraps around).
    #[allow(dead_code)]
    pub fn focus_prev_split(&mut self) {
        let count = self.splits.pane_count();
        if count > 0 {
            if self.focused_split == 0 {
                self.focused_split = count - 1;
            } else {
                self.focused_split -= 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_app() -> GhostApp {
        GhostApp::new(PathBuf::from("/tmp/vault"))
    }

    #[test]
    fn new_defaults() {
        let app = make_app();
        assert!(app.sidebar_visible);
        assert_eq!(app.overlay, Overlay::None);
        assert_eq!(app.splits.pane_count(), 1);
        assert!(app.open_files.is_empty());
        assert_eq!(app.focused_split, 0);
    }

    #[test]
    fn toggle_sidebar_flips_state() {
        let mut app = make_app();
        assert!(app.sidebar_visible);
        app.toggle_sidebar();
        assert!(!app.sidebar_visible);
        app.toggle_sidebar();
        assert!(app.sidebar_visible);
    }

    #[test]
    fn open_overlay_sets_overlay() {
        let mut app = make_app();
        app.open_overlay(Overlay::FileFinder);
        assert_eq!(app.overlay, Overlay::FileFinder);
    }

    #[test]
    fn close_overlay_resets_to_none() {
        let mut app = make_app();
        app.open_overlay(Overlay::CommandPalette);
        app.close_overlay();
        assert_eq!(app.overlay, Overlay::None);
    }

    #[test]
    fn open_overlay_replaces_existing() {
        let mut app = make_app();
        app.open_overlay(Overlay::FileFinder);
        app.open_overlay(Overlay::ContentSearch);
        assert_eq!(app.overlay, Overlay::ContentSearch);
    }

    #[test]
    fn toggle_overlay_open_then_close() {
        let mut app = make_app();
        app.toggle_overlay(Overlay::CommandPalette);
        assert_eq!(app.overlay, Overlay::CommandPalette);
        app.toggle_overlay(Overlay::CommandPalette);
        assert_eq!(app.overlay, Overlay::None);
    }

    #[test]
    fn toggle_overlay_switches_between_overlays() {
        let mut app = make_app();
        app.toggle_overlay(Overlay::FileFinder);
        assert_eq!(app.overlay, Overlay::FileFinder);
        // Toggling a different overlay opens it (replaces)
        app.toggle_overlay(Overlay::ContentSearch);
        assert_eq!(app.overlay, Overlay::ContentSearch);
    }

    #[test]
    fn open_file_adds_to_open_files() {
        let mut app = make_app();
        let result = app.open_file(PathBuf::from("notes/hello.md"));
        assert!(result);
        assert_eq!(app.open_files.len(), 1);
    }

    #[test]
    fn open_file_already_open_returns_false() {
        let mut app = make_app();
        app.open_file(PathBuf::from("notes/hello.md"));
        let result = app.open_file(PathBuf::from("notes/hello.md"));
        assert!(!result);
        assert_eq!(app.open_files.len(), 1);
    }

    #[test]
    fn close_file_removes_and_pushes_to_tab_manager() {
        let mut app = make_app();
        app.open_file(PathBuf::from("notes/hello.md"));
        app.close_file(Path::new("notes/hello.md"), 42);
        assert!(app.open_files.is_empty());
        assert_eq!(app.tab_manager.len(), 1);
    }

    #[test]
    fn restore_tab_pops_from_tab_manager_and_adds_back() {
        let mut app = make_app();
        app.open_file(PathBuf::from("notes/hello.md"));
        app.close_file(Path::new("notes/hello.md"), 42);
        assert!(app.open_files.is_empty());

        let restored = app.restore_tab();
        assert_eq!(restored, Some(PathBuf::from("notes/hello.md")));
        assert_eq!(app.open_files.len(), 1);
        assert!(app.tab_manager.is_empty());
    }

    #[test]
    fn restore_tab_when_empty_returns_none() {
        let mut app = make_app();
        assert!(app.restore_tab().is_none());
    }

    #[test]
    fn is_file_open_true_for_open_false_for_closed() {
        let mut app = make_app();
        let path = PathBuf::from("notes/test.md");
        assert!(!app.is_file_open(&path));
        app.open_file(path.clone());
        assert!(app.is_file_open(&path));
        app.close_file(&path, 0);
        assert!(!app.is_file_open(&path));
    }

    #[test]
    fn focus_next_split_wraps_around() {
        let mut app = make_app();
        // With 1 pane, next stays at 0
        app.focus_next_split();
        assert_eq!(app.focused_split, 0);

        // Add a second pane
        app.splits.split_right(0);
        assert_eq!(app.splits.pane_count(), 2);

        app.focus_next_split();
        assert_eq!(app.focused_split, 1);
        app.focus_next_split();
        assert_eq!(app.focused_split, 0); // wraps
    }

    #[test]
    fn focus_prev_split_wraps_around() {
        let mut app = make_app();
        // With 1 pane, prev stays at 0
        app.focus_prev_split();
        assert_eq!(app.focused_split, 0);

        // Add a second pane
        app.splits.split_right(0);
        assert_eq!(app.splits.pane_count(), 2);

        app.focus_prev_split();
        assert_eq!(app.focused_split, 1); // wraps from 0 to last
        app.focus_prev_split();
        assert_eq!(app.focused_split, 0);
    }
}
