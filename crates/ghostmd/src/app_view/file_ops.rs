use std::path::{Path, PathBuf};
use std::process::Command;

use gpui::*;

use ghostmd_core::diary;
use ghostmd_core::path_utils::unique_path;

use super::*;

impl GhostAppView {
    /// Update editor paths across all workspaces when a file or directory is renamed/moved.
    /// Handles both exact path matches and child paths (for directory moves).
    pub(crate) fn update_editor_paths(&mut self, old: &Path, new: &Path, cx: &mut Context<Self>) {
        let is_dir = new.is_dir();
        let mut editors_to_update: Vec<(Entity<EditorView>, PathBuf)> = Vec::new();
        for ws in &mut self.workspaces {
            for pane in ws.panes.values_mut() {
                if let Some(old_p) = &pane.active_path {
                    let updated = if *old_p == old {
                        Some(new.to_path_buf())
                    } else if is_dir && old_p.starts_with(old) {
                        old_p.strip_prefix(old).ok().map(|rel| new.join(rel))
                    } else {
                        None
                    };
                    if let Some(np) = updated {
                        pane.active_path = Some(np.clone());
                        if let Some(editor) = &pane.editor {
                            editors_to_update.push((editor.clone(), np));
                        }
                    }
                }
            }
        }
        for (editor, np) in editors_to_update {
            editor.update(cx, |e, _cx| { e.path = np; });
        }
    }

    /// Open a file: create per-pane editor and set it as active in the focused pane.
    pub(crate) fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.ensure_workspace(window, cx);
        // Save current editor if switching files
        let save_editor = {
            let ws = &self.workspaces[self.active_workspace];
            ws.panes.get(&ws.focused_pane).and_then(|p| {
                if p.active_path.as_ref() != Some(&path) {
                    p.editor.clone()
                } else {
                    None
                }
            })
        };
        if let Some(editor) = save_editor {
            editor.update(cx, |e, cx| { e.save(cx).ok(); });
        }

        // Check if this pane already has this file
        let already_open = {
            let ws = &self.workspaces[self.active_workspace];
            ws.panes.get(&ws.focused_pane)
                .map(|p| p.active_path.as_ref() == Some(&path))
                .unwrap_or(false)
        };

        if !already_open {
            let p = path.clone();
            let editor = cx.new(|cx| crate::editor_view::EditorView::new(p, window, cx));
            let ws = self.active_ws_mut();
            if let Some(pane) = ws.panes.get_mut(&ws.focused_pane) {
                pane.editor = Some(editor);
                pane.active_path = Some(path.clone());
            }
        }

        let focused = self.active_ws().focused_pane;
        self.focus_pane_editor(focused, window, cx);
        // Reveal file in tree (collapse non-ancestors, expand ancestors, scroll)
        self.file_tree.update(cx, |tree, cx| {
            tree.reveal_file(&path, cx);
        });
        cx.notify();
    }

    /// Create a new note with inline rename in the file tree (cmd-n).
    /// If a folder is selected, shows a location picker to choose between diary and selected folder.
    pub(crate) fn new_note_in_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.ensure_workspace(window, cx);
        let root = self.root.clone();
        let diary_dir = diary::today_diary_dir(&root);
        let selected_dir = self.file_tree.read(cx).selected_path()
            .and_then(|p| {
                if p.is_dir() { Some(p.clone()) } else { p.parent().map(|pp| pp.to_path_buf()) }
            })
            .filter(|d| d.starts_with(&root) && *d != root);

        // If a non-diary folder is selected, show the location picker
        if let Some(ref dir) = selected_dir {
            if *dir != diary_dir {
                let rel_path = dir.strip_prefix(&root)
                    .unwrap_or(dir)
                    .to_string_lossy()
                    .to_string();
                self.location_picker_options = vec![
                    (format!("diary ({})", diary_dir.strip_prefix(&root).unwrap_or(&diary_dir).to_string_lossy()), diary_dir),
                    (rel_path, dir.clone()),
                ];
                self.location_picker_selected = 0;
                self.active_overlay = Some(OverlayKind::LocationPicker);
                // Focus root so Enter/Up/Down aren't consumed by the editor's Input
                window.focus(&self.focus_handle);
                cx.notify();
                return;
            }
        }

        // No folder selected or already in diary — create directly
        self.create_note_at(diary_dir, window, cx);
    }

    /// Actually create the note at the given directory.
    pub(crate) fn create_note_at(&mut self, parent_dir: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        std::fs::create_dir_all(&parent_dir).ok();
        if !self.sidebar_visible {
            self.sidebar_visible = true;
        }
        let name = pick_note_name(&parent_dir);
        self.file_tree.update(cx, |tree, cx| {
            tree.start_new_note(&parent_dir, &name, window, cx);
        });
        cx.notify();
    }

    /// Close the location picker and refocus the editor.
    pub(crate) fn close_location_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.active_overlay = None;
        self.location_picker_options.clear();
        if !self.workspaces.is_empty() {
            let focused = self.active_ws().focused_pane;
            self.focus_pane_editor(focused, window, cx);
        }
        cx.notify();
    }

    /// Confirm the location picker selection and create the note.
    pub(crate) fn confirm_location_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some((_, dir)) = self.location_picker_options.get(self.location_picker_selected).cloned() {
            self.active_overlay = None;
            self.location_picker_options.clear();
            self.create_note_at(dir, window, cx);
        }
    }

    /// Create a new note in a specific directory with inline rename.
    pub(crate) fn new_note_in_dir(&mut self, dir: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if !self.sidebar_visible {
            self.sidebar_visible = true;
        }
        let name = pick_note_name(&dir);
        self.file_tree.update(cx, |tree, cx| {
            tree.start_new_note(&dir, &name, window, cx);
        });
        cx.notify();
    }

    /// Create a new folder inside a parent directory with inline rename.
    pub(crate) fn create_new_folder(&mut self, parent: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if !self.sidebar_visible {
            self.sidebar_visible = true;
        }
        self.file_tree.update(cx, |tree, cx| {
            tree.start_new_folder(&parent, window, cx);
        });
        cx.notify();
    }

    /// Move a file to a target directory, updating editor paths and tree.
    pub(crate) fn move_file_to_dir(&mut self, source: PathBuf, target_dir: &std::path::Path, cx: &mut Context<Self>) {
        let file_name = source.file_name().unwrap_or_default();
        let new_path = unique_path(&target_dir.join(file_name));
        if new_path == source || new_path.exists() { return; }
        if std::fs::rename(&source, &new_path).is_ok() {
            self.update_editor_paths(&source, &new_path, cx);
            self.file_tree.update(cx, |tree, cx| {
                tree.refresh(cx);
                tree.reveal_file(&new_path, cx);
            });
        }
    }

    /// Open the file finder in folder-only mode for moving a file.
    pub(crate) fn start_move_to_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let source = match self.focused_active_path() {
            Some(p) => p,
            None => return,
        };
        self.folder_move_source = Some(source);
        self.active_overlay = Some(OverlayKind::FileFinder);
        self.file_finder.open_folders().ok();
        self.finder_scroll = ScrollHandle::new();
        self.file_finder_input.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }

    /// Move a file or folder to the macOS Trash and update the UI.
    pub(crate) fn move_to_trash(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        // Close any panes showing this file (or files inside this directory)
        let is_dir = path.is_dir();
        let mut editors_to_save: Vec<_> = Vec::new();
        for ws in &mut self.workspaces {
            for pane in ws.panes.values_mut() {
                let should_close = pane.active_path.as_ref().map(|p| {
                    if is_dir { p.starts_with(&path) } else { p == &path }
                }).unwrap_or(false);
                if should_close {
                    if let Some(editor) = pane.editor.take() {
                        editors_to_save.push(editor);
                    }
                    pane.active_path = None;
                }
            }
        }
        // Save editors before trashing (best effort)
        for editor in editors_to_save {
            editor.update(cx, |e, cx| { e.save(cx).ok(); });
        }

        // Move to Trash using the trash crate (macOS native)
        if trash::delete(&path).is_ok() {
            self.file_tree.update(cx, |tree, cx| tree.refresh(cx));
            // Re-focus current pane
            let focused = self.active_ws().focused_pane;
            self.focus_pane_editor(focused, window, cx);
            cx.notify();
        }
    }

    /// Run the update script and restart the app.
    pub(crate) fn run_update(&mut self, cx: &mut Context<Self>) {
        // Save session before updating
        self.save_session();
        cx.spawn(async |_this, cx: &mut AsyncApp| {
            let result = cx.background_executor().spawn(async {
                Command::new("bash")
                    .args(["-c", "curl -fsSL https://raw.githubusercontent.com/mimoo/ghostmd/main/scripts/install.sh | bash"])
                    .output()
            }).await;
            if let Ok(output) = result {
                if output.status.success() {
                    // Relaunch the app
                    Command::new("open")
                        .args(["-a", "/Applications/GhostMD.app"])
                        .spawn()
                        .ok();
                    cx.update(|cx| cx.quit()).ok();
                }
            }
        }).detach();
    }
}
