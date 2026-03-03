use std::path::PathBuf;

/// Top-level application state for GhostMD.
pub struct GhostApp {
    /// Root directory of the note vault.
    pub root: PathBuf,
    /// Whether the sidebar file tree is visible.
    pub sidebar_visible: bool,
    /// Paths of files currently open in tabs (across all splits).
    pub open_files: Vec<PathBuf>,
}

impl GhostApp {
    /// Creates a new GhostApp for the given vault root.
    pub fn new(root: PathBuf) -> Self {
        GhostApp {
            root,
            sidebar_visible: true,
            open_files: Vec::new(),
        }
    }

    /// Toggles sidebar visibility.
    pub fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
    }

    /// Open a file in the active split. Returns true if newly opened, false if already open.
    pub fn open_file(&mut self, path: PathBuf) -> bool {
        if self.open_files.contains(&path) {
            return false;
        }
        self.open_files.push(path);
        true
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
        assert!(app.open_files.is_empty());
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
}
