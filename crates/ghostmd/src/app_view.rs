use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use gpui::prelude::FluentBuilder as _;
use gpui::*;

use crate::app::GhostApp;
use crate::editor_view::EditorView;
use crate::file_tree_view::{FileSelected, FileTreeView};
use crate::keybindings;
use crate::theme::{rgb_to_hsla, GhostTheme};

use ghostmd_core::diary;
use ghostmd_core::note::Note;

// ---------------------------------------------------------------------------
// Split tree
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum SplitNode {
    Leaf(usize),
    Split {
        direction: SplitDirection,
        left: Box<SplitNode>,
        right: Box<SplitNode>,
    },
}

#[derive(Clone, Copy, PartialEq)]
enum SplitDirection {
    Vertical,   // side-by-side (cmd-d)
    Horizontal, // top/bottom  (cmd-shift-d)
}

impl SplitNode {
    /// Collect all leaf pane IDs in left-to-right / top-to-bottom order.
    fn leaves(&self) -> Vec<usize> {
        match self {
            SplitNode::Leaf(id) => vec![*id],
            SplitNode::Split { left, right, .. } => {
                let mut v = left.leaves();
                v.extend(right.leaves());
                v
            }
        }
    }

    /// Replace the leaf with `pane_id` by a split containing `pane_id` and `new_id`.
    fn split_leaf(&mut self, pane_id: usize, new_id: usize, direction: SplitDirection) {
        match self {
            SplitNode::Leaf(id) if *id == pane_id => {
                *self = SplitNode::Split {
                    direction,
                    left: Box::new(SplitNode::Leaf(pane_id)),
                    right: Box::new(SplitNode::Leaf(new_id)),
                };
            }
            SplitNode::Split { left, right, .. } => {
                left.split_leaf(pane_id, new_id, direction);
                right.split_leaf(pane_id, new_id, direction);
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Pane
// ---------------------------------------------------------------------------

struct Pane {
    active_path: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// GhostAppView
// ---------------------------------------------------------------------------

/// Root GPUI view for the GhostMD application.
pub struct GhostAppView {
    app: GhostApp,
    file_tree: Entity<FileTreeView>,
    editors: HashMap<PathBuf, Entity<EditorView>>,
    panes: HashMap<usize, Pane>,
    next_pane_id: usize,
    focused_pane: usize,
    split_root: SplitNode,
    focus_handle: FocusHandle,
}

impl GhostAppView {
    pub fn new(root: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let app = GhostApp::new(root.clone());

        let file_tree = cx.new(|cx| FileTreeView::new(root.clone(), cx));

        // Subscribe to file selection events from the tree (with window access)
        cx.subscribe_in(&file_tree, window, |this: &mut Self, _entity, event: &FileSelected, window, cx| {
            this.open_file(event.0.clone(), window, cx);
        })
        .detach();

        let focus_handle = cx.focus_handle();

        let pane_id = 0;
        let mut panes = HashMap::new();
        panes.insert(pane_id, Pane { active_path: None });

        let mut view = Self {
            app,
            file_tree,
            editors: HashMap::new(),
            panes,
            next_pane_id: 1,
            focused_pane: pane_id,
            split_root: SplitNode::Leaf(pane_id),
            focus_handle,
        };

        // Open a new diary note by default
        let diary_path = diary::new_diary_path(&root, "untitled");
        let note = Note::new(diary_path.clone());
        note.ensure_dir().ok();
        note.save("").ok();
        view.open_file(diary_path, window, cx);

        // Start auto-save timer
        cx.spawn(async |this: WeakEntity<GhostAppView>, cx: &mut AsyncApp| {
            loop {
                cx.background_executor().timer(Duration::from_millis(500)).await;
                let result = this.update(cx, |this: &mut GhostAppView, cx: &mut Context<GhostAppView>| {
                    this.auto_save(cx);
                });
                if result.is_err() {
                    break;
                }
            }
        })
        .detach();

        view
    }

    /// Ensure an editor exists for `path` and register it in the tab bar.
    fn ensure_editor(&mut self, path: &PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if !self.editors.contains_key(path) {
            let p = path.clone();
            let editor = cx.new(|cx| EditorView::new(p, window, cx));
            self.editors.insert(path.clone(), editor);
            self.app.open_file(path.clone());
        }
    }

    /// Open a file: ensure editor exists, set it as active in the focused pane, and focus.
    fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.ensure_editor(&path, window, cx);
        if let Some(pane) = self.panes.get_mut(&self.focused_pane) {
            pane.active_path = Some(path.clone());
        }
        self.focus_pane_editor(self.focused_pane, window, cx);
        cx.notify();
    }

    /// Create a new diary note and open it in the focused pane (cmd-n).
    fn new_note_in_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.app.root.clone();
        let path = diary::new_diary_path(&root, "untitled");
        let note = Note::new(path.clone());
        note.ensure_dir().ok();
        note.save("").ok();
        self.open_file(path, window, cx);
        self.file_tree.update(cx, |tree, cx| tree.refresh(cx));
    }

    /// Create a new diary note and open it as a new tab (cmd-shift-n).
    /// Also opens it in the focused pane.
    fn new_note_as_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.new_note_in_pane(window, cx);
    }

    /// Focus the editor shown in the given pane.
    fn focus_pane_editor(&self, pane_id: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(pane) = self.panes.get(&pane_id) {
            if let Some(path) = &pane.active_path {
                if let Some(editor) = self.editors.get(path) {
                    editor.update(cx, |e, cx| {
                        e.focus_input(window, cx);
                    });
                }
            }
        }
    }

    /// Split the focused pane, creating a new pane showing the same file.
    fn split(&mut self, direction: SplitDirection, window: &mut Window, cx: &mut Context<Self>) {
        let current_path = self.panes.get(&self.focused_pane)
            .and_then(|p| p.active_path.clone());

        let new_id = self.next_pane_id;
        self.next_pane_id += 1;
        self.panes.insert(new_id, Pane { active_path: current_path });

        self.split_root.split_leaf(self.focused_pane, new_id, direction);
        self.focused_pane = new_id;
        self.focus_pane_editor(new_id, window, cx);
        cx.notify();
    }

    /// Navigate focus to an adjacent pane in the given direction.
    fn focus_pane_direction(&mut self, dx: i32, dy: i32, window: &mut Window, cx: &mut Context<Self>) {
        let leaves = self.split_root.leaves();
        if leaves.len() <= 1 {
            return;
        }
        if let Some(pos) = leaves.iter().position(|&id| id == self.focused_pane) {
            // For simplicity, use linear ordering: left/up = previous, right/down = next
            let offset = if dx > 0 || dy > 0 { 1i32 } else { -1i32 };
            let new_pos = (pos as i32 + offset).rem_euclid(leaves.len() as i32) as usize;
            self.focused_pane = leaves[new_pos];
            self.focus_pane_editor(self.focused_pane, window, cx);
            cx.notify();
        }
    }

    fn close_active_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let path = self.panes.get(&self.focused_pane)
            .and_then(|p| p.active_path.clone());

        if let Some(path) = path {
            // Save before closing
            if let Some(editor) = self.editors.get(&path) {
                editor.update(cx, |e, _cx| {
                    e.save(_cx).ok();
                });
            }

            // Check if any other pane still shows this file
            let still_used = self.panes.iter()
                .any(|(&id, p)| id != self.focused_pane && p.active_path.as_ref() == Some(&path));

            if !still_used {
                self.editors.remove(&path);
            }
            self.app.close_file(&path, 0);

            // Clear the focused pane, switch to another open file or None
            if let Some(pane) = self.panes.get_mut(&self.focused_pane) {
                pane.active_path = self.app.open_files.last().cloned();
            }
            self.focus_pane_editor(self.focused_pane, window, cx);
            cx.notify();
        }
    }

    fn auto_save(&mut self, cx: &mut Context<Self>) {
        for editor in self.editors.values() {
            editor.update(cx, |e, cx| {
                if e.should_auto_save(300) {
                    e.save(cx).ok();
                }
            });
        }
    }

    /// The path currently active in the focused pane.
    fn focused_active_path(&self) -> Option<PathBuf> {
        self.panes.get(&self.focused_pane)
            .and_then(|p| p.active_path.clone())
    }

    fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let ghost = GhostTheme::default_dark();
        let tab_bar_bg = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
        let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);

        let focused_path = self.focused_active_path();

        let mut tabs = div()
            .w_full()
            .h(px(36.0))
            .flex()
            .flex_row()
            .items_center()
            .bg(tab_bar_bg)
            .border_b_1()
            .border_color(border_color)
            .overflow_x_hidden();

        for (i, path) in self.app.open_files.iter().enumerate() {
            let is_active = focused_path.as_ref() == Some(path);
            let title = Note::title_from_path(path);
            let dirty = self
                .editors
                .get(path)
                .map(|e| e.read(cx).dirty)
                .unwrap_or(false);

            let display = if dirty {
                format!("{} *", title)
            } else {
                title
            };

            let tab_bg = if is_active {
                rgb_to_hsla(ghost.tab_active.0, ghost.tab_active.1, ghost.tab_active.2)
            } else {
                rgb_to_hsla(ghost.tab_inactive.0, ghost.tab_inactive.1, ghost.tab_inactive.2)
            };
            let fg = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);

            let path_clone = path.clone();
            tabs = tabs.child(
                div()
                    .id(ElementId::NamedInteger("tab".into(), i as u64))
                    .px(px(12.0))
                    .py(px(6.0))
                    .text_sm()
                    .bg(tab_bg)
                    .text_color(fg)
                    .cursor_pointer()
                    .on_click(cx.listener(move |this: &mut Self, _event, window, cx| {
                        if let Some(pane) = this.panes.get_mut(&this.focused_pane) {
                            pane.active_path = Some(path_clone.clone());
                        }
                        this.focus_pane_editor(this.focused_pane, window, cx);
                        cx.notify();
                    }))
                    .child(display),
            );
        }

        tabs
    }

    fn render_split_node(&self, node: &SplitNode) -> Div {
        let ghost = GhostTheme::default_dark();
        let bg = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
        let fg = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
        let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);
        let focused_highlight = rgb_to_hsla(ghost.tab_active.0, ghost.tab_active.1, ghost.tab_active.2);

        match node {
            SplitNode::Leaf(pane_id) => {
                let is_focused = *pane_id == self.focused_pane;
                let mut pane_div = div()
                    .flex_1()
                    .min_w(px(100.0))
                    .min_h(px(100.0))
                    .bg(bg)
                    .text_color(fg);

                // Show a subtle top border on the focused pane when there are splits
                if is_focused && self.panes.len() > 1 {
                    pane_div = pane_div.border_2().border_color(focused_highlight);
                } else if self.panes.len() > 1 {
                    pane_div = pane_div.border_1().border_color(border_color);
                }

                if let Some(pane) = self.panes.get(pane_id) {
                    if let Some(path) = &pane.active_path {
                        if let Some(editor) = self.editors.get(path) {
                            pane_div = pane_div.child(editor.clone());
                        }
                    }
                }

                pane_div
            }
            SplitNode::Split { direction, left, right } => {
                let container = div()
                    .flex_1()
                    .flex()
                    .when(*direction == SplitDirection::Vertical, |d| d.flex_row())
                    .when(*direction == SplitDirection::Horizontal, |d| d.flex_col());

                container
                    .child(self.render_split_node(left))
                    .child(self.render_split_node(right))
            }
        }
    }
}

impl Focusable for GhostAppView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GhostAppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ghost = GhostTheme::default_dark();
        let bg = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
        let sidebar_visible = self.app.sidebar_visible;
        let split_root = self.split_root.clone();

        div()
            .id("ghost-app")
            .size_full()
            .flex()
            .flex_row()
            .bg(bg)
            .track_focus(&self.focus_handle)
            // Action handlers
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::NewNote, window, cx| {
                this.new_note_in_pane(window, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::NewTab, window, cx| {
                this.new_note_as_tab(window, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::Save, _window, cx| {
                if let Some(path) = this.focused_active_path() {
                    if let Some(editor) = this.editors.get(&path) {
                        editor.update(cx, |e, cx| {
                            e.save(cx).ok();
                        });
                        cx.notify();
                    }
                }
            }))
            .on_action(cx.listener(|_this: &mut Self, _action: &keybindings::Quit, _window, cx| {
                cx.quit();
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::CloseTab, window, cx| {
                this.close_active_file(window, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::RestoreTab, window, cx| {
                if let Some(path) = this.app.restore_tab() {
                    this.open_file(path, window, cx);
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::NextTab, window, cx| {
                let focused_path = this.focused_active_path();
                if let Some(active) = &focused_path {
                    if let Some(pos) = this.app.open_files.iter().position(|p| p == active) {
                        let next = (pos + 1) % this.app.open_files.len();
                        let new_path = this.app.open_files[next].clone();
                        if let Some(pane) = this.panes.get_mut(&this.focused_pane) {
                            pane.active_path = Some(new_path);
                        }
                        this.focus_pane_editor(this.focused_pane, window, cx);
                        cx.notify();
                    }
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::PrevTab, window, cx| {
                let focused_path = this.focused_active_path();
                if let Some(active) = &focused_path {
                    if let Some(pos) = this.app.open_files.iter().position(|p| p == active) {
                        let prev = if pos == 0 {
                            this.app.open_files.len() - 1
                        } else {
                            pos - 1
                        };
                        let new_path = this.app.open_files[prev].clone();
                        if let Some(pane) = this.panes.get_mut(&this.focused_pane) {
                            pane.active_path = Some(new_path);
                        }
                        this.focus_pane_editor(this.focused_pane, window, cx);
                        cx.notify();
                    }
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::ToggleSidebar, _window, cx| {
                this.app.toggle_sidebar();
                cx.notify();
            }))
            .on_action(cx.listener(|_this: &mut Self, _action: &keybindings::OpenFileFinder, _window, _cx| {
                // TODO: wire up file finder overlay
            }))
            .on_action(cx.listener(|_this: &mut Self, _action: &keybindings::OpenContentSearch, _window, _cx| {
                // TODO: wire up content search overlay
            }))
            .on_action(cx.listener(|_this: &mut Self, _action: &keybindings::OpenCommandPalette, _window, _cx| {
                // TODO: wire up command palette overlay
            }))
            // Splits
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::SplitRight, window, cx| {
                this.split(SplitDirection::Vertical, window, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::SplitDown, window, cx| {
                this.split(SplitDirection::Horizontal, window, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::FocusPaneRight, window, cx| {
                this.focus_pane_direction(1, 0, window, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::FocusPaneLeft, window, cx| {
                this.focus_pane_direction(-1, 0, window, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::FocusPaneDown, window, cx| {
                this.focus_pane_direction(0, 1, window, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::FocusPaneUp, window, cx| {
                this.focus_pane_direction(0, -1, window, cx);
            }))
            // Layout
            .child(
                div()
                    .when(!sidebar_visible, |d| d.w(px(0.0)).overflow_hidden())
                    .child(self.file_tree.clone()),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(self.render_tab_bar(cx))
                    .child(self.render_split_node(&split_root)),
            )
    }
}
