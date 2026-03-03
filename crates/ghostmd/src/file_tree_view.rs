use std::path::{Path, PathBuf};

use gpui::*;
use gpui_component::list::ListItem;
use gpui_component::tree::{tree, TreeItem, TreeState};

use crate::file_tree::FileTreePanel;
use crate::theme::rgb_to_hsla;
use ghostmd_core::tree::TreeNode;

/// Event emitted when a file is selected in the tree.
pub struct FileSelected(pub PathBuf);

/// Event emitted when a file rename is requested (double-click on a file).
pub struct FileRenameRequested(pub PathBuf);

/// Event emitted when "Open in Finder" is requested for a path.
pub struct OpenInFinderRequested(pub PathBuf);

/// Event emitted when "Move to Trash" is requested for a path.
pub struct MoveToTrashRequested(pub PathBuf);

/// GPUI view wrapping the FileTreePanel state machine with a gpui-component Tree.
pub struct FileTreeView {
    panel: FileTreePanel,
    tree_state: Entity<TreeState>,
    focus_handle: FocusHandle,
    /// Last known selected entry ID (to detect changes).
    last_selected_id: Option<String>,
    /// Flat list of tree item IDs in display order (for path→index lookups).
    flat_ids: Vec<String>,
    /// Context menu state: (path, position) when visible.
    context_menu: Option<(PathBuf, Point<Pixels>)>,
}

impl EventEmitter<FileSelected> for FileTreeView {}
impl EventEmitter<FileRenameRequested> for FileTreeView {}
impl EventEmitter<OpenInFinderRequested> for FileTreeView {}
impl EventEmitter<MoveToTrashRequested> for FileTreeView {}

impl FileTreeView {
    pub fn new(root: PathBuf, cx: &mut Context<Self>) -> Self {
        let mut panel = FileTreePanel::new(root);
        panel.refresh().ok();

        let items = tree_items_from_panel(&panel);
        let flat_ids = flatten_node_ids(&panel.tree.nodes);
        let tree_state = cx.new(|cx| TreeState::new(cx).items(items));

        // Observe tree state changes to detect selection
        cx.observe(&tree_state, |this: &mut Self, tree_state: Entity<TreeState>, cx: &mut Context<Self>| {
            let selected_id = tree_state.read(cx).selected_entry().map(|e| e.item().id.to_string());
            if selected_id != this.last_selected_id {
                this.last_selected_id = selected_id.clone();
                if let Some(id) = selected_id {
                    let path = PathBuf::from(&id);
                    if path.is_file() {
                        cx.emit(FileSelected(path));
                    }
                }
            }
        })
        .detach();

        let focus_handle = cx.focus_handle();

        Self {
            panel,
            tree_state,
            focus_handle,
            last_selected_id: None,
            flat_ids,
            context_menu: None,
        }
    }

    /// Refresh the file tree from disk.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.panel.refresh().ok();
        let items = tree_items_from_panel(&self.panel);
        self.flat_ids = flatten_node_ids(&self.panel.tree.nodes);
        self.tree_state.update(cx, |state, cx| {
            state.set_items(items, cx);
        });
    }

    /// Programmatically select a file in the tree by path.
    /// Updates `last_selected_id` to prevent the observer from re-emitting FileSelected.
    pub fn select_file(&mut self, path: &Path, cx: &mut Context<Self>) {
        let id = path.to_string_lossy().to_string();
        self.last_selected_id = Some(id.clone());
        if let Some(idx) = self.flat_ids.iter().position(|p| p == &id) {
            self.tree_state.update(cx, |state, cx| {
                state.set_selected_index(Some(idx), cx);
            });
        }
    }
}

/// Convert FileTreePanel's visible items to gpui-component TreeItems.
fn tree_items_from_panel(panel: &FileTreePanel) -> Vec<TreeItem> {
    convert_nodes(&panel.tree.nodes)
}

fn convert_nodes(nodes: &[TreeNode]) -> Vec<TreeItem> {
    nodes
        .iter()
        .map(|node| match node {
            TreeNode::Directory {
                path,
                name,
                children,
                expanded,
            } => {
                TreeItem::new(
                    path.to_string_lossy().to_string(),
                    name.clone(),
                )
                .expanded(*expanded)
                .children(convert_nodes(children))
            }
            TreeNode::File { path, name } => {
                TreeItem::new(
                    path.to_string_lossy().to_string(),
                    name.clone(),
                )
            }
        })
        .collect()
}

/// Flatten tree nodes into a list of IDs in display order (expanded dirs recurse).
fn flatten_node_ids(nodes: &[TreeNode]) -> Vec<String> {
    let mut result = Vec::new();
    for node in nodes {
        match node {
            TreeNode::Directory { path, children, expanded, .. } => {
                result.push(path.to_string_lossy().to_string());
                if *expanded {
                    result.extend(flatten_node_ids(children));
                }
            }
            TreeNode::File { path, .. } => {
                result.push(path.to_string_lossy().to_string());
            }
        }
    }
    result
}

impl Focusable for FileTreeView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FileTreeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ghost = crate::theme::GhostTheme::default_dark();
        let sidebar_bg = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
        let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);
        let fg = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
        let selection_bg = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);

        let mut root = div()
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
                    .text_color(rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2))
                    .child("ghostmd"),
            )
            .child(
                div()
                    .id("file-tree-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .on_click(cx.listener(|this: &mut Self, event: &ClickEvent, _window, cx| {
                        if event.click_count() >= 2 {
                            // Double-click: emit rename for the selected file
                            if let Some(id) = &this.last_selected_id {
                                let path = PathBuf::from(id);
                                if path.is_file() {
                                    cx.emit(FileRenameRequested(path));
                                }
                            }
                        }
                    }))
                    .on_mouse_down(MouseButton::Right, cx.listener(|this: &mut Self, event: &MouseDownEvent, _window, cx| {
                        // Right-click: show context menu for the selected item
                        if let Some(id) = &this.last_selected_id {
                            let path = PathBuf::from(id);
                            this.context_menu = Some((path, event.position));
                            cx.notify();
                        }
                    }))
                    .child(
                        tree(&self.tree_state, |ix, entry, selected, _window, _cx| {
                            ListItem::new(ix)
                                .selected(selected)
                                .child(
                                    div()
                                        .pl(px(16.0 * entry.depth() as f32))
                                        .text_sm()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .child(entry.item().label.clone()),
                                )
                        })
                        .w_full(),
                    ),
            );

        // Context menu overlay
        if let Some((ref path, position)) = self.context_menu {
            let is_file = path.is_file();
            let rename_path = path.clone();
            let finder_path = path.clone();
            let trash_path = path.clone();

            let mut menu = div()
                .absolute()
                .top(position.y)
                .left(position.x)
                .bg(sidebar_bg)
                .border_1()
                .border_color(border_color)
                .rounded(px(4.0))
                .shadow_lg()
                .min_w(px(160.0))
                .flex()
                .flex_col()
                .on_mouse_down_out(cx.listener(|this: &mut Self, _event: &MouseDownEvent, _window, cx| {
                    this.context_menu = None;
                    cx.notify();
                }));

            // Only show Rename for files, not directories
            if is_file {
                menu = menu.child(
                    div()
                        .id("ctx-rename")
                        .px(px(12.0))
                        .py(px(6.0))
                        .text_sm()
                        .text_color(fg)
                        .cursor_pointer()
                        .hover(|s| s.bg(selection_bg))
                        .on_click(cx.listener(move |this: &mut Self, _event, _window, cx| {
                            this.context_menu = None;
                            cx.emit(FileRenameRequested(rename_path.clone()));
                        }))
                        .child("Rename"),
                );
            }

            menu = menu.child(
                div()
                    .id("ctx-open-finder")
                    .px(px(12.0))
                    .py(px(6.0))
                    .text_sm()
                    .text_color(fg)
                    .cursor_pointer()
                    .hover(|s| s.bg(selection_bg))
                    .on_click(cx.listener(move |this: &mut Self, _event, _window, cx| {
                        this.context_menu = None;
                        cx.emit(OpenInFinderRequested(finder_path.clone()));
                    }))
                    .child("Open in Finder"),
            );

            let error_fg = rgb_to_hsla(ghost.error.0, ghost.error.1, ghost.error.2);
            menu = menu.child(
                div()
                    .id("ctx-move-to-trash")
                    .px(px(12.0))
                    .py(px(6.0))
                    .text_sm()
                    .text_color(error_fg)
                    .cursor_pointer()
                    .hover(|s| s.bg(selection_bg))
                    .on_click(cx.listener(move |this: &mut Self, _event, _window, cx| {
                        this.context_menu = None;
                        cx.emit(MoveToTrashRequested(trash_path.clone()));
                    }))
                    .child("Move to Trash"),
            );

            root = root.child(menu);
        }

        root
    }
}
