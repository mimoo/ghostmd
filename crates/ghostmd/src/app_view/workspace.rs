use std::collections::HashMap;

use gpui::*;
use gpui_component::Root;

use crate::editor_view::EditorView;
use crate::theme::ThemeName;

use super::*;

impl GhostAppView {
    /// Create a new empty workspace with one pane.
    pub(crate) fn new_workspace(&mut self, _root: &std::path::Path, window: &mut Window, cx: &mut Context<Self>) {
        let ws_id = self.next_workspace_id;
        self.next_workspace_id += 1;

        let pane_id = self.next_pane_id;
        self.next_pane_id += 1;

        let mut panes = HashMap::new();
        panes.insert(pane_id, Pane { active_path: None, editor: None });

        let ws = Workspace {
            id: ws_id,
            title: random_note_name(),
            split_root: SplitNode::Leaf(pane_id),
            panes,
            focused_pane: pane_id,
            pane_focus_history: Vec::new(),
        };

        self.workspaces.push(ws);
        self.active_workspace = self.workspaces.len() - 1;
        self.focus_pane_editor(pane_id, window, cx);
        cx.notify();
    }

    /// Create a new workspace with a diary note (cmd-t).
    pub(crate) fn new_workspace_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.root.clone();
        self.new_workspace(&root, window, cx);
    }

    /// Open a new OS window (cmd-shift-n).
    pub(crate) fn new_window(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let root = self.root.clone();
        cx.spawn(async move |_this: WeakEntity<GhostAppView>, cx: &mut AsyncApp| {
            cx.update(|cx: &mut App| {
                let bounds = Bounds::centered(None, size(px(1200.), px(800.)), cx);
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        focus: true,
                        titlebar: Some(TitlebarOptions {
                            appears_transparent: true,
                            traffic_light_position: Some(gpui::point(px(9.0), px(9.0))),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    |window, cx| {
                        let app_view = cx.new(|cx| GhostAppView::new(root, false, window, cx));
                        cx.new(|cx| Root::new(app_view, window, cx))
                    },
                ).ok();
            }).ok();
        })
        .detach();
    }

    /// Switch to workspace at index.
    pub(crate) fn switch_workspace(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        if idx < self.workspaces.len() {
            self.active_workspace = idx;
            let focused = self.workspaces[idx].focused_pane;
            self.focus_pane_editor(focused, window, cx);
            // Sync file tree selection for the new workspace's focused file
            self.sync_file_tree_selection(cx);
            cx.notify();
        }
    }

    /// Focus the editor shown in the given pane.
    /// Falls back to root focus handle when the pane has no editor,
    /// so keybindings (cmd-n, cmd-w, etc.) still work in empty panes.
    pub(crate) fn focus_pane_editor(&self, pane_id: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspaces.is_empty() {
            window.focus(&self.focus_handle);
            return;
        }
        let ws = self.active_ws();
        if let Some(pane) = ws.panes.get(&pane_id) {
            if let Some(editor) = &pane.editor {
                editor.update(cx, |e, cx| {
                    e.focus_input(window, cx);
                });
                return;
            }
        }
        window.focus(&self.focus_handle);
    }

    /// Sync the file tree selection to the currently focused pane's file.
    pub(crate) fn sync_file_tree_selection(&self, cx: &mut Context<Self>) {
        if self.workspaces.is_empty() {
            return;
        }
        if let Some(path) = self.focused_active_path() {
            self.file_tree.update(cx, |tree, cx| {
                tree.reveal_file(&path, cx);
            });
        }
    }

    /// Split the focused pane, creating a new pane showing the same file.
    pub(crate) fn split(&mut self, direction: SplitDirection, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspaces.is_empty() {
            return;
        }
        let new_id = self.next_pane_id;
        self.next_pane_id += 1;

        let ws = self.active_ws_mut();
        ws.panes.insert(new_id, Pane { active_path: None, editor: None });
        ws.split_root.split_leaf(ws.focused_pane, new_id, direction);
        ws.pane_focus_history.push(ws.focused_pane);
        ws.focused_pane = new_id;
        self.focus_pane_editor(new_id, window, cx);
        cx.notify();
    }

    /// Navigate focus to an adjacent pane using 2D-aware tree navigation.
    /// Stops at edges (no wrapping).
    pub(crate) fn focus_pane_direction(&mut self, dx: i32, dy: i32, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspaces.is_empty() {
            return;
        }
        // Keep the search overlay open when navigating between panes
        // so match count can update; dismiss all other overlays.
        let keep_search = self.overlay_is(OverlayKind::Search);
        if !keep_search {
            self.dismiss_overlays(window, cx);
        }
        let ws = self.active_ws_mut();
        let from = ws.focused_pane;
        let target = if dx > 0 {
            ws.split_root.find_right(from)
        } else if dx < 0 {
            ws.split_root.find_left(from)
        } else if dy > 0 {
            ws.split_root.find_down(from)
        } else if dy < 0 {
            ws.split_root.find_up(from)
        } else {
            None
        };
        if let Some(new_id) = target {
            ws.pane_focus_history.push(ws.focused_pane);
            ws.focused_pane = new_id;
            self.focus_pane_editor(new_id, window, cx);
            self.sync_file_tree_selection(cx);
            if keep_search {
                self.update_search_matches(cx);
            }
            cx.notify();
        }
    }

    /// Close the focused pane. If it's the last pane with a file, clear to empty.
    /// If last pane is already empty, close the workspace.
    pub(crate) fn close_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspaces.is_empty() {
            return;
        }

        // Save the file in the focused pane before closing
        let save_editor = {
            let ws = &self.workspaces[self.active_workspace];
            ws.panes.get(&ws.focused_pane).and_then(|p| p.editor.clone())
        };
        if let Some(editor) = save_editor {
            editor.update(cx, |e, cx| { e.save(cx).ok(); });
        }

        let ws = &self.workspaces[self.active_workspace];
        let leaves = ws.split_root.leaves();

        if leaves.len() == 1 {
            let pane_id = leaves[0];
            let has_file = ws.panes.get(&pane_id)
                .map(|p| p.active_path.is_some())
                .unwrap_or(false);

            if has_file {
                // Clear the pane to empty state instead of closing workspace
                let pane = self.workspaces[self.active_workspace].panes.get_mut(&pane_id).unwrap();
                pane.active_path = None;
                pane.editor = None;
                window.focus(&self.focus_handle);
                cx.notify();
                return;
            }

            // Already empty → close the whole workspace
            let removed = self.workspaces.remove(self.active_workspace);
            self.closed_workspaces.push(removed);

            if self.workspaces.is_empty() {
                self.save_session(cx);
                window.remove_window();
                return;
            } else if self.active_workspace >= self.workspaces.len() {
                self.active_workspace = self.workspaces.len() - 1;
            }

            let focused = self.workspaces[self.active_workspace].focused_pane;
            self.focus_pane_editor(focused, window, cx);
            self.sync_file_tree_selection(cx);
            cx.notify();
            return;
        }

        let ws = self.active_ws_mut();
        let focused_id = ws.focused_pane;
        ws.panes.remove(&focused_id);
        ws.split_root.remove_leaf(focused_id);
        // Remove closed pane from history
        ws.pane_focus_history.retain(|&id| id != focused_id);

        // Switch focus to the most recently focused pane, or first remaining leaf
        let remaining: std::collections::HashSet<usize> = ws.panes.keys().copied().collect();
        let prev = ws.pane_focus_history.iter().rev()
            .find(|id| remaining.contains(id))
            .copied();
        if let Some(prev_id) = prev {
            ws.focused_pane = prev_id;
        } else {
            let leaves = ws.split_root.leaves();
            if let Some(&first) = leaves.first() {
                ws.focused_pane = first;
            }
        }

        let focused = self.active_ws().focused_pane;
        self.focus_pane_editor(focused, window, cx);
        self.sync_file_tree_selection(cx);
        cx.notify();
    }

    /// Switch to a named theme.
    pub(crate) fn switch_theme(&mut self, name: ThemeName, cx: &mut Context<Self>) {
        self.active_theme = name;
        self.theme = crate::theme::ResolvedTheme::from_name(name);
        self.file_tree.update(cx, |tree, _cx| {
            tree.set_theme(name);
        });
        crate::theme::apply_theme(name, cx);
        cx.notify();
    }

    /// Close workspace at given index.
    pub(crate) fn close_workspace(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        if idx >= self.workspaces.len() {
            return;
        }
        // Save editors in the workspace
        let editors: Vec<Entity<EditorView>> = self.workspaces[idx].panes.values()
            .filter_map(|p| p.editor.clone())
            .collect();
        for editor in editors {
            editor.update(cx, |e, cx| { e.save(cx).ok(); });
        }
        let removed = self.workspaces.remove(idx);
        self.closed_workspaces.push(removed);

        if self.workspaces.is_empty() {
            self.save_session(cx);
            window.remove_window();
            return;
        } else if self.active_workspace >= self.workspaces.len() {
            self.active_workspace = self.workspaces.len() - 1;
        } else if idx < self.active_workspace {
            self.active_workspace -= 1;
        }

        let focused = self.workspaces[self.active_workspace].focused_pane;
        self.focus_pane_editor(focused, window, cx);
        self.sync_file_tree_selection(cx);
        cx.notify();
    }
}
