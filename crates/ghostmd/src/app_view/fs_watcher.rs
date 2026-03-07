use std::path::PathBuf;

use gpui::*;

use super::*;

impl GhostAppView {
    pub(crate) fn auto_save(&mut self, cx: &mut Context<Self>) {
        self.process_fs_events(cx);

        for ws in &self.workspaces {
            for pane in ws.panes.values() {
                if let Some(editor) = &pane.editor {
                    editor.update(cx, |e, cx| {
                        if e.should_auto_save(300) {
                            e.save(cx).ok();
                        }
                    });
                }
            }
        }
        // Periodically save session state
        self.save_session();
    }

    pub(crate) fn process_fs_events(&mut self, cx: &mut Context<Self>) {
        let Some(rx) = &self.fs_events_rx else { return };

        let mut tree_changed = false;
        let mut session_changed = false;
        let mut changed_files: Vec<PathBuf> = Vec::new();

        while let Ok(event) = rx.try_recv() {
            for path in &event.paths {
                if path.ends_with("session.json")
                    && path.starts_with(self.root.join(".ghostmd"))
                {
                    session_changed = true;
                } else if path.starts_with(&self.root) {
                    tree_changed = true;
                    if path.extension().is_some_and(|e| e == "md") {
                        changed_files.push(path.clone());
                    }
                }
            }
        }

        if tree_changed {
            self.file_tree.update(cx, |tree, cx| tree.refresh(cx));
        }
        if session_changed {
            self.reload_session_titles();
        }

        // Flag open editors whose file changed externally
        for path in changed_files {
            for ws in &self.workspaces {
                for pane in ws.panes.values() {
                    if pane.active_path.as_ref() == Some(&path) {
                        if let Some(editor) = &pane.editor {
                            editor.update(cx, |e, _cx| {
                                if !e.dirty {
                                    e.needs_reload = true;
                                }
                            });
                        }
                    }
                }
            }
        }

        if tree_changed || session_changed {
            cx.notify();
        }
    }
}
