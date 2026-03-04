use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::resizable::{h_resizable, v_resizable, resizable_panel};
use gpui_component::Root;
use serde::{Serialize, Deserialize};

use crate::app::GhostApp;
use crate::editor_view::EditorView;
use crate::file_tree_view::{FileSelected, FileTreeView, ItemRenamed, NewItemCreated, OpenInFinderRequested, MoveToTrashRequested, ContextMenuRequested};
use crate::keybindings;
use crate::palette::{CommandPalette, PaletteCommand};
use crate::search::FileFinder;
use crate::theme::{rgb_to_hsla, GhostTheme, ThemeName};

use ghostmd_core::diary;

fn random_note_name() -> String {
    const ADJECTIVES: &[&str] = &[
        "bright", "calm", "deep", "eager", "faint", "gentle", "hazy", "keen",
        "light", "mellow", "neat", "pale", "quiet", "rare", "soft", "tidy",
        "vast", "warm", "bold", "crisp", "fresh", "grand", "lucid", "swift",
    ];
    const NOUNS: &[&str] = &[
        "bloom", "cloud", "dawn", "ember", "flame", "grove", "haven", "isle",
        "jade", "knoll", "lake", "mist", "north", "opal", "pine", "ridge",
        "spark", "trail", "vale", "wave", "brook", "cliff", "drift", "frost",
    ];
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize;
    let adj = ADJECTIVES[nanos % ADJECTIVES.len()];
    let noun = NOUNS[(nanos / 7) % NOUNS.len()];
    format!("{}-{}", adj, noun)
}

// ---------------------------------------------------------------------------
// Rename mode
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
enum RenameMode {
    Tab,
}

// ---------------------------------------------------------------------------
// Session persistence types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct SessionState {
    workspaces: Vec<SessionWorkspace>,
    active_workspace: usize,
    sidebar_visible: bool,
    #[serde(default)]
    theme: Option<ThemeName>,
}

#[derive(Serialize, Deserialize)]
struct SessionWorkspace {
    title: String,
    split_root: SessionSplitNode,
    focused_pane_idx: usize,
}

#[derive(Serialize, Deserialize)]
enum SessionSplitNode {
    Leaf { path: Option<String> },
    Split {
        direction: String,
        left: Box<SessionSplitNode>,
        right: Box<SessionSplitNode>,
    },
}

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

    /// Whether this subtree contains the given pane_id.
    fn contains(&self, pane_id: usize) -> bool {
        match self {
            SplitNode::Leaf(id) => *id == pane_id,
            SplitNode::Split { left, right, .. } => {
                left.contains(pane_id) || right.contains(pane_id)
            }
        }
    }

    #[allow(dead_code)]
    fn leftmost_leaf(&self) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { left, .. } => left.leftmost_leaf(),
        }
    }

    #[allow(dead_code)]
    fn rightmost_leaf(&self) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { right, .. } => right.rightmost_leaf(),
        }
    }

    #[allow(dead_code)]
    fn topmost_leaf(&self) -> usize {
        self.leftmost_leaf()
    }

    #[allow(dead_code)]
    fn bottommost_leaf(&self) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { right, .. } => right.bottommost_leaf(),
        }
    }

    // -----------------------------------------------------------------------
    // Position-aware navigation helpers
    // -----------------------------------------------------------------------

    /// Check if pane is in the top half of this subtree.
    fn is_in_top_half(&self, pane_id: usize) -> bool {
        match self {
            SplitNode::Leaf(_) => true,
            SplitNode::Split { direction, left, right, .. } => {
                if *direction == SplitDirection::Horizontal {
                    left.contains(pane_id)
                } else if left.contains(pane_id) {
                    left.is_in_top_half(pane_id)
                } else {
                    right.is_in_top_half(pane_id)
                }
            }
        }
    }

    /// Check if pane is in the left half of this subtree.
    fn is_in_left_half(&self, pane_id: usize) -> bool {
        match self {
            SplitNode::Leaf(_) => true,
            SplitNode::Split { direction, left, right, .. } => {
                if *direction == SplitDirection::Vertical {
                    left.contains(pane_id)
                } else if left.contains(pane_id) {
                    left.is_in_left_half(pane_id)
                } else {
                    right.is_in_left_half(pane_id)
                }
            }
        }
    }

    /// Enter this subtree from the right side, preserving vertical position.
    fn enter_from_right(&self, prefer_top: bool) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { direction, left, right, .. } => {
                if *direction == SplitDirection::Vertical {
                    right.enter_from_right(prefer_top)
                } else if prefer_top {
                    left.enter_from_right(true)
                } else {
                    right.enter_from_right(false)
                }
            }
        }
    }

    /// Enter this subtree from the left side, preserving vertical position.
    fn enter_from_left(&self, prefer_top: bool) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { direction, left, right, .. } => {
                if *direction == SplitDirection::Vertical {
                    left.enter_from_left(prefer_top)
                } else if prefer_top {
                    left.enter_from_left(true)
                } else {
                    right.enter_from_left(false)
                }
            }
        }
    }

    /// Enter this subtree from below, preserving horizontal position.
    fn enter_from_below(&self, prefer_left: bool) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { direction, left, right, .. } => {
                if *direction == SplitDirection::Horizontal {
                    right.enter_from_below(prefer_left)
                } else if prefer_left {
                    left.enter_from_below(true)
                } else {
                    right.enter_from_below(false)
                }
            }
        }
    }

    /// Enter this subtree from above, preserving horizontal position.
    fn enter_from_above(&self, prefer_left: bool) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { direction, left, right, .. } => {
                if *direction == SplitDirection::Horizontal {
                    left.enter_from_above(prefer_left)
                } else if prefer_left {
                    left.enter_from_above(true)
                } else {
                    right.enter_from_above(false)
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Directional navigation (position-aware)
    // -----------------------------------------------------------------------

    /// Find the pane to the right of `from` in a 2D-aware manner.
    fn find_right(&self, from: usize) -> Option<usize> {
        match self {
            SplitNode::Leaf(_) => None,
            SplitNode::Split { direction, left, right } => {
                if *direction == SplitDirection::Vertical {
                    if left.contains(from) {
                        if let Some(id) = left.find_right(from) {
                            return Some(id);
                        }
                        let prefer_top = left.is_in_top_half(from);
                        return Some(right.enter_from_left(prefer_top));
                    }
                    right.find_right(from)
                } else if left.contains(from) {
                    left.find_right(from)
                } else {
                    right.find_right(from)
                }
            }
        }
    }

    /// Find the pane to the left of `from` in a 2D-aware manner.
    fn find_left(&self, from: usize) -> Option<usize> {
        match self {
            SplitNode::Leaf(_) => None,
            SplitNode::Split { direction, left, right } => {
                if *direction == SplitDirection::Vertical {
                    if right.contains(from) {
                        if let Some(id) = right.find_left(from) {
                            return Some(id);
                        }
                        let prefer_top = right.is_in_top_half(from);
                        return Some(left.enter_from_right(prefer_top));
                    }
                    left.find_left(from)
                } else if left.contains(from) {
                    left.find_left(from)
                } else {
                    right.find_left(from)
                }
            }
        }
    }

    /// Find the pane below `from` in a 2D-aware manner.
    fn find_down(&self, from: usize) -> Option<usize> {
        match self {
            SplitNode::Leaf(_) => None,
            SplitNode::Split { direction, left, right } => {
                if *direction == SplitDirection::Horizontal {
                    if left.contains(from) {
                        if let Some(id) = left.find_down(from) {
                            return Some(id);
                        }
                        let prefer_left = left.is_in_left_half(from);
                        return Some(right.enter_from_above(prefer_left));
                    }
                    right.find_down(from)
                } else if left.contains(from) {
                    left.find_down(from)
                } else {
                    right.find_down(from)
                }
            }
        }
    }

    /// Find the pane above `from` in a 2D-aware manner.
    fn find_up(&self, from: usize) -> Option<usize> {
        match self {
            SplitNode::Leaf(_) => None,
            SplitNode::Split { direction, left, right } => {
                if *direction == SplitDirection::Horizontal {
                    if right.contains(from) {
                        if let Some(id) = right.find_up(from) {
                            return Some(id);
                        }
                        let prefer_left = right.is_in_left_half(from);
                        return Some(left.enter_from_below(prefer_left));
                    }
                    left.find_up(from)
                } else if left.contains(from) {
                    left.find_up(from)
                } else {
                    right.find_up(from)
                }
            }
        }
    }

    /// A stable ID for this node, derived from the leftmost leaf.
    fn stable_id(&self) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { left, .. } => left.stable_id(),
        }
    }

    /// Remove a leaf by pane_id. Returns true if removed.
    /// When a leaf is removed, its parent split is collapsed to the sibling.
    fn remove_leaf(&mut self, pane_id: usize) -> bool {
        match self {
            SplitNode::Leaf(_) => false,
            SplitNode::Split { left, right, .. } => {
                if let SplitNode::Leaf(id) = **left {
                    if id == pane_id {
                        *self = *right.clone();
                        return true;
                    }
                }
                if let SplitNode::Leaf(id) = **right {
                    if id == pane_id {
                        *self = *left.clone();
                        return true;
                    }
                }
                left.remove_leaf(pane_id) || right.remove_leaf(pane_id)
            }
        }
    }

    /// Convert to serializable session format.
    fn to_session(&self, panes: &HashMap<usize, Pane>) -> SessionSplitNode {
        match self {
            SplitNode::Leaf(id) => {
                let path = panes.get(id)
                    .and_then(|p| p.active_path.as_ref())
                    .map(|p| p.to_string_lossy().to_string());
                SessionSplitNode::Leaf { path }
            }
            SplitNode::Split { direction, left, right } => {
                SessionSplitNode::Split {
                    direction: match direction {
                        SplitDirection::Vertical => "vertical".to_string(),
                        SplitDirection::Horizontal => "horizontal".to_string(),
                    },
                    left: Box::new(left.to_session(panes)),
                    right: Box::new(right.to_session(panes)),
                }
            }
        }
    }
}

/// Reconstruct a SplitNode tree from a serialized session, creating EditorView entities for each pane.
fn restore_split_node(
    session_node: &SessionSplitNode,
    next_pane_id: &mut usize,
    panes: &mut HashMap<usize, Pane>,
    window: &mut Window,
    cx: &mut Context<GhostAppView>,
) -> SplitNode {
    match session_node {
        SessionSplitNode::Leaf { path } => {
            let pane_id = *next_pane_id;
            *next_pane_id += 1;

            let (active_path, editor) = if let Some(p) = path {
                let path_buf = PathBuf::from(p);
                if path_buf.exists() {
                    let pb = path_buf.clone();
                    let e = cx.new(|cx| EditorView::new(pb, window, cx));
                    (Some(path_buf), Some(e))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            panes.insert(pane_id, Pane { active_path, editor });
            SplitNode::Leaf(pane_id)
        }
        SessionSplitNode::Split { direction, left, right } => {
            let dir = if direction == "horizontal" {
                SplitDirection::Horizontal
            } else {
                SplitDirection::Vertical
            };
            SplitNode::Split {
                direction: dir,
                left: Box::new(restore_split_node(left, next_pane_id, panes, window, cx)),
                right: Box::new(restore_split_node(right, next_pane_id, panes, window, cx)),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pane — each pane owns its own editor (independent scroll, cursor, state)
// ---------------------------------------------------------------------------

struct Pane {
    active_path: Option<PathBuf>,
    editor: Option<Entity<EditorView>>,
}

// ---------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------

struct Workspace {
    id: usize,
    title: String,
    split_root: SplitNode,
    panes: HashMap<usize, Pane>,
    focused_pane: usize,
}

// ---------------------------------------------------------------------------
// GhostAppView
// ---------------------------------------------------------------------------

/// Root GPUI view for the GhostMD application.
pub struct GhostAppView {
    app: GhostApp,
    file_tree: Entity<FileTreeView>,
    workspaces: Vec<Workspace>,
    active_workspace: usize,
    closed_workspaces: Vec<Workspace>,
    next_workspace_id: usize,
    next_pane_id: usize,
    show_palette: bool,
    palette: CommandPalette,
    palette_input: Entity<InputState>,
    rename_mode: Option<RenameMode>,
    show_file_finder: bool,
    file_finder: FileFinder,
    file_finder_input: Entity<InputState>,
    focus_handle: FocusHandle,
    // Search bar
    show_search: bool,
    search_input: Entity<InputState>,
    search_match_count: usize,
    // Theme
    active_theme: ThemeName,
    // Context menu (from file tree right-click)
    tree_context_menu: Option<(PathBuf, Point<Pixels>)>,
    // Agentic search (cmd-shift-f)
    show_agentic_search: bool,
    agentic_input: Entity<InputState>,
    agentic_results: Vec<String>,
    agentic_loading: bool,
    // Scroll handles for overlays
    palette_scroll: ScrollHandle,
    finder_scroll: ScrollHandle,
}

impl GhostAppView {
    pub fn new(root: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let app = GhostApp::new(root.clone());

        let file_tree = cx.new(|cx| FileTreeView::new(root.clone(), window, cx));

        // Subscribe to file selection events from the tree (with window access)
        cx.subscribe_in(&file_tree, window, |this: &mut Self, _entity, event: &FileSelected, window, cx| {
            this.open_file(event.0.clone(), window, cx);
        })
        .detach();

        // Subscribe to inline rename events from the tree
        cx.subscribe_in(&file_tree, window, |this: &mut Self, _entity, event: &ItemRenamed, _window, cx| {
            // Update any open editor paths if the file was renamed
            let old = &event.old_path;
            let new = &event.new_path;
            let mut editors_to_update = Vec::new();
            for ws in &mut this.workspaces {
                for pane in ws.panes.values_mut() {
                    if pane.active_path.as_ref() == Some(old) {
                        pane.active_path = Some(new.clone());
                        if let Some(editor) = &pane.editor {
                            editors_to_update.push(editor.clone());
                        }
                    }
                }
            }
            for editor in editors_to_update {
                let np = new.clone();
                editor.update(cx, |e, _cx| {
                    e.path = np;
                });
            }
            cx.notify();
        })
        .detach();

        // Subscribe to new item creation from the tree
        cx.subscribe_in(&file_tree, window, |this: &mut Self, _entity, event: &NewItemCreated, window, cx| {
            if event.0.is_file() {
                this.open_file(event.0.clone(), window, cx);
            }
        })
        .detach();

        // Subscribe to open-in-finder requests from the tree
        cx.subscribe_in(&file_tree, window, |_this: &mut Self, _entity, event: &OpenInFinderRequested, _window, _cx| {
            std::process::Command::new("open").arg("-R").arg(&event.0).spawn().ok();
        })
        .detach();

        // Subscribe to move-to-trash requests from the tree
        cx.subscribe_in(&file_tree, window, |this: &mut Self, _entity, event: &MoveToTrashRequested, window, cx| {
            this.move_to_trash(event.0.clone(), window, cx);
        })
        .detach();

        // Subscribe to context menu requests from the tree
        cx.subscribe_in(&file_tree, window, |this: &mut Self, _entity, event: &ContextMenuRequested, _window, cx| {
            this.tree_context_menu = Some((event.0.clone(), event.1));
            cx.notify();
        })
        .detach();

        let focus_handle = cx.focus_handle();

        let palette = CommandPalette::new(Self::palette_commands());

        let palette_input = cx.new(|cx| InputState::new(window, cx).placeholder("Type a command..."));

        // Subscribe to palette input changes (with window access for PressEnter)
        cx.subscribe_in(&palette_input, window, |this: &mut Self, _entity: &Entity<InputState>, event: &InputEvent, window, cx| {
            match event {
                InputEvent::Change => {
                    if this.rename_mode.is_some() {
                        // In rename mode, don't filter commands
                        cx.notify();
                    } else {
                        let value = this.palette_input.read(cx).value().to_string();
                        this.palette.query = value;
                        this.palette.selected_index = 0;
                        cx.notify();
                    }
                }
                InputEvent::PressEnter { .. } => {
                    if let Some(mode) = this.rename_mode.clone() {
                        let new_name = this.palette_input.read(cx).value().to_string().trim().to_string();
                        if !new_name.is_empty() {
                            this.apply_rename(&new_name, &mode, window, cx);
                        }
                        this.rename_mode = None;
                        this.show_palette = false;
                        this.palette.close();
                        let focused = this.active_ws().focused_pane;
                        this.focus_pane_editor(focused, window, cx);
                        cx.notify();
                    } else if this.show_palette {
                        this.palette_confirm(window, cx);
                    }
                }
                _ => {}
            }
        })
        .detach();

        let file_finder = FileFinder::new(root.clone());
        let file_finder_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search files..."));

        // Subscribe to file finder input changes
        cx.subscribe_in(&file_finder_input, window, |this: &mut Self, _entity: &Entity<InputState>, event: &InputEvent, window, cx| {
            match event {
                InputEvent::Change => {
                    if this.show_file_finder {
                        let value = this.file_finder_input.read(cx).value().to_string();
                        this.file_finder.set_query(&value);
                        cx.notify();
                    }
                }
                InputEvent::PressEnter { .. } => {
                    if this.show_file_finder {
                        if let Some(path) = this.file_finder.selected_path().map(|p| p.to_path_buf()) {
                            this.show_file_finder = false;
                            this.file_finder.close();
                            this.open_file(path, window, cx);
                        }
                    }
                }
                _ => {}
            }
        })
        .detach();

        // Search bar input
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Find in file..."));
        cx.subscribe_in(&search_input, window, |this: &mut Self, _entity: &Entity<InputState>, event: &InputEvent, window, cx| {
            match event {
                InputEvent::Change => {
                    if this.show_search {
                        this.update_search_matches(cx);
                    }
                }
                InputEvent::PressEnter { .. } => {
                    if this.show_search {
                        this.close_search(window, cx);
                    }
                }
                _ => {}
            }
        })
        .detach();

        // Agentic search input (cmd-shift-f)
        let agentic_input = cx.new(|cx| InputState::new(window, cx).placeholder("Ask Claude about your notes..."));
        cx.subscribe_in(&agentic_input, window, |this: &mut Self, _entity: &Entity<InputState>, event: &InputEvent, window, cx| {
            if let InputEvent::PressEnter { .. } = event {
                if this.show_agentic_search && !this.agentic_loading {
                    this.run_agentic_search(window, cx);
                }
            }
        })
        .detach();

        // --- Load session if available ---
        let session_path = root.join(".ghostmd").join("session.json");
        let session: Option<SessionState> = std::fs::read_to_string(&session_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());

        let mut next_pane_id = 0usize;
        let mut next_workspace_id = 0usize;
        let mut workspaces = Vec::new();
        let mut active_workspace = 0usize;
        let mut sidebar_visible = true;
        let mut active_theme = ThemeName::default();

        if let Some(session) = session {
            sidebar_visible = session.sidebar_visible;
            active_workspace = session.active_workspace.min(session.workspaces.len().saturating_sub(1));
            if let Some(theme) = session.theme {
                active_theme = theme;
            }

            for sws in &session.workspaces {
                let ws_id = next_workspace_id;
                next_workspace_id += 1;

                let mut panes = HashMap::new();
                let split_root = restore_split_node(&sws.split_root, &mut next_pane_id, &mut panes, window, cx);

                let leaves = split_root.leaves();
                let focused_pane = if sws.focused_pane_idx < leaves.len() {
                    leaves[sws.focused_pane_idx]
                } else {
                    leaves.first().copied().unwrap_or(0)
                };

                workspaces.push(Workspace {
                    id: ws_id,
                    title: sws.title.clone(),
                    split_root,
                    panes,
                    focused_pane,
                });
            }
        }

        // Apply saved theme to file tree
        file_tree.update(cx, |tree, _cx| {
            tree.set_theme(active_theme);
        });

        let mut view = Self {
            app: {
                let mut a = app;
                a.sidebar_visible = sidebar_visible;
                a
            },
            file_tree,
            workspaces,
            active_workspace,
            closed_workspaces: Vec::new(),
            next_workspace_id,
            next_pane_id,
            show_palette: false,
            palette,
            palette_input,
            rename_mode: None,
            show_file_finder: false,
            file_finder,
            file_finder_input,
            focus_handle,
            show_search: false,
            search_input,
            search_match_count: 0,
            active_theme,
            tree_context_menu: None,
            show_agentic_search: false,
            agentic_input,
            agentic_results: Vec::new(),
            agentic_loading: false,
            palette_scroll: ScrollHandle::new(),
            finder_scroll: ScrollHandle::new(),
        };

        // If no session was loaded (or it was empty), create a default workspace
        if view.workspaces.is_empty() {
            let root_ref = view.app.root.clone();
            view.new_workspace(&root_ref, window, cx);
        }

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

    fn palette_commands() -> Vec<PaletteCommand> {
        vec![
            PaletteCommand { label: "New Note".into(), shortcut_hint: Some("Cmd+N".into()), action_id: "new_note".into() },
            PaletteCommand { label: "New Workspace".into(), shortcut_hint: Some("Cmd+T".into()), action_id: "new_workspace".into() },
            PaletteCommand { label: "New Window".into(), shortcut_hint: Some("Cmd+Shift+N".into()), action_id: "new_window".into() },
            PaletteCommand { label: "Save".into(), shortcut_hint: Some("Cmd+S".into()), action_id: "save".into() },
            PaletteCommand { label: "Close Pane".into(), shortcut_hint: Some("Cmd+W".into()), action_id: "close_pane".into() },
            PaletteCommand { label: "Restore Workspace".into(), shortcut_hint: Some("Cmd+Shift+T".into()), action_id: "restore_workspace".into() },
            PaletteCommand { label: "Split Right".into(), shortcut_hint: Some("Cmd+D".into()), action_id: "split_right".into() },
            PaletteCommand { label: "Split Down".into(), shortcut_hint: Some("Cmd+Shift+D".into()), action_id: "split_down".into() },
            PaletteCommand { label: "Toggle Sidebar".into(), shortcut_hint: Some("Cmd+B".into()), action_id: "toggle_sidebar".into() },
            PaletteCommand { label: "Rename File...".into(), shortcut_hint: None, action_id: "rename_file".into() },
            PaletteCommand { label: "Rename Tab...".into(), shortcut_hint: None, action_id: "rename_tab".into() },
            PaletteCommand { label: "Open in Finder".into(), shortcut_hint: None, action_id: "open_in_finder".into() },
            PaletteCommand { label: "Theme: Rose Pine".into(), shortcut_hint: None, action_id: "theme_rose_pine".into() },
            PaletteCommand { label: "Theme: Nord".into(), shortcut_hint: None, action_id: "theme_nord".into() },
            PaletteCommand { label: "Theme: Solarized".into(), shortcut_hint: None, action_id: "theme_solarized".into() },
            PaletteCommand { label: "Theme: Dracula".into(), shortcut_hint: None, action_id: "theme_dracula".into() },
            PaletteCommand { label: "Theme: Light".into(), shortcut_hint: None, action_id: "theme_light".into() },
            PaletteCommand { label: "Delete Current File".into(), shortcut_hint: None, action_id: "delete_file".into() },
            PaletteCommand { label: "Quit".into(), shortcut_hint: Some("Cmd+Q".into()), action_id: "quit".into() },
        ]
    }

    fn active_ws(&self) -> &Workspace {
        &self.workspaces[self.active_workspace]
    }

    fn active_ws_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active_workspace]
    }

    /// Ensure at least one workspace exists, creating one if needed.
    fn ensure_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspaces.is_empty() {
            let root = self.app.root.clone();
            self.new_workspace(&root, window, cx);
        }
    }

    /// Create a new empty workspace with one pane.
    fn new_workspace(&mut self, _root: &std::path::Path, window: &mut Window, cx: &mut Context<Self>) {
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
        };

        self.workspaces.push(ws);
        self.active_workspace = self.workspaces.len() - 1;
        self.focus_pane_editor(pane_id, window, cx);
        cx.notify();
    }

    /// Open a file: create per-pane editor and set it as active in the focused pane.
    fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.ensure_workspace(window, cx);
        // Save current editor if switching files
        let save_editor = {
            let ws = &self.workspaces[self.active_workspace];
            ws.panes.get(&ws.focused_pane).and_then(|p| {
                if p.active_path.as_ref() != Some(&path) {
                    p.editor.clone()
                } else {
                    None
                }
            })
        };
        if let Some(editor) = save_editor {
            editor.update(cx, |e, cx| { e.save(cx).ok(); });
        }

        // Check if this pane already has this file
        let already_open = {
            let ws = &self.workspaces[self.active_workspace];
            ws.panes.get(&ws.focused_pane)
                .map(|p| p.active_path.as_ref() == Some(&path))
                .unwrap_or(false)
        };

        if !already_open {
            let p = path.clone();
            let editor = cx.new(|cx| EditorView::new(p, window, cx));
            let ws = self.active_ws_mut();
            if let Some(pane) = ws.panes.get_mut(&ws.focused_pane) {
                pane.editor = Some(editor);
                pane.active_path = Some(path.clone());
            }
        }

        let focused = self.active_ws().focused_pane;
        self.focus_pane_editor(focused, window, cx);
        // Reveal file in tree (collapse non-ancestors, expand ancestors, scroll)
        self.file_tree.update(cx, |tree, cx| {
            tree.reveal_file(&path, cx);
        });
        cx.notify();
    }

    /// Create a new note with inline rename in the file tree (cmd-n).
    /// If a folder is selected in the file tree, creates the note there instead of diary path.
    fn new_note_in_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.ensure_workspace(window, cx);
        let root = self.app.root.clone();
        let selected_dir = self.file_tree.read(cx).selected_path()
            .and_then(|p| {
                if p.is_dir() { Some(p.clone()) } else { p.parent().map(|pp| pp.to_path_buf()) }
            })
            .filter(|d| d.starts_with(&root) && *d != root);

        let parent_dir = selected_dir.unwrap_or_else(|| diary::today_diary_dir(&root));
        std::fs::create_dir_all(&parent_dir).ok();

        if !self.app.sidebar_visible {
            self.app.toggle_sidebar();
        }
        self.file_tree.update(cx, |tree, cx| {
            tree.start_new_note(&parent_dir, window, cx);
        });
        cx.notify();
    }

    /// Create a new note in a specific directory with inline rename.
    fn new_note_in_dir(&mut self, dir: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if !self.app.sidebar_visible {
            self.app.toggle_sidebar();
        }
        self.file_tree.update(cx, |tree, cx| {
            tree.start_new_note(&dir, window, cx);
        });
        cx.notify();
    }

    /// Create a new folder inside a parent directory with inline rename.
    fn create_new_folder(&mut self, parent: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if !self.app.sidebar_visible {
            self.app.toggle_sidebar();
        }
        self.file_tree.update(cx, |tree, cx| {
            tree.start_new_folder(&parent, window, cx);
        });
        cx.notify();
    }

    /// Create a new workspace with a diary note (cmd-t).
    fn new_workspace_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.app.root.clone();
        self.new_workspace(&root, window, cx);
    }

    /// Open a new OS window (cmd-shift-n).
    fn new_window(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let root = self.app.root.clone();
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
                        let app_view = cx.new(|cx| GhostAppView::new(root, window, cx));
                        cx.new(|cx| Root::new(app_view, window, cx))
                    },
                ).ok();
            }).ok();
        })
        .detach();
    }

    /// Switch to workspace at index.
    fn switch_workspace(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
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
    fn focus_pane_editor(&self, pane_id: usize, window: &mut Window, cx: &mut Context<Self>) {
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
    fn sync_file_tree_selection(&self, cx: &mut Context<Self>) {
        if self.workspaces.is_empty() {
            return;
        }
        if let Some(path) = self.focused_active_path() {
            self.file_tree.update(cx, |tree, cx| {
                tree.select_file(&path, cx);
            });
        }
    }

    /// Split the focused pane, creating a new pane showing the same file.
    fn split(&mut self, direction: SplitDirection, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspaces.is_empty() {
            return;
        }
        let new_id = self.next_pane_id;
        self.next_pane_id += 1;

        let ws = self.active_ws_mut();
        ws.panes.insert(new_id, Pane { active_path: None, editor: None });
        ws.split_root.split_leaf(ws.focused_pane, new_id, direction);
        ws.focused_pane = new_id;
        self.focus_pane_editor(new_id, window, cx);
        cx.notify();
    }

    /// Navigate focus to an adjacent pane using 2D-aware tree navigation.
    /// Stops at edges (no wrapping).
    fn focus_pane_direction(&mut self, dx: i32, dy: i32, window: &mut Window, cx: &mut Context<Self>) {
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
            ws.focused_pane = new_id;
            self.focus_pane_editor(new_id, window, cx);
            self.sync_file_tree_selection(cx);
            cx.notify();
        }
    }

    /// Close the focused pane. If it's the last pane with a file, clear to empty.
    /// If last pane is already empty, close the workspace.
    fn close_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
                self.active_workspace = 0;
                window.focus(&self.focus_handle);
                cx.notify();
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

        // Switch focus to the first remaining leaf
        let leaves = ws.split_root.leaves();
        if let Some(&first) = leaves.first() {
            ws.focused_pane = first;
        }

        let focused = self.active_ws().focused_pane;
        self.focus_pane_editor(focused, window, cx);
        self.sync_file_tree_selection(cx);
        cx.notify();
    }

    fn auto_save(&mut self, cx: &mut Context<Self>) {
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

    /// The path currently active in the focused pane of the active workspace.
    fn focused_active_path(&self) -> Option<PathBuf> {
        if self.workspaces.is_empty() {
            return None;
        }
        let ws = self.active_ws();
        ws.panes.get(&ws.focused_pane)
            .and_then(|p| p.active_path.clone())
    }

    /// Dispatch a palette command by action_id.
    fn dispatch_palette_action(&mut self, action_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        match action_id {
            "new_note" => self.new_note_in_pane(window, cx),
            "new_workspace" => self.new_workspace_tab(window, cx),
            "new_window" => self.new_window(window, cx),
            "save" => {
                if self.workspaces.is_empty() { return; }
                let editor = {
                    let ws = self.active_ws();
                    ws.panes.get(&ws.focused_pane).and_then(|p| p.editor.clone())
                };
                if let Some(editor) = editor {
                    editor.update(cx, |e, cx| { e.save(cx).ok(); });
                    cx.notify();
                }
            }
            "close_pane" => self.close_pane(window, cx),
            "restore_workspace" => {
                if let Some(ws) = self.closed_workspaces.pop() {
                    self.workspaces.push(ws);
                    self.active_workspace = self.workspaces.len() - 1;
                    let focused = self.workspaces[self.active_workspace].focused_pane;
                    self.focus_pane_editor(focused, window, cx);
                    cx.notify();
                }
            }
            "split_right" => self.split(SplitDirection::Vertical, window, cx),
            "split_down" => self.split(SplitDirection::Horizontal, window, cx),
            "toggle_sidebar" => { self.app.toggle_sidebar(); cx.notify(); }
            "rename_file" => {
                if let Some(path) = self.focused_active_path() {
                    if !self.app.sidebar_visible {
                        self.app.toggle_sidebar();
                    }
                    let p = path.clone();
                    self.file_tree.update(cx, |tree, cx| {
                        tree.reveal_file(&p, cx);
                    });
                    self.file_tree.update(cx, |tree, cx| {
                        tree.start_rename(&p, &mut *window, cx);
                    });
                }
            }
            "rename_tab" => self.enter_rename_mode(RenameMode::Tab, window, cx),
            "open_in_finder" => {
                if let Some(path) = self.focused_active_path() {
                    std::process::Command::new("open").arg("-R").arg(&path).spawn().ok();
                }
            }
            "theme_rose_pine" => self.switch_theme(ThemeName::RosePine, cx),
            "theme_nord" => self.switch_theme(ThemeName::Nord, cx),
            "theme_solarized" => self.switch_theme(ThemeName::Solarized, cx),
            "theme_dracula" => self.switch_theme(ThemeName::Dracula, cx),
            "theme_light" => self.switch_theme(ThemeName::Light, cx),
            "delete_file" => {
                if let Some(path) = self.focused_active_path() {
                    self.move_to_trash(path, window, cx);
                }
            }
            "quit" => cx.quit(),
            _ => {}
        }
    }

    /// Move palette selection up.
    fn palette_move_up(&mut self, cx: &mut Context<Self>) {
        if self.palette.selected_index > 0 {
            self.palette.selected_index -= 1;
            self.palette_scroll.scroll_to_item(self.palette.selected_index);
            cx.notify();
        }
    }

    /// Move palette selection down.
    fn palette_move_down(&mut self, cx: &mut Context<Self>) {
        let count = self.palette.filtered_commands().len();
        if count > 0 && self.palette.selected_index < count - 1 {
            self.palette.selected_index += 1;
            self.palette_scroll.scroll_to_item(self.palette.selected_index);
            cx.notify();
        }
    }

    /// Confirm the selected palette command.
    fn palette_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let filtered = self.palette.filtered_commands();
        if let Some(cmd) = filtered.get(self.palette.selected_index) {
            let action_id = cmd.action_id.clone();
            self.show_palette = false;
            self.palette.close();
            self.dispatch_palette_action(&action_id, window, cx);
            // Don't refocus editor if we entered rename mode (it needs palette focus)
            if self.rename_mode.is_none() && !self.workspaces.is_empty() {
                let focused = self.active_ws().focused_pane;
                self.focus_pane_editor(focused, window, cx);
            }
            cx.notify();
        }
    }

    /// Enter rename mode for tab (via palette).
    fn enter_rename_mode(&mut self, _mode: RenameMode, window: &mut Window, cx: &mut Context<Self>) {
        let current_value = self.active_ws().title.clone();
        self.rename_mode = Some(RenameMode::Tab);
        self.show_palette = true;
        self.palette_input.update(cx, |state, cx| {
            state.set_value(&current_value, window, cx);
            state.focus(window, cx);
        });
        cx.notify();
        cx.defer_in(window, |_this: &mut Self, window, cx| {
            window.dispatch_action(Box::new(gpui_component::input::SelectAll), cx);
        });
    }

    /// Apply the rename (tab only — file rename is handled inline in the tree).
    fn apply_rename(&mut self, new_name: &str, _mode: &RenameMode, _window: &mut Window, _cx: &mut Context<Self>) {
        self.active_ws_mut().title = new_name.to_string();
    }

    /// Close the file finder and refocus the editor.
    fn close_file_finder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_file_finder = false;
        self.file_finder.close();
        if !self.workspaces.is_empty() {
            let focused = self.active_ws().focused_pane;
            self.focus_pane_editor(focused, window, cx);
        }
        cx.notify();
    }

    /// Close the command palette and refocus the editor.
    fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_palette = false;
        self.rename_mode = None;
        self.palette.close();
        if !self.workspaces.is_empty() {
            let focused = self.active_ws().focused_pane;
            self.focus_pane_editor(focused, window, cx);
        }
        cx.notify();
    }

    /// Open the search bar.
    fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_search = true;
        self.search_match_count = 0;
        self.search_input.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }

    /// Close the search bar and refocus the editor.
    fn close_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_search = false;
        self.search_match_count = 0;
        if !self.workspaces.is_empty() {
            let focused = self.active_ws().focused_pane;
            self.focus_pane_editor(focused, window, cx);
        }
        cx.notify();
    }

    fn open_agentic_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_agentic_search = true;
        self.agentic_results.clear();
        self.agentic_loading = false;
        self.agentic_input.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }

    fn close_agentic_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_agentic_search = false;
        self.agentic_loading = false;
        if !self.workspaces.is_empty() {
            let focused = self.active_ws().focused_pane;
            self.focus_pane_editor(focused, window, cx);
        }
        cx.notify();
    }

    /// Dismiss any open overlays (palette, finder, agentic search).
    fn dismiss_overlays(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.show_file_finder {
            self.close_file_finder(window, cx);
        }
        if self.show_agentic_search {
            self.close_agentic_search(window, cx);
        }
        if self.show_palette {
            self.close_palette(window, cx);
        }
    }

    fn run_agentic_search(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let query = self.agentic_input.read(cx).value().to_string().trim().to_string();
        if query.is_empty() {
            return;
        }
        self.agentic_loading = true;
        self.agentic_results.clear();
        cx.notify();

        let root = self.app.root.clone();
        cx.spawn(async move |this: WeakEntity<GhostAppView>, cx: &mut AsyncApp| {
            let output = cx.background_executor().spawn(async move {
                let prompt = format!(
                    "Search through the markdown notes in {} and answer: {}. \
                     Be concise. List relevant file paths and quotes.",
                    root.display(), query
                );
                std::process::Command::new("claude")
                    .arg("-p")
                    .arg(&prompt)
                    .current_dir(&root)
                    .output()
            }).await;

            match output {
                Ok(out) => {
                    let text = String::from_utf8_lossy(&out.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let lines: Vec<String> = if text.trim().is_empty() {
                        if stderr.trim().is_empty() {
                            vec!["No results found.".to_string()]
                        } else {
                            vec![format!("Error: {}", stderr.trim())]
                        }
                    } else {
                        text.lines().map(|l| l.to_string()).collect()
                    };
                    this.update(cx, |this, cx| {
                        this.agentic_results = lines;
                        this.agentic_loading = false;
                        cx.notify();
                    }).ok();
                }
                Err(e) => {
                    this.update(cx, |this, cx| {
                        this.agentic_results = vec![format!("Failed to run claude: {}", e)];
                        this.agentic_loading = false;
                        cx.notify();
                    }).ok();
                }
            }
        })
        .detach();
    }

    /// Update match count based on current search query and focused editor.
    fn update_search_matches(&mut self, cx: &mut Context<Self>) {
        let query = self.search_input.read(cx).value().to_string().to_lowercase();
        if query.is_empty() {
            self.search_match_count = 0;
            cx.notify();
            return;
        }
        let editor = {
            let ws = self.active_ws();
            ws.panes.get(&ws.focused_pane).and_then(|p| p.editor.clone())
        };
        if let Some(editor) = editor {
            let text = editor.read(cx).text(cx).to_lowercase();
            self.search_match_count = text.matches(&query).count();
        } else {
            self.search_match_count = 0;
        }
        cx.notify();
    }

    /// Switch to a named theme.
    fn switch_theme(&mut self, name: ThemeName, cx: &mut Context<Self>) {
        self.active_theme = name;
        self.file_tree.update(cx, |tree, _cx| {
            tree.set_theme(name);
        });
        crate::theme::apply_theme(name, cx);
        cx.notify();
    }

    /// Close workspace at given index.
    fn close_workspace(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
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
            let root = self.app.root.clone();
            self.new_workspace(&root, window, cx);
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

    /// Move a file or folder to the macOS Trash and update the UI.
    fn move_to_trash(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        // Close any panes showing this file (or files inside this directory)
        let is_dir = path.is_dir();
        let mut editors_to_save: Vec<Entity<EditorView>> = Vec::new();
        for ws in &mut self.workspaces {
            for pane in ws.panes.values_mut() {
                let should_close = pane.active_path.as_ref().map(|p| {
                    if is_dir { p.starts_with(&path) } else { p == &path }
                }).unwrap_or(false);
                if should_close {
                    if let Some(editor) = pane.editor.take() {
                        editors_to_save.push(editor);
                    }
                    pane.active_path = None;
                }
            }
        }
        // Save editors before trashing (best effort)
        for editor in editors_to_save {
            editor.update(cx, |e, cx| { e.save(cx).ok(); });
        }

        // Move to Trash using the trash crate (macOS native)
        if trash::delete(&path).is_ok() {
            self.file_tree.update(cx, |tree, cx| tree.refresh(cx));
            // Re-focus current pane
            let focused = self.active_ws().focused_pane;
            self.focus_pane_editor(focused, window, cx);
            cx.notify();
        }
    }

    /// Save session state to disk.
    fn save_session(&self) {
        let session = SessionState {
            workspaces: self.workspaces.iter().map(|ws| {
                let leaves = ws.split_root.leaves();
                let focused_idx = leaves.iter().position(|&id| id == ws.focused_pane).unwrap_or(0);
                SessionWorkspace {
                    title: ws.title.clone(),
                    split_root: ws.split_root.to_session(&ws.panes),
                    focused_pane_idx: focused_idx,
                }
            }).collect(),
            active_workspace: self.active_workspace,
            sidebar_visible: self.app.sidebar_visible,
            theme: Some(self.active_theme),
        };

        let dir = self.app.root.join(".ghostmd");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("session.json");
        if let Ok(json) = serde_json::to_string_pretty(&session) {
            std::fs::write(path, json).ok();
        }
    }

    fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let ghost = GhostTheme::from_name(self.active_theme);
        let tab_bar_bg = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
        let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);
        let accent = rgb_to_hsla(ghost.accent.0, ghost.accent.1, ghost.accent.2);
        let hint_fg = rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2);

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

        for (i, ws) in self.workspaces.iter().enumerate() {
            let is_active = i == self.active_workspace;

            let dirty = ws.panes.values().any(|p| {
                p.editor.as_ref()
                    .map(|e| e.read(cx).dirty)
                    .unwrap_or(false)
            });

            let display = if dirty {
                format!("{} *", ws.title)
            } else {
                ws.title.clone()
            };

            let tab_bg = if is_active {
                rgb_to_hsla(ghost.tab_active.0, ghost.tab_active.1, ghost.tab_active.2)
            } else {
                rgb_to_hsla(ghost.tab_inactive.0, ghost.tab_inactive.1, ghost.tab_inactive.2)
            };
            let fg = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);

            let ws_idx = i;
            let close_idx = i;
            let mut tab_div = div()
                .id(ElementId::NamedInteger("ws-tab".into(), i as u64))
                .group(SharedString::from(format!("tab-{}", i)))
                .px(px(12.0))
                .py(px(6.0))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.0))
                .text_sm()
                .bg(tab_bg)
                .text_color(fg)
                .cursor_pointer()
                .on_click(cx.listener(move |this: &mut Self, _event, window, cx| {
                    this.switch_workspace(ws_idx, window, cx);
                }))
                .child(display)
                .child(
                    div()
                        .id(ElementId::NamedInteger("ws-close".into(), i as u64))
                        .text_xs()
                        .text_color(hint_fg)
                        .opacity(0.0)
                        .group_hover(SharedString::from(format!("tab-{}", i)), |s| s.opacity(1.0))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this: &mut Self, _event: &ClickEvent, window, cx| {
                            this.close_workspace(close_idx, window, cx);
                        }))
                        .child("\u{00d7}"),
                );

            if is_active {
                tab_div = tab_div.border_b_2().border_color(accent);
            }

            tabs = tabs.child(tab_div);
        }

        // "+" button for new workspace
        tabs = tabs.child(
            div()
                .id("new-workspace-btn")
                .px(px(8.0))
                .py(px(6.0))
                .text_sm()
                .text_color(hint_fg)
                .cursor_pointer()
                .on_click(cx.listener(|this: &mut Self, _event, window, cx| {
                    this.new_workspace_tab(window, cx);
                }))
                .child("+"),
        );

        tabs
    }

    fn render_split_node(&self, node: &SplitNode, ws: &Workspace, cx: &mut Context<Self>) -> AnyElement {
        let ghost = GhostTheme::from_name(self.active_theme);
        let bg = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
        let fg = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
        let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);
        let accent = rgb_to_hsla(ghost.accent.0, ghost.accent.1, ghost.accent.2);
        let pane_title_bg = rgb_to_hsla(ghost.pane_title_bg.0, ghost.pane_title_bg.1, ghost.pane_title_bg.2);
        let pane_title_fg = rgb_to_hsla(ghost.pane_title_fg.0, ghost.pane_title_fg.1, ghost.pane_title_fg.2);
        let sidebar_bg = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
        let hint_fg = rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2);
        let multi_pane = ws.panes.len() > 1;

        match node {
            SplitNode::Leaf(pane_id) => {
                let is_focused = *pane_id == ws.focused_pane;
                let pid = *pane_id;
                let pane = ws.panes.get(pane_id);
                let has_editor = pane.map(|p| p.editor.is_some()).unwrap_or(false);

                let mut pane_div = div()
                    .id(ElementId::NamedInteger("pane".into(), pid as u64))
                    .flex_1()
                    .min_w(px(100.0))
                    .min_h(px(100.0))
                    .flex()
                    .flex_col()
                    .bg(bg)
                    .text_color(fg)
                    .on_click(cx.listener(move |this: &mut Self, _event, window, cx| {
                        let ws = this.active_ws_mut();
                        if ws.focused_pane != pid {
                            ws.focused_pane = pid;
                            this.focus_pane_editor(pid, window, cx);
                            this.sync_file_tree_selection(cx);
                            cx.notify();
                        }
                    }));

                if multi_pane {
                    if is_focused {
                        pane_div = pane_div.border_2().border_color(accent);
                    } else {
                        pane_div = pane_div.border_2().border_color(hsla(0., 0., 0., 0.)).opacity(0.5);
                    }
                }

                if has_editor {
                    // Title bar + editor
                    let title_text = pane
                        .and_then(|p| p.active_path.as_ref())
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "untitled".to_string());

                    let title_bar = div()
                        .w_full()
                        .h(px(24.0))
                        .flex()
                        .items_center()
                        .px(px(8.0))
                        .bg(pane_title_bg)
                        .text_color(pane_title_fg)
                        .text_xs()
                        .child(title_text);

                    pane_div = pane_div.child(title_bar);

                    // Search bar (only on focused pane)
                    if is_focused && self.show_search {
                        let match_text = if self.search_match_count > 0 {
                            format!("{} matches", self.search_match_count)
                        } else {
                            let query = self.search_input.read(cx).value().to_string();
                            if query.is_empty() { String::new() } else { "No matches".to_string() }
                        };
                        let search_bar = div()
                            .w_full()
                            .h(px(32.0))
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(8.0))
                            .px(px(8.0))
                            .bg(pane_title_bg)
                            .border_b_1()
                            .border_color(border_color)
                            .child(
                                Input::new(&self.search_input)
                                    .appearance(false)
                                    .w(px(200.0)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hint_fg)
                                    .child(match_text),
                            );
                        pane_div = pane_div.child(search_bar);
                    }

                    if let Some(p) = pane {
                        if let Some(editor) = &p.editor {
                            pane_div = pane_div.child(editor.clone());
                        }
                    }
                } else {
                    // Empty pane placeholder
                    pane_div = pane_div.child(
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_col()
                            .gap(px(8.0))
                            .bg(sidebar_bg)
                            .child(div().text_lg().text_color(hint_fg).child("No file open"))
                            .child(div().text_sm().text_color(hint_fg).child("Cmd+N  Create a new note"))
                            .child(div().text_sm().text_color(hint_fg).child("Cmd+P  Search files")),
                    );
                }

                pane_div.into_any_element()
            }
            SplitNode::Split { direction, left, right } => {
                let left_el = self.render_split_node(left, ws, cx);
                let right_el = self.render_split_node(right, ws, cx);
                let sid = node.stable_id();
                let group = if *direction == SplitDirection::Vertical {
                    h_resizable(ElementId::NamedInteger("split-h".into(), sid as u64))
                        .child(resizable_panel().child(left_el))
                        .child(resizable_panel().child(right_el))
                } else {
                    v_resizable(ElementId::NamedInteger("split-v".into(), sid as u64))
                        .child(resizable_panel().child(left_el))
                        .child(resizable_panel().child(right_el))
                };
                group.into_any_element()
            }
        }
    }

    fn render_file_finder(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let ghost = GhostTheme::from_name(self.active_theme);
        let overlay_bg = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
        let fg = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
        let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);
        let selection_bg = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);
        let hint_fg = rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2);

        let root_prefix = self.app.root.to_string_lossy().to_string();

        let mut list = div()
            .id("finder-results")
            .flex()
            .flex_col()
            .max_h(px(400.0))
            .overflow_y_scroll()
            .track_scroll(&self.finder_scroll);

        let max_display = 50.min(self.file_finder.results.len());
        for i in 0..max_display {
            let result = &self.file_finder.results[i];
            let is_selected = i == self.file_finder.selected_index;
            let bg = if is_selected { selection_bg } else { overlay_bg };

            // Strip root prefix for display
            let full_path = result.path().to_string_lossy().to_string();
            let display_path = full_path
                .strip_prefix(&root_prefix)
                .unwrap_or(&full_path)
                .trim_start_matches('/')
                .to_string();

            let display = match result {
                crate::search::FinderResult::File(_) => display_path,
                crate::search::FinderResult::Content(m) => {
                    let line_preview = m.line_text.trim();
                    let truncated = if line_preview.len() > 60 {
                        format!("{}…", &line_preview[..60])
                    } else {
                        line_preview.to_string()
                    };
                    format!("{}:{} — {}", display_path, m.line_number, truncated)
                }
            };

            list = list.child(
                div()
                    .id(ElementId::NamedInteger("finder-item".into(), i as u64))
                    .w_full()
                    .px(px(12.0))
                    .py(px(4.0))
                    .bg(bg)
                    .text_color(fg)
                    .text_sm()
                    .child(display),
            );
        }

        let count_text = format!("{} files", self.file_finder.result_count());

        div()
            .id("finder-dismiss-bg")
            .absolute()
            .inset_0()
            .on_click(cx.listener(|this: &mut Self, _, window, cx| {
                this.close_file_finder(window, cx);
            }))
            .child(
                div()
                    .absolute()
                    .top(px(60.0))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .id("finder-card")
                            .on_click(cx.listener(|_this: &mut Self, _, _window, cx| {
                                cx.stop_propagation();
                            }))
                            .w(px(500.0))
                            .bg(overlay_bg)
                            .border_1()
                            .border_color(border_color)
                            .rounded(px(8.0))
                            .shadow_lg()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(6.0))
                                    .border_b_1()
                                    .border_color(border_color)
                                    .child(
                                        Input::new(&self.file_finder_input)
                                            .appearance(false)
                                            .w_full(),
                                    ),
                            )
                            .child(list)
                            .child(
                                div()
                                    .px(px(12.0))
                                    .py(px(4.0))
                                    .text_xs()
                                    .text_color(hint_fg)
                                    .child(count_text),
                            ),
                    ),
            )
    }

    fn render_agentic_search(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let ghost = GhostTheme::from_name(self.active_theme);
        let overlay_bg = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
        let fg = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
        let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);
        let hint_fg = rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2);
        let accent = rgb_to_hsla(ghost.accent.0, ghost.accent.1, ghost.accent.2);

        let mut results_div = div()
            .id("agentic-results")
            .flex()
            .flex_col()
            .max_h(px(400.0))
            .overflow_y_scroll();

        if self.agentic_loading {
            results_div = results_div.child(
                div()
                    .px(px(12.0))
                    .py(px(8.0))
                    .text_sm()
                    .text_color(accent)
                    .child("Searching with Claude..."),
            );
        } else {
            for (i, line) in self.agentic_results.iter().enumerate() {
                results_div = results_div.child(
                    div()
                        .id(ElementId::NamedInteger("agentic-line".into(), i as u64))
                        .w_full()
                        .px(px(12.0))
                        .py(px(2.0))
                        .text_color(fg)
                        .text_sm()
                        .child(line.clone()),
                );
            }
        }

        let status = if self.agentic_loading {
            "Running...".to_string()
        } else if self.agentic_results.is_empty() {
            "Press Enter to search".to_string()
        } else {
            format!("{} lines", self.agentic_results.len())
        };

        div()
            .id("agentic-dismiss-bg")
            .absolute()
            .inset_0()
            .on_click(cx.listener(|this: &mut Self, _, window, cx| {
                this.close_agentic_search(window, cx);
            }))
            .child(
                div()
                    .absolute()
                    .top(px(60.0))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .id("agentic-card")
                            .on_click(cx.listener(|_this: &mut Self, _, _window, cx| {
                                cx.stop_propagation();
                            }))
                            .w(px(600.0))
                            .bg(overlay_bg)
                            .border_1()
                            .border_color(border_color)
                            .rounded(px(8.0))
                            .shadow_lg()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(6.0))
                                    .border_b_1()
                                    .border_color(border_color)
                                    .child(
                                        Input::new(&self.agentic_input)
                                            .appearance(false)
                                            .w_full(),
                                    ),
                            )
                            .child(results_div)
                            .child(
                                div()
                                    .px(px(12.0))
                                    .py(px(4.0))
                                    .text_xs()
                                    .text_color(hint_fg)
                                    .child(status),
                            ),
                    ),
            )
    }

    fn render_command_palette(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let ghost = GhostTheme::from_name(self.active_theme);
        let overlay_bg = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
        let fg = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
        let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);
        let selection_bg = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);
        let hint_fg = rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2);

        let is_rename = self.rename_mode.is_some();
        let rename_label = match &self.rename_mode {
            Some(RenameMode::Tab) => "Rename tab:",
            None => "",
        };

        let mut body = div().flex().flex_col();

        if is_rename {
            // Rename mode: show label + input only
            body = body.child(
                div()
                    .px(px(12.0))
                    .py(px(6.0))
                    .text_sm()
                    .text_color(hint_fg)
                    .child(rename_label),
            );
        } else {
            // Normal palette: show filtered command list
            let filtered = self.palette.filtered_commands();

            let mut list = div()
                .id("palette-list")
                .flex()
                .flex_col()
                .max_h(px(300.0))
                .overflow_y_scroll()
                .track_scroll(&self.palette_scroll);

            for (i, cmd) in filtered.iter().enumerate() {
                let is_selected = i == self.palette.selected_index;
                let bg = if is_selected { selection_bg } else { overlay_bg };
                let action_id = cmd.action_id.clone();

                let mut row = div()
                    .id(ElementId::NamedInteger("palette-item".into(), i as u64))
                    .w_full()
                    .px(px(12.0))
                    .py(px(6.0))
                    .flex()
                    .flex_row()
                    .justify_between()
                    .bg(bg)
                    .text_color(fg)
                    .text_sm()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this: &mut Self, _event, window, cx| {
                        this.show_palette = false;
                        this.palette.close();
                        this.dispatch_palette_action(&action_id, window, cx);
                        let focused = this.active_ws().focused_pane;
                        this.focus_pane_editor(focused, window, cx);
                        cx.notify();
                    }))
                    .child(cmd.label.clone());

                if let Some(hint) = &cmd.shortcut_hint {
                    row = row.child(
                        div()
                            .text_color(hint_fg)
                            .text_xs()
                            .child(hint.clone()),
                    );
                }

                list = list.child(row);
            }
            body = body.child(list);
        }

        // Overlay container — full-screen backdrop with nested card
        div()
            .id("palette-dismiss-bg")
            .absolute()
            .inset_0()
            .on_click(cx.listener(|this: &mut Self, _, window, cx| {
                this.close_palette(window, cx);
            }))
            .child(
                div()
                    .absolute()
                    .top(px(60.0))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .id("palette-card")
                            .on_click(cx.listener(|_this: &mut Self, _, _window, cx| {
                                cx.stop_propagation();
                            }))
                            .w(px(400.0))
                            .bg(overlay_bg)
                            .border_1()
                            .border_color(border_color)
                            .rounded(px(8.0))
                            .shadow_lg()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(6.0))
                                    .border_b_1()
                                    .border_color(border_color)
                                    .child(
                                        Input::new(&self.palette_input)
                                            .appearance(false)
                                            .w_full(),
                                    ),
                            )
                            .child(body),
                    ),
            )
    }
}

impl Focusable for GhostAppView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GhostAppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ghost = GhostTheme::from_name(self.active_theme);
        let bg = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
        let sidebar_visible = self.app.sidebar_visible;
        let has_workspaces = !self.workspaces.is_empty();

        let (split_root, ws_clone) = if has_workspaces {
            let sr = self.active_ws().split_root.clone();
            let wsc = Workspace {
                id: self.active_ws().id,
                title: self.active_ws().title.clone(),
                split_root: sr.clone(),
                panes: self.active_ws().panes.iter().map(|(&k, v)| {
                    (k, Pane { active_path: v.active_path.clone(), editor: v.editor.clone() })
                }).collect(),
                focused_pane: self.active_ws().focused_pane,
            };
            (Some(sr), Some(wsc))
        } else {
            (None, None)
        };
        let show_palette = self.show_palette;
        let show_file_finder = self.show_file_finder;
        let show_agentic_search = self.show_agentic_search;

        // Context menu overlay data
        let ctx_menu = self.tree_context_menu.clone();

        let mut root = div()
            .id("ghost-app")
            .size_full()
            .flex()
            .flex_col()
            .bg(bg)
            .track_focus(&self.focus_handle)
            // Action handlers
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::NewNote, window, cx| {
                this.new_note_in_pane(window, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::NewTab, window, cx| {
                this.new_workspace_tab(window, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::NewWindow, window, cx| {
                this.new_window(window, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::Save, _window, cx| {
                if this.workspaces.is_empty() { return; }
                let editor = {
                    let ws = this.active_ws();
                    ws.panes.get(&ws.focused_pane).and_then(|p| p.editor.clone())
                };
                if let Some(editor) = editor {
                    editor.update(cx, |e, cx| {
                        e.save(cx).ok();
                    });
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::Quit, _window, cx| {
                let editors: Vec<Entity<EditorView>> = this.workspaces.iter()
                    .flat_map(|ws| ws.panes.values())
                    .filter_map(|p| p.editor.clone())
                    .collect();
                for editor in editors {
                    editor.update(cx, |e, cx| { e.save(cx).ok(); });
                }
                this.save_session();
                cx.quit();
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::CloseTab, window, cx| {
                this.close_pane(window, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::RestoreTab, window, cx| {
                if let Some(ws) = this.closed_workspaces.pop() {
                    this.workspaces.push(ws);
                    this.active_workspace = this.workspaces.len() - 1;
                    let focused = this.workspaces[this.active_workspace].focused_pane;
                    this.focus_pane_editor(focused, window, cx);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::NextTab, window, cx| {
                if this.workspaces.len() > 1 {
                    let next = (this.active_workspace + 1) % this.workspaces.len();
                    this.switch_workspace(next, window, cx);
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::PrevTab, window, cx| {
                if this.workspaces.len() > 1 {
                    let prev = if this.active_workspace == 0 {
                        this.workspaces.len() - 1
                    } else {
                        this.active_workspace - 1
                    };
                    this.switch_workspace(prev, window, cx);
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::ToggleSidebar, _window, cx| {
                this.app.toggle_sidebar();
                cx.notify();
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::OpenFileFinder, window, cx| {
                this.show_file_finder = !this.show_file_finder;
                if this.show_file_finder {
                    this.file_finder.open().ok();
                    this.finder_scroll = ScrollHandle::new();
                    this.file_finder_input.update(cx, |state, cx| {
                        state.set_value("", window, cx);
                        state.focus(window, cx);
                    });
                } else {
                    this.close_file_finder(window, cx);
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::OpenContentSearch, window, cx| {
                if this.show_agentic_search {
                    this.close_agentic_search(window, cx);
                } else {
                    this.open_agentic_search(window, cx);
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::OpenCommandPalette, window, cx| {
                this.show_palette = !this.show_palette;
                if this.show_palette {
                    this.palette.open();
                    this.palette_scroll = ScrollHandle::new();
                    this.palette_input.update(cx, |state, cx| {
                        state.set_value("", window, cx);
                        state.focus(window, cx);
                    });
                } else {
                    this.close_palette(window, cx);
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::Escape, window, cx| {
                let editing = this.file_tree.read(cx).is_editing();
                if editing {
                    this.file_tree.update(cx, |tree, cx| tree.cancel_rename(window, cx));
                } else if this.show_search {
                    this.close_search(window, cx);
                } else if this.show_agentic_search {
                    this.close_agentic_search(window, cx);
                } else if this.show_file_finder {
                    this.close_file_finder(window, cx);
                } else if this.show_palette {
                    this.close_palette(window, cx);
                } else if this.tree_context_menu.is_some() {
                    this.tree_context_menu = None;
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::PaletteUp, window, cx| {
                if this.show_file_finder {
                    this.file_finder.select_prev();
                    this.finder_scroll.scroll_to_item(this.file_finder.selected_index);
                    cx.notify();
                } else if this.show_agentic_search {
                    // No item selection for agentic search
                    cx.notify();
                } else if this.show_palette {
                    this.palette_move_up(cx);
                } else {
                    window.dispatch_action(Box::new(gpui_component::input::MoveUp), cx);
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::PaletteDown, window, cx| {
                if this.show_file_finder {
                    this.file_finder.select_next();
                    this.finder_scroll.scroll_to_item(this.file_finder.selected_index);
                    cx.notify();
                } else if this.show_agentic_search {
                    cx.notify();
                } else if this.show_palette {
                    this.palette_move_down(cx);
                } else {
                    window.dispatch_action(Box::new(gpui_component::input::MoveDown), cx);
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::PaletteConfirm, window, cx| {
                if this.show_palette {
                    this.palette_confirm(window, cx);
                }
            }))
            // Find in file
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::FindInFile, window, cx| {
                if this.show_search {
                    this.close_search(window, cx);
                } else {
                    this.open_search(window, cx);
                }
            }))
            // Quick tab switching (cmd-1 through cmd-9)
            .on_action(cx.listener(|this, _: &keybindings::SwitchTab1, window, cx| { this.switch_workspace(0, window, cx); }))
            .on_action(cx.listener(|this, _: &keybindings::SwitchTab2, window, cx| { this.switch_workspace(1, window, cx); }))
            .on_action(cx.listener(|this, _: &keybindings::SwitchTab3, window, cx| { this.switch_workspace(2, window, cx); }))
            .on_action(cx.listener(|this, _: &keybindings::SwitchTab4, window, cx| { this.switch_workspace(3, window, cx); }))
            .on_action(cx.listener(|this, _: &keybindings::SwitchTab5, window, cx| { this.switch_workspace(4, window, cx); }))
            .on_action(cx.listener(|this, _: &keybindings::SwitchTab6, window, cx| { this.switch_workspace(5, window, cx); }))
            .on_action(cx.listener(|this, _: &keybindings::SwitchTab7, window, cx| { this.switch_workspace(6, window, cx); }))
            .on_action(cx.listener(|this, _: &keybindings::SwitchTab8, window, cx| { this.switch_workspace(7, window, cx); }))
            .on_action(cx.listener(|this, _: &keybindings::SwitchTab9, window, cx| { this.switch_workspace(8, window, cx); }))
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
            // Dismiss context menu on click (using on_click so menu item handlers fire first)
            .on_click(cx.listener(|this: &mut Self, _, _window, cx| {
                if this.tree_context_menu.is_some() {
                    this.tree_context_menu = None;
                    cx.notify();
                }
            }))
            // Layout: flex_col with titlebar spacer then main content
            .child(
                // Titlebar spacer — prevents content from overlapping traffic lights
                div().w_full().h(px(38.0)).flex_shrink_0()
            );

        if let (Some(split_root), Some(ws_clone)) = (split_root, ws_clone) {
            // Normal layout with workspaces
            root = root.child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_row()
                    .child(
                        h_resizable("main-layout")
                            .child(
                                resizable_panel()
                                    .size(px(240.0))
                                    .size_range(px(150.)..px(500.))
                                    .visible(sidebar_visible)
                                    .child(self.file_tree.clone()),
                            )
                            .child(
                                resizable_panel()
                                    .child(
                                        div()
                                            .size_full()
                                            .flex()
                                            .flex_col()
                                            .relative()
                                            .child(self.render_tab_bar(cx))
                                            .child(self.render_split_node(&split_root, &ws_clone, cx))
                                            .when(show_file_finder, |d| d.child(self.render_file_finder(cx)))
                                            .when(show_agentic_search, |d| d.child(self.render_agentic_search(cx)))
                                            .when(show_palette, |d| d.child(self.render_command_palette(cx))),
                                    ),
                            ),
                    ),
            );
        } else {
            // Welcome screen — no workspaces
            let sidebar_bg = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
            let hint_fg = rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2);
            let fg = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
            root = root.child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_row()
                    .child(
                        h_resizable("main-layout")
                            .child(
                                resizable_panel()
                                    .size(px(240.0))
                                    .size_range(px(150.)..px(500.))
                                    .visible(sidebar_visible)
                                    .child(self.file_tree.clone()),
                            )
                            .child(
                                resizable_panel()
                                    .child(
                                        div()
                                            .size_full()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .flex_col()
                                            .gap(px(12.0))
                                            .bg(sidebar_bg)
                                            .child(div().text_lg().text_color(fg).child("ghostmd"))
                                            .child(div().text_sm().text_color(hint_fg).child("Cmd+N  Create a new note"))
                                            .child(div().text_sm().text_color(hint_fg).child("Cmd+P  Search files"))
                                            .child(div().text_sm().text_color(hint_fg).child("Cmd+T  New workspace"))
                                            .child(div().text_sm().text_color(hint_fg).child("Cmd+Shift+T  Restore last workspace"))
                                            .when(show_file_finder, |d| d.child(self.render_file_finder(cx)))
                                            .when(show_palette, |d| d.child(self.render_command_palette(cx))),
                                    ),
                            ),
                    ),
            );
        }

        // Context menu overlay (rendered at root level for correct z-order and positioning)
        if let Some((ref path, position)) = ctx_menu {
            let is_file = path.is_file();
            let is_dir = path.is_dir();
            let is_root = *path == self.app.root;
            let diary_dir = self.app.root.join("diary");
            let is_diary_path = path.starts_with(&diary_dir);

            // Determine the directory for "New Note" / "New Folder"
            let context_dir = if is_dir {
                path.clone()
            } else {
                path.parent().unwrap_or(&self.app.root).to_path_buf()
            };

            let rename_path = path.clone();
            let new_note_dir = context_dir.clone();
            let new_folder_dir = context_dir;
            let finder_path = path.clone();
            let trash_path = path.clone();

            let sidebar_bg = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
            let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);
            let fg = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
            let selection_bg = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);
            let error_fg = rgb_to_hsla(ghost.error.0, ghost.error.1, ghost.error.2);

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
                .flex_col();

            // Rename (files not in diary, and non-root/non-diary folders)
            let show_rename = if is_file {
                !is_diary_path
            } else if is_dir {
                !is_root && *path != diary_dir && !is_diary_path
            } else {
                false
            };
            if show_rename {
                menu = menu.child(
                    div()
                        .id("ctx-rename")
                        .px(px(12.0))
                        .py(px(6.0))
                        .text_sm()
                        .text_color(fg)
                        .cursor_pointer()
                        .hover(|s| s.bg(selection_bg))
                        .on_click(cx.listener(move |this: &mut Self, _event, window, cx| {
                            this.tree_context_menu = None;
                            this.file_tree.update(cx, |tree, cx| {
                                tree.start_rename(&rename_path, window, cx);
                            });
                        }))
                        .child("Rename"),
                );
            }

            // New Note
            menu = menu.child(
                div()
                    .id("ctx-new-note")
                    .px(px(12.0))
                    .py(px(6.0))
                    .text_sm()
                    .text_color(fg)
                    .cursor_pointer()
                    .hover(|s| s.bg(selection_bg))
                    .on_click(cx.listener(move |this: &mut Self, _event, window, cx| {
                        this.tree_context_menu = None;
                        this.new_note_in_dir(new_note_dir.clone(), window, cx);
                    }))
                    .child("New Note"),
            );

            // New Folder
            menu = menu.child(
                div()
                    .id("ctx-new-folder")
                    .px(px(12.0))
                    .py(px(6.0))
                    .text_sm()
                    .text_color(fg)
                    .cursor_pointer()
                    .hover(|s| s.bg(selection_bg))
                    .on_click(cx.listener(move |this: &mut Self, _event, window, cx| {
                        this.tree_context_menu = None;
                        this.create_new_folder(new_folder_dir.clone(), window, cx);
                    }))
                    .child("New Folder"),
            );

            // Open in Finder
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
                        this.tree_context_menu = None;
                        std::process::Command::new("open").arg("-R").arg(&finder_path).spawn().ok();
                        cx.notify();
                    }))
                    .child("Open in Finder"),
            );

            // Move to Trash (not for root)
            if !is_root {
                menu = menu.child(
                    div()
                        .id("ctx-move-to-trash")
                        .px(px(12.0))
                        .py(px(6.0))
                        .text_sm()
                        .text_color(error_fg)
                        .cursor_pointer()
                        .hover(|s| s.bg(selection_bg))
                        .on_click(cx.listener(move |this: &mut Self, _event, window, cx| {
                            this.tree_context_menu = None;
                            this.move_to_trash(trash_path.clone(), window, cx);
                        }))
                        .child("Move to Trash"),
                );
            }

            root = root.child(menu);
        }

        root
    }
}
