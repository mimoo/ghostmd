use std::collections::BTreeSet;
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
    /// Currently selected paths in the tree (supports multi-select).
    selected_paths: BTreeSet<PathBuf>,
    /// The "anchor" path for shift-click range selection.
    anchor_path: Option<PathBuf>,
    /// The last-clicked path (primary selection for single operations).
    last_clicked: Option<PathBuf>,
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
            selected_paths: BTreeSet::new(),
            anchor_path: None,
            last_clicked: None,
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

    /// Programmatically select a file in the tree by path (single selection).
    pub fn select_file(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.selected_paths.clear();
        self.selected_paths.insert(path.to_path_buf());
        self.anchor_path = Some(path.to_path_buf());
        self.last_clicked = Some(path.to_path_buf());
        self.scroll_to_path(path);
        cx.notify();
    }

    /// Reveal a file in the tree: expand ancestors, scroll to file.
    pub fn reveal_file(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.panel.tree.reveal_path(path);
        self.selected_paths.clear();
        self.selected_paths.insert(path.to_path_buf());
        self.anchor_path = Some(path.to_path_buf());
        self.last_clicked = Some(path.to_path_buf());
        self.scroll_to_path(path);
        cx.notify();
    }

    /// Get the currently selected path (primary/last-clicked, for single-file operations).
    pub fn selected_path(&self) -> Option<&PathBuf> {
        self.last_clicked.as_ref()
            .filter(|p| self.selected_paths.contains(*p))
            .or_else(|| self.selected_paths.iter().next())
    }

    /// Get all selected paths.
    pub fn selected_paths(&self) -> &BTreeSet<PathBuf> {
        &self.selected_paths
    }

    /// Whether the tree is currently in inline editing mode.
    pub fn is_editing(&self) -> bool {
        self.editing_path.is_some()
    }

    /// Set the active theme.
    pub fn set_theme(&mut self, name: ThemeName) {
        self.active_theme = name;
    }

    /// Collapse all directories.
    pub fn collapse_all(&mut self, cx: &mut Context<Self>) {
        self.panel.tree.collapse_all();
        cx.notify();
    }

    /// Expand all directories.
    pub fn expand_all(&mut self, cx: &mut Context<Self>) {
        self.panel.tree.expand_all();
        cx.notify();
    }

    /// Scroll to the given path in the flat list.
    fn scroll_to_path(&self, path: &Path) {
        let flat = self.panel.tree.flatten();
        if let Some(idx) = flat.iter().position(|(_, node)| node.path() == path) {
            self.scroll_handle.scroll_to_item(idx);
        }
    }

    /// Handle a click on a tree item, supporting multi-select with cmd/shift.
    fn handle_click(&mut self, path: &Path, modifiers: &Modifiers) {
        let flat = self.panel.tree.flatten();
        let flat_paths: Vec<PathBuf> = flat.iter().map(|(_, n)| n.path().to_path_buf()).collect();

        if modifiers.platform {
            // Cmd-click: toggle individual item
            let pb = path.to_path_buf();
            if self.selected_paths.contains(&pb) {
                self.selected_paths.remove(&pb);
            } else {
                self.selected_paths.insert(pb);
            }
            self.anchor_path = Some(path.to_path_buf());
            self.last_clicked = Some(path.to_path_buf());
        } else if modifiers.shift {
            // Shift-click: select contiguous range from anchor
            if let Some(anchor) = &self.anchor_path {
                let anchor_idx = flat_paths.iter().position(|p| p == anchor);
                let click_idx = flat_paths.iter().position(|p| p == path);
                if let (Some(a), Some(c)) = (anchor_idx, click_idx) {
                    let (start, end) = if a <= c { (a, c) } else { (c, a) };
                    self.selected_paths.clear();
                    for p in &flat_paths[start..=end] {
                        self.selected_paths.insert(p.clone());
                    }
                }
            } else {
                self.selected_paths.clear();
                self.selected_paths.insert(path.to_path_buf());
                self.anchor_path = Some(path.to_path_buf());
            }
            self.last_clicked = Some(path.to_path_buf());
        } else {
            // Plain click: single selection
            self.selected_paths.clear();
            self.selected_paths.insert(path.to_path_buf());
            self.anchor_path = Some(path.to_path_buf());
            self.last_clicked = Some(path.to_path_buf());
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
        self.panel.tree.reveal_path(path);
        self.rename_input.update(cx, |state, cx| {
            state.set_value(&name, window, cx);
            state.focus(window, cx);
        });
        self.scroll_to_path(path);
        cx.notify();
        let input = self.rename_input.clone();
        cx.defer_in(window, move |_this: &mut Self, window, cx| {
            input.update(cx, |state, cx| state.focus(window, cx));
            window.dispatch_action(Box::new(gpui_component::input::SelectAll), cx);
        });
    }

    /// Start inline creation of a new note in the given directory.
    pub fn start_new_note(&mut self, parent_dir: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let name = "untitled";
        let temp_path = parent_dir.join(format!("{}.md", name));
        std::fs::create_dir_all(parent_dir).ok();
        std::fs::write(&temp_path, "").ok();
        self.panel.refresh().ok();
        // Expand the parent directory so the new file is visible
        self.panel.tree.reveal_path(&temp_path);
        self.editing_path = Some(temp_path.clone());
        self.editing_is_new = true;
        self.editing_is_note = true;
        self.selected_paths.clear();
        self.selected_paths.insert(temp_path.clone());
        self.last_clicked = Some(temp_path.clone());
        self.rename_input.update(cx, |state, cx| {
            state.set_value(name, window, cx);
            state.focus(window, cx);
        });
        self.scroll_to_path(&temp_path);
        cx.notify();
        let input = self.rename_input.clone();
        cx.defer_in(window, move |_this: &mut Self, window, cx| {
            input.update(cx, |state, cx| state.focus(window, cx));
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
        self.selected_paths.clear();
        self.selected_paths.insert(temp_path.clone());
        self.last_clicked = Some(temp_path.clone());
        self.rename_input.update(cx, |state, cx| {
            state.set_value(name, window, cx);
            state.focus(window, cx);
        });
        self.scroll_to_path(&temp_path);
        cx.notify();
        let input = self.rename_input.clone();
        cx.defer_in(window, move |_this: &mut Self, window, cx| {
            input.update(cx, |state, cx| state.focus(window, cx));
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
                self.selected_paths.clear();
                self.selected_paths.insert(new_path.clone());
                self.last_clicked = Some(new_path.clone());
                self.scroll_to_path(&new_path);
                if is_new {
                    cx.emit(NewItemCreated(new_path));
                } else {
                    cx.emit(ItemRenamed { old_path, new_path });
                }
            } else {
                self.panel.refresh().ok();
            }
        } else {
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
        let selected = self.selected_paths.clone();
        let root_path = self.panel.tree.root.clone();
        let editing_path = self.editing_path.clone();

        let mut list = div()
            .id("file-tree-list")
            .flex_1()
            .overflow_y_scroll()
            .track_scroll(&self.scroll_handle)
            .on_mouse_down(MouseButton::Right, cx.listener(move |_this: &mut Self, event: &MouseDownEvent, _window, cx| {
                cx.emit(ContextMenuRequested(root_path.clone(), event.position));
            }));

        for (i, (depth, node)) in flat.iter().enumerate() {
            let node_path = node.path().to_path_buf();
            let is_selected = selected.contains(&node_path);
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
                Input::new(&self.rename_input)
                    .appearance(true)
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
                    // If right-clicked item is not in selection, select only it
                    if !this.selected_paths.contains(&right_click_path) {
                        this.selected_paths.clear();
                        this.selected_paths.insert(right_click_path.clone());
                        this.last_clicked = Some(right_click_path.clone());
                    }
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
                                let was_selected = this.selected_paths.contains(&label_path);
                                this.handle_click(&label_path, &event.modifiers());

                                if is_dir {
                                    // Toggle dir on click if already selected (plain click only)
                                    if was_selected && !event.modifiers().platform && !event.modifiers().shift {
                                        this.panel.tree.toggle_dir(&label_path);
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
