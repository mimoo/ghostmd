use std::path::{Path, PathBuf};

use gpui::prelude::FluentBuilder as _;
use gpui::*;

use crate::file_tree::FileTreePanel;
use crate::theme::{rgb_to_hsla, GhostTheme, ThemeName};

/// Event emitted when a file is selected in the tree.
pub struct FileSelected(pub PathBuf);

/// Event emitted when a file rename is requested (double-click on a file).
pub struct FileRenameRequested(pub PathBuf);

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
}

impl EventEmitter<FileSelected> for FileTreeView {}
impl EventEmitter<FileRenameRequested> for FileTreeView {}
impl EventEmitter<OpenInFinderRequested> for FileTreeView {}
impl EventEmitter<MoveToTrashRequested> for FileTreeView {}
impl EventEmitter<ContextMenuRequested> for FileTreeView {}

impl FileTreeView {
    pub fn new(root: PathBuf, cx: &mut Context<Self>) -> Self {
        let mut panel = FileTreePanel::new(root);
        panel.refresh().ok();

        let focus_handle = cx.focus_handle();

        Self {
            panel,
            focus_handle,
            selected_path: None,
            scroll_handle: ScrollHandle::new(),
            active_theme: ThemeName::default(),
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

            let row_bg = if is_selected { selection_bg } else { sidebar_bg };

            let chevron_label = if is_dir {
                if is_expanded { "\u{25be}" } else { "\u{25b8}" }
            } else {
                ""
            };

            let chevron_path = node_path.clone();
            let label_path = node_path.clone();
            let right_click_path = node_path.clone();

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
                        .text_xs()
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
                        .on_click(cx.listener(move |this: &mut Self, event: &ClickEvent, _window, cx| {
                            this.selected_path = Some(label_path.clone());
                            if is_dir {
                                // Just select the dir, don't toggle
                            } else {
                                cx.emit(FileSelected(label_path.clone()));
                                if event.click_count() >= 2 {
                                    cx.emit(FileRenameRequested(label_path.clone()));
                                }
                            }
                            cx.notify();
                        }))
                        .child(name),
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
