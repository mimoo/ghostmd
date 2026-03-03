use std::path::{Path, PathBuf};

use gpui::*;
use gpui_component::list::ListItem;
use gpui_component::tree::{tree, TreeItem, TreeState};

use crate::file_tree::FileTreePanel;
use crate::theme::{rgb_to_hsla, GhostTheme, ThemeName};
use ghostmd_core::tree::TreeNode;

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

/// GPUI view wrapping the FileTreePanel state machine with a gpui-component Tree.
pub struct FileTreeView {
    panel: FileTreePanel,
    tree_state: Entity<TreeState>,
    focus_handle: FocusHandle,
    /// Last known selected entry ID (to detect changes).
    last_selected_id: Option<String>,
    /// Flat list of tree item IDs in display order (for path→index lookups).
    flat_ids: Vec<String>,
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
            active_theme: ThemeName::default(),
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

    /// Reveal a file in the tree: collapse non-ancestors, expand ancestors, scroll to file.
    pub fn reveal_file(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.panel.tree.reveal_path(path);
        let items = tree_items_from_panel(&self.panel);
        self.flat_ids = flatten_node_ids(&self.panel.tree.nodes);
        self.tree_state.update(cx, |state, cx| {
            state.set_items(items, cx);
        });
        let id = path.to_string_lossy().to_string();
        self.last_selected_id = Some(id.clone());
        if let Some(idx) = self.flat_ids.iter().position(|p| p == &id) {
            self.tree_state.update(cx, |state, cx| {
                state.set_selected_index(Some(idx), cx);
            });
        }
    }

    /// Set the active theme.
    pub fn set_theme(&mut self, name: ThemeName) {
        self.active_theme = name;
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
        let ghost = GhostTheme::from_name(self.active_theme);
        let sidebar_bg = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
        let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);

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
                            if let Some(id) = &this.last_selected_id {
                                let path = PathBuf::from(id);
                                if path.is_file() {
                                    cx.emit(FileRenameRequested(path));
                                }
                            }
                        }
                    }))
                    .on_mouse_down(MouseButton::Right, cx.listener(|this: &mut Self, event: &MouseDownEvent, _window, cx| {
                        if let Some(id) = &this.last_selected_id {
                            let path = PathBuf::from(id);
                            cx.emit(ContextMenuRequested(path, event.position));
                        }
                    }))
                    .child(
                        tree(&self.tree_state, |ix, entry, selected, _window, _cx| {
                            let label = entry.item().label.clone();
                            let is_folder = entry.is_folder();
                            let is_expanded = entry.is_expanded();

                            let prefix = if is_folder {
                                if is_expanded { "\u{25be} " } else { "\u{25b8} " }
                            } else {
                                "  "
                            };

                            ListItem::new(ix)
                                .selected(selected)
                                .child(
                                    div()
                                        .pl(px(16.0 * entry.depth() as f32))
                                        .text_sm()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .child(format!("{}{}", prefix, label)),
                                )
                        })
                        .w_full(),
                    ),
            )
    }
}
