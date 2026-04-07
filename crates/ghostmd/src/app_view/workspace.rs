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
        panes.insert(pane_id, Pane { active_path: None, editor: None, path_history: Vec::new() });

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

    /// Tear off a workspace tab into a new OS window.
    pub(crate) fn tear_off_tab(&mut self, ws_idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        // Don't tear off the last tab — keep at least one workspace per window
        if ws_idx >= self.workspaces.len() || self.workspaces.len() <= 1 {
            self.tab_drag_active = None;
            return;
        }
        self.tab_drag_active = None;

        // Serialize workspace state before removing
        let ws = &self.workspaces[ws_idx];
        let session_ws = SessionWorkspace {
            title: ws.title.clone(),
            split_root: ws.split_root.to_session(&ws.panes),
            focused_pane_idx: {
                let leaves = ws.split_root.leaves();
                leaves.iter().position(|&id| id == ws.focused_pane).unwrap_or(0)
            },
        };

        // Save editors before removing
        let editors: Vec<Entity<EditorView>> = self.workspaces[ws_idx].panes.values()
            .filter_map(|p| p.editor.clone())
            .collect();
        for editor in &editors {
            editor.update(cx, |e, cx| { e.save(cx).ok(); });
        }

        // Remove workspace from current window
        self.workspaces.remove(ws_idx);
        if self.active_workspace >= self.workspaces.len() {
            self.active_workspace = self.workspaces.len() - 1;
        } else if ws_idx < self.active_workspace {
            self.active_workspace -= 1;
        }

        let focused = self.workspaces[self.active_workspace].focused_pane;
        self.focus_pane_editor(focused, window, cx);
        self.sync_file_tree_selection(cx);
        self.save_session(cx);
        cx.notify();

        // Open new window with the torn-off workspace
        let root = self.root.clone();
        let theme = self.active_theme;
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
                        let app_view = cx.new(|cx| {
                            let mut view = GhostAppView::new(root, false, window, cx);

                            // Replace the default empty workspace with the torn-off one
                            view.workspaces.clear();

                            let ws_id = view.next_workspace_id;
                            view.next_workspace_id += 1;

                            let mut panes = HashMap::new();
                            let mut editors_out = Vec::new();
                            let split_root = restore_split_node(
                                &session_ws.split_root,
                                &mut view.next_pane_id,
                                &mut panes,
                                &mut editors_out,
                                view.syntax_highlight,
                                window,
                                cx,
                            );

                            let leaves = split_root.leaves();
                            let focused_pane = if session_ws.focused_pane_idx < leaves.len() {
                                leaves[session_ws.focused_pane_idx]
                            } else {
                                leaves.first().copied().unwrap_or(0)
                            };

                            view.workspaces.push(Workspace {
                                id: ws_id,
                                title: session_ws.title,
                                split_root,
                                panes,
                                focused_pane,
                                pane_focus_history: Vec::new(),
                            });
                            view.active_workspace = 0;

                            // Subscribe to restored editors for cross-pane sync
                            for editor in editors_out {
                                view.subscribe_editor(&editor, window, cx);
                            }

                            // Apply the same theme
                            view.active_theme = theme;
                            view.theme = crate::theme::ResolvedTheme::from_name(theme);
                            view.file_tree.update(cx, |tree, _cx| {
                                tree.set_theme(theme);
                            });

                            // Focus the restored workspace
                            view.focus_pane_editor(focused_pane, window, cx);

                            view
                        });
                        cx.new(|cx| Root::new(app_view, window, cx))
                    },
                ).ok();
            }).ok();
        })
        .detach();
    }

    /// Reorder a workspace tab from `from` index to `to` index via drag-and-drop.
    pub(crate) fn reorder_workspace(&mut self, from: usize, to: usize, _window: &mut Window, cx: &mut Context<Self>) {
        if from == to || from >= self.workspaces.len() || to >= self.workspaces.len() {
            return;
        }
        let ws = self.workspaces.remove(from);
        self.workspaces.insert(to, ws);
        // Keep active_workspace pointing at the same workspace
        if self.active_workspace == from {
            self.active_workspace = to;
        } else if from < to {
            // Moved right: items in (from..=to) shifted left by 1
            if self.active_workspace > from && self.active_workspace <= to {
                self.active_workspace -= 1;
            }
        } else {
            // Moved left: items in (to..from) shifted right by 1
            if self.active_workspace >= to && self.active_workspace < from {
                self.active_workspace += 1;
            }
        }
        self.save_session(cx);
        cx.notify();
    }

    /// Switch to workspace at index.
    pub(crate) fn switch_workspace(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
        if idx < self.workspaces.len() {
            self.push_nav_history(cx);
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
        ws.panes.insert(new_id, Pane { active_path: None, editor: None, path_history: Vec::new() });
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
        self.dismiss_overlays(window, cx);
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
            self.push_nav_history(cx);
            let ws = self.active_ws_mut();
            ws.pane_focus_history.push(ws.focused_pane);
            ws.focused_pane = new_id;
            self.focus_pane_editor(new_id, window, cx);
            self.sync_file_tree_selection(cx);
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
