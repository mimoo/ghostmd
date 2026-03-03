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

/// Root GPUI view for the GhostMD application.
pub struct GhostAppView {
    app: GhostApp,
    file_tree: Entity<FileTreeView>,
    editors: HashMap<PathBuf, Entity<EditorView>>,
    active_path: Option<PathBuf>,
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

        let mut view = Self {
            app,
            file_tree,
            editors: HashMap::new(),
            active_path: None,
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

    fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if !self.editors.contains_key(&path) {
            let p = path.clone();
            let editor = cx.new(|cx| {
                EditorView::new(p, window, cx)
            });
            self.editors.insert(path.clone(), editor);
            self.app.open_file(path.clone());
        }
        self.active_path = Some(path.clone());
        // Focus the newly active editor's input
        if let Some(editor) = self.editors.get(&path) {
            editor.update(cx, |e, cx| {
                e.focus_input(window, cx);
            });
        }
        cx.notify();
    }

    fn focus_active_editor(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = &self.active_path {
            if let Some(editor) = self.editors.get(path) {
                editor.update(cx, |e, cx| {
                    e.focus_input(window, cx);
                });
            }
        }
    }

    fn close_active_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = self.active_path.take() {
            // Save before closing
            if let Some(editor) = self.editors.get(&path) {
                editor.update(cx, |e, _cx| {
                    e.save(_cx).ok();
                });
            }
            self.editors.remove(&path);
            self.app.close_file(&path, 0);

            // Switch to another open file or None
            self.active_path = self.app.open_files.last().cloned();
            self.focus_active_editor(window, cx);
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

    fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let ghost = GhostTheme::default_dark();
        let tab_bar_bg = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
        let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);

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
            let is_active = self.active_path.as_ref() == Some(path);
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
                        this.active_path = Some(path_clone.clone());
                        this.focus_active_editor(window, cx);
                        cx.notify();
                    }))
                    .child(display),
            );
        }

        tabs
    }

    fn render_editor_area(&self) -> impl IntoElement {
        let ghost = GhostTheme::default_dark();
        let bg = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
        let fg = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);

        let mut area = div().flex_1().flex().flex_col().bg(bg).text_color(fg);

        if let Some(path) = &self.active_path {
            if let Some(editor) = self.editors.get(path) {
                area = area.child(editor.clone());
            }
        } else {
            area = area.child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb_to_hsla(
                        ghost.line_number.0,
                        ghost.line_number.1,
                        ghost.line_number.2,
                    ))
                    .child("Cmd+N to create a new note"),
            );
        }

        area
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

        div()
            .id("ghost-app")
            .size_full()
            .flex()
            .flex_row()
            .bg(bg)
            .track_focus(&self.focus_handle)
            // Action handlers
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::NewNote, window, cx| {
                let root = this.app.root.clone();
                let path = diary::new_diary_path(&root, "untitled");
                let note = Note::new(path.clone());
                note.ensure_dir().ok();
                note.save("").ok();
                this.open_file(path, window, cx);
                // Refresh tree to show new file
                this.file_tree.update(cx, |tree, cx| tree.refresh(cx));
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::Save, _window, cx| {
                if let Some(path) = this.active_path.clone() {
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
                if let Some(active) = &this.active_path {
                    if let Some(pos) = this.app.open_files.iter().position(|p| p == active) {
                        let next = (pos + 1) % this.app.open_files.len();
                        this.active_path = Some(this.app.open_files[next].clone());
                        this.focus_active_editor(window, cx);
                        cx.notify();
                    }
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::PrevTab, window, cx| {
                if let Some(active) = &this.active_path {
                    if let Some(pos) = this.app.open_files.iter().position(|p| p == active) {
                        let prev = if pos == 0 {
                            this.app.open_files.len() - 1
                        } else {
                            pos - 1
                        };
                        this.active_path = Some(this.app.open_files[prev].clone());
                        this.focus_active_editor(window, cx);
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
            // Layout — always render tree to preserve entity state, hide via size
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
                    .child(self.render_editor_area()),
            )
    }
}
