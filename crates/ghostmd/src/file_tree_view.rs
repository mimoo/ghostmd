use std::path::{Path, PathBuf};

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};

use crate::file_tree::FileTreePanel;
use crate::theme::{rgb_to_hsla, GhostTheme, ThemeName};

/// Event emitted when a file is selected in the tree.
pub struct FileSelected(pub PathBuf);

/// Event emitted when a file/folder has been renamed inline.
pub struct ItemRenamed {
    pub old_path: PathBuf,
    pub new_path: PathBuf,
}

/// Event emitted after a new item (note/folder) is created inline.
pub struct NewItemCreated(pub PathBuf);

/// Event emitted when "Open in Finder" is requested for a path.
pub struct OpenInFinderRequested(pub PathBuf);

/// Event emitted when "Move to Trash" is requested for a path.
pub struct MoveToTrashRequested(pub PathBuf);

/// Event emitted when a context menu should appear (right-click).
/// Contains the path and the window-relative position.
pub struct ContextMenuRequested(pub PathBuf, pub Point<Pixels>);

/// GPUI view wrapping the FileTreePanel state machine with a custom flat list.
pub struct FileTreeView {
    panel: FileTreePanel,
    focus_handle: FocusHandle,
    /// Currently selected path in the tree.
    selected_path: Option<PathBuf>,
    /// Scroll handle for the tree list.
    scroll_handle: ScrollHandle,
    /// Active theme name.
    active_theme: ThemeName,
    /// Input entity for inline rename/create.
    rename_input: Entity<InputState>,
    /// Path currently being edited (rename or new item).
    editing_path: Option<PathBuf>,
    /// True if creating a new item (not renaming existing).
    editing_is_new: bool,
    /// True if the new item is a note (vs folder).
    editing_is_note: bool,
}

impl EventEmitter<FileSelected> for FileTreeView {}
impl EventEmitter<ItemRenamed> for FileTreeView {}
impl EventEmitter<NewItemCreated> for FileTreeView {}
impl EventEmitter<OpenInFinderRequested> for FileTreeView {}
impl EventEmitter<MoveToTrashRequested> for FileTreeView {}
impl EventEmitter<ContextMenuRequested> for FileTreeView {}

impl FileTreeView {
    pub fn new(root: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut panel = FileTreePanel::new(root);
        panel.refresh().ok();

        let focus_handle = cx.focus_handle();

        let rename_input = cx.new(|cx| InputState::new(window, cx));

        // Subscribe to rename input events
        cx.subscribe_in(&rename_input, window, |this: &mut Self, _entity: &Entity<InputState>, event: &InputEvent, window, cx| {
            match event {
                InputEvent::PressEnter { .. } => {
                    this.finish_rename(window, cx);
                }
                InputEvent::Blur => {
                    if this.editing_path.is_some() {
                        this.finish_rename(window, cx);
                    }
                }
                _ => {}
            }
        })
        .detach();

        Self {
            panel,
            focus_handle,
            selected_path: None,
            scroll_handle: ScrollHandle::new(),
            active_theme: ThemeName::default(),
            rename_input,
            editing_path: None,
            editing_is_new: false,
            editing_is_note: false,
        }
    }

    /// Refresh the file tree from disk.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.panel.refresh().ok();
        cx.notify();
    }

    /// Programmatically select a file in the tree by path.
    pub fn select_file(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.selected_path = Some(path.to_path_buf());
        self.scroll_to_path(path);
        cx.notify();
    }

    /// Reveal a file in the tree: collapse non-ancestors, expand ancestors, scroll to file.
    pub fn reveal_file(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.panel.tree.reveal_path(path);
        self.selected_path = Some(path.to_path_buf());
        self.scroll_to_path(path);
        cx.notify();
    }

    /// Get the currently selected path.
    pub fn selected_path(&self) -> Option<&PathBuf> {
        self.selected_path.as_ref()
    }

    /// Whether the tree is currently in inline editing mode.
    pub fn is_editing(&self) -> bool {
        self.editing_path.is_some()
    }

    /// Set the active theme.
    pub fn set_theme(&mut self, name: ThemeName) {
        self.active_theme = name;
    }

    /// Scroll to the given path in the flat list.
    fn scroll_to_path(&self, path: &Path) {
        let flat = self.panel.tree.flatten();
        if let Some(idx) = flat.iter().position(|(_, node)| node.path() == path) {
            self.scroll_handle.scroll_to_item(idx);
        }
    }

    /// Start inline rename for an existing file or folder.
    pub fn start_rename(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let name = if path.is_dir() {
            path.file_name().unwrap_or_default().to_string_lossy().to_string()
        } else {
            path.file_stem().unwrap_or_default().to_string_lossy().to_string()
        };
        self.editing_path = Some(path.to_path_buf());
        self.editing_is_new = false;
        self.editing_is_note = path.is_file();
        // Expand parent so the item is visible
        self.panel.tree.reveal_path(path);
        self.rename_input.update(cx, |state, cx| {
            state.set_value(&name, window, cx);
            state.focus(window, cx);
        });
        self.scroll_to_path(path);
        cx.notify();
        cx.defer_in(window, |_this: &mut Self, window, cx| {
            window.dispatch_action(Box::new(gpui_component::input::SelectAll), cx);
        });
    }

    /// Start inline creation of a new note in the given directory.
    pub fn start_new_note(&mut self, parent_dir: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let name = "untitled";
        let temp_path = parent_dir.join(format!("{}.md", name));
        // Create the file on disk
        std::fs::create_dir_all(parent_dir).ok();
        std::fs::write(&temp_path, "").ok();
        self.panel.refresh().ok();
        self.panel.tree.reveal_path(&temp_path);
        self.editing_path = Some(temp_path.clone());
        self.editing_is_new = true;
        self.editing_is_note = true;
        self.selected_path = Some(temp_path.clone());
        self.rename_input.update(cx, |state, cx| {
            state.set_value(name, window, cx);
            state.focus(window, cx);
        });
        self.scroll_to_path(&temp_path);
        cx.notify();
        cx.defer_in(window, |_this: &mut Self, window, cx| {
            window.dispatch_action(Box::new(gpui_component::input::SelectAll), cx);
        });
    }

    /// Start inline creation of a new folder in the given directory.
    pub fn start_new_folder(&mut self, parent_dir: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let name = "new-folder";
        let temp_path = parent_dir.join(name);
        std::fs::create_dir_all(&temp_path).ok();
        self.panel.refresh().ok();
        self.panel.tree.reveal_path(&temp_path);
        self.editing_path = Some(temp_path.clone());
        self.editing_is_new = true;
        self.editing_is_note = false;
        self.selected_path = Some(temp_path.clone());
        self.rename_input.update(cx, |state, cx| {
            state.set_value(name, window, cx);
            state.focus(window, cx);
        });
        self.scroll_to_path(&temp_path);
        cx.notify();
        cx.defer_in(window, |_this: &mut Self, window, cx| {
            window.dispatch_action(Box::new(gpui_component::input::SelectAll), cx);
        });
    }

    /// Finish inline rename: apply the new name on disk.
    fn finish_rename(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(old_path) = self.editing_path.take() else { return };
        let new_name = self.rename_input.read(cx).value().to_string().trim().to_string();
        let is_new = self.editing_is_new;
        let is_note = self.editing_is_note;
        self.editing_is_new = false;
        self.editing_is_note = false;

        if new_name.is_empty() {
            // Empty name: cancel (delete if new)
            if is_new {
                if old_path.is_dir() {
                    std::fs::remove_dir_all(&old_path).ok();
                } else {
                    std::fs::remove_file(&old_path).ok();
                }
            }
            self.panel.refresh().ok();
            cx.notify();
            return;
        }

        let new_filename = if is_note && !new_name.ends_with(".md") {
            format!("{}.md", new_name)
        } else {
            new_name
        };
        let new_path = old_path.parent()
            .unwrap_or(&old_path)
            .join(&new_filename);

        if new_path != old_path {
            if std::fs::rename(&old_path, &new_path).is_ok() {
                self.panel.refresh().ok();
                self.panel.tree.reveal_path(&new_path);
                self.selected_path = Some(new_path.clone());
                self.scroll_to_path(&new_path);
                if is_new {
                    cx.emit(NewItemCreated(new_path));
                } else {
                    cx.emit(ItemRenamed { old_path, new_path });
                }
            } else {
                // Rename failed, refresh anyway
                self.panel.refresh().ok();
            }
        } else {
            // Name unchanged
            if is_new {
                cx.emit(NewItemCreated(new_path));
            }
            self.panel.refresh().ok();
        }
        cx.notify();
    }

    /// Cancel inline rename.
    pub fn cancel_rename(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(old_path) = self.editing_path.take() {
            if self.editing_is_new {
                if old_path.is_dir() {
                    std::fs::remove_dir_all(&old_path).ok();
                } else {
                    std::fs::remove_file(&old_path).ok();
                }
                self.panel.refresh().ok();
            }
        }
        self.editing_is_new = false;
        self.editing_is_note = false;
        cx.notify();
    }
}

impl Focusable for FileTreeView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FileTreeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ghost = GhostTheme::from_name(self.active_theme);
        let sidebar_bg = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
        let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);
        let fg = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
        let selection_bg = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);
        let hint_fg = rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2);

        let flat = self.panel.tree.flatten();
        let selected = self.selected_path.clone();
        let root_path = self.panel.tree.root.clone();
        let editing_path = self.editing_path.clone();

        let mut list = div()
            .id("file-tree-list")
            .flex_1()
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .on_mouse_down(MouseButton::Right, cx.listener(move |_this: &mut Self, event: &MouseDownEvent, _window, cx| {
                // Right-click on empty area → context menu for root
                cx.emit(ContextMenuRequested(root_path.clone(), event.position));
            }));

        for (i, (depth, node)) in flat.iter().enumerate() {
            let node_path = node.path().to_path_buf();
            let is_selected = selected.as_ref() == Some(&node_path);
            let is_dir = node.is_dir();
            let is_expanded = node.is_expanded();
            let name = node.name().to_string();
            let indent = *depth as f32 * 16.0;
            let is_editing = editing_path.as_ref() == Some(&node_path);

            let row_bg = if is_selected { selection_bg } else { sidebar_bg };

            let chevron_label = if is_dir {
                if is_expanded { "\u{25bc}" } else { "\u{25b6}" }
            } else {
                ""
            };

            let chevron_path = node_path.clone();
            let label_path = node_path.clone();
            let right_click_path = node_path.clone();

            let label_child: AnyElement = if is_editing {
                // Inline rename input
                Input::new(&self.rename_input)
                    .appearance(false)
                    .text_size(px(13.0))
                    .w(px(200.0))
                    .into_any_element()
            } else {
                div()
                    .text_color(fg)
                    .child(name)
                    .into_any_element()
            };

            let row = div()
                .id(ElementId::NamedInteger("tree-row".into(), i as u64))
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .bg(row_bg)
                .on_mouse_down(MouseButton::Right, cx.listener(move |this: &mut Self, event: &MouseDownEvent, _window, cx| {
                    this.selected_path = Some(right_click_path.clone());
                    cx.emit(ContextMenuRequested(right_click_path.clone(), event.position));
                    cx.stop_propagation();
                }))
                // Chevron area (for toggling dirs)
                .child(
                    div()
                        .id(ElementId::NamedInteger("tree-chevron".into(), i as u64))
                        .w(px(indent + 20.0))
                        .pl(px(indent + 4.0))
                        .text_color(hint_fg)
                        .flex_shrink_0()
                        .when(is_dir, |d| {
                            d.cursor_pointer()
                                .on_click(cx.listener(move |this: &mut Self, _event: &ClickEvent, _window, cx| {
                                    this.panel.tree.toggle_dir(&chevron_path);
                                    cx.notify();
                                }))
                        })
                        .child(chevron_label),
                )
                // Label area
                .child(
                    div()
                        .id(ElementId::NamedInteger("tree-label".into(), i as u64))
                        .flex_1()
                        .text_sm()
                        .text_color(fg)
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .cursor_pointer()
                        .py(px(2.0))
                        .when(!is_editing, |d| {
                            d.on_click(cx.listener(move |this: &mut Self, event: &ClickEvent, window, cx| {
                                this.selected_path = Some(label_path.clone());
                                if is_dir {
                                    if event.click_count() >= 2 {
                                        this.start_rename(&label_path, window, cx);
                                    }
                                } else {
                                    cx.emit(FileSelected(label_path.clone()));
                                    if event.click_count() >= 2 {
                                        this.start_rename(&label_path, window, cx);
                                    }
                                }
                                cx.notify();
                            }))
                        })
                        .child(label_child),
                );

            list = list.child(row);
        }

        div()
            .size_full()
            .relative()
            .bg(sidebar_bg)
            .border_r_1()
            .border_color(border_color)
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .child(
                div()
                    .p(px(8.0))
                    .text_sm()
                    .flex_shrink_0()
                    .text_color(hint_fg)
                    .child("ghostmd"),
            )
            .child(list)
    }
}
