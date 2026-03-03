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
use crate::file_tree_view::{FileSelected, FileTreeView, FileRenameRequested, OpenInFinderRequested};
use crate::keybindings;
use crate::palette::{CommandPalette, PaletteCommand};
use crate::search::FileFinder;
use crate::theme::{rgb_to_hsla, GhostTheme};

use ghostmd_core::diary;
use ghostmd_core::note::Note;

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
    File,
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

        // Subscribe to rename requests from the tree
        cx.subscribe_in(&file_tree, window, |this: &mut Self, _entity, event: &FileRenameRequested, window, cx| {
            // Open the file first (so it's the focused file), then enter rename mode
            this.open_file(event.0.clone(), window, cx);
            this.enter_rename_mode(RenameMode::File, window, cx);
        })
        .detach();

        // Subscribe to open-in-finder requests from the tree
        cx.subscribe_in(&file_tree, window, |_this: &mut Self, _entity, event: &OpenInFinderRequested, _window, _cx| {
            if let Some(parent) = event.0.parent() {
                std::process::Command::new("open").arg(parent).spawn().ok();
            }
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

        if let Some(session) = session {
            sidebar_visible = session.sidebar_visible;
            active_workspace = session.active_workspace.min(session.workspaces.len().saturating_sub(1));

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
            PaletteCommand { label: "Quit".into(), shortcut_hint: Some("Cmd+Q".into()), action_id: "quit".into() },
        ]
    }

    fn active_ws(&self) -> &Workspace {
        &self.workspaces[self.active_workspace]
    }

    fn active_ws_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active_workspace]
    }

    /// Create a new empty workspace with one pane.
    fn new_workspace(&mut self, _root: &std::path::Path, _window: &mut Window, cx: &mut Context<Self>) {
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
        cx.notify();
    }

    /// Open a file: create per-pane editor and set it as active in the focused pane.
    fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
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
        // Sync file tree selection
        self.file_tree.update(cx, |tree, cx| {
            tree.select_file(&path, cx);
        });
        cx.notify();
    }

    /// Create a new diary note and open it in the focused pane of the active workspace (cmd-n).
    fn new_note_in_pane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.app.root.clone();
        let path = diary::new_diary_path(&root, &random_note_name());
        let note = Note::new(path.clone());
        note.ensure_dir().ok();
        note.save("").ok();
        self.file_tree.update(cx, |tree, cx| tree.refresh(cx));
        self.open_file(path, window, cx);
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
    fn focus_pane_editor(&self, pane_id: usize, window: &mut Window, cx: &mut Context<Self>) {
        let ws = self.active_ws();
        if let Some(pane) = ws.panes.get(&pane_id) {
            if let Some(editor) = &pane.editor {
                editor.update(cx, |e, cx| {
                    e.focus_input(window, cx);
                });
            }
        }
    }

    /// Sync the file tree selection to the currently focused pane's file.
    fn sync_file_tree_selection(&self, cx: &mut Context<Self>) {
        if let Some(path) = self.focused_active_path() {
            self.file_tree.update(cx, |tree, cx| {
                tree.select_file(&path, cx);
            });
        }
    }

    /// Split the focused pane, creating a new pane showing the same file.
    fn split(&mut self, direction: SplitDirection, window: &mut Window, cx: &mut Context<Self>) {
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

    /// Close the focused pane. If it's the last pane, close the workspace.
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

        let pane_count = self.workspaces[self.active_workspace].panes.len();
        if pane_count <= 1 {
            // Last pane — close the whole workspace
            let removed = self.workspaces.remove(self.active_workspace);
            self.closed_workspaces.push(removed);

            if self.workspaces.is_empty() {
                // Create a fresh workspace
                let root = self.app.root.clone();
                self.new_workspace(&root, window, cx);
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
            "rename_file" => self.enter_rename_mode(RenameMode::File, window, cx),
            "rename_tab" => self.enter_rename_mode(RenameMode::Tab, window, cx),
            "open_in_finder" => {
                if let Some(path) = self.focused_active_path() {
                    if let Some(parent) = path.parent() {
                        std::process::Command::new("open").arg(parent).spawn().ok();
                    }
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
            cx.notify();
        }
    }

    /// Move palette selection down.
    fn palette_move_down(&mut self, cx: &mut Context<Self>) {
        let count = self.palette.filtered_commands().len();
        if count > 0 && self.palette.selected_index < count - 1 {
            self.palette.selected_index += 1;
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
            let focused = self.active_ws().focused_pane;
            self.focus_pane_editor(focused, window, cx);
            cx.notify();
        }
    }

    /// Enter rename mode for file or tab.
    fn enter_rename_mode(&mut self, mode: RenameMode, window: &mut Window, cx: &mut Context<Self>) {
        let current_value = match mode {
            RenameMode::File => {
                self.focused_active_path()
                    .map(|p| Note::title_from_path(&p))
                    .unwrap_or_default()
            }
            RenameMode::Tab => {
                self.active_ws().title.clone()
            }
        };
        self.rename_mode = Some(mode);
        self.show_palette = true;
        self.palette_input.update(cx, |state, cx| {
            state.set_value(&current_value, window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }

    /// Apply the rename.
    fn apply_rename(&mut self, new_name: &str, mode: &RenameMode, _window: &mut Window, cx: &mut Context<Self>) {
        match mode {
            RenameMode::Tab => {
                self.active_ws_mut().title = new_name.to_string();
            }
            RenameMode::File => {
                if let Some(old_path) = self.focused_active_path() {
                    // Build new path: same directory, new filename with .md extension
                    let slug = ghostmd_core::diary::slugify(new_name);
                    let new_filename = if slug.is_empty() { "untitled".to_string() } else { slug };
                    let new_path = old_path.with_file_name(format!("{}.md", new_filename));
                    if new_path != old_path {
                        // Rename on disk
                        if std::fs::rename(&old_path, &new_path).is_ok() {
                            // Collect editors that need path updates
                            let mut editors_to_update = Vec::new();
                            for ws in &mut self.workspaces {
                                for pane in ws.panes.values_mut() {
                                    if pane.active_path.as_ref() == Some(&old_path) {
                                        pane.active_path = Some(new_path.clone());
                                        if let Some(editor) = &pane.editor {
                                            editors_to_update.push(editor.clone());
                                        }
                                    }
                                }
                            }
                            for editor in editors_to_update {
                                let np = new_path.clone();
                                editor.update(cx, |e, _cx| {
                                    e.path = np;
                                });
                            }
                            self.file_tree.update(cx, |tree, cx| tree.refresh(cx));
                            self.file_tree.update(cx, |tree, cx| tree.select_file(&new_path, cx));
                        }
                    }
                }
            }
        }
    }

    /// Close the file finder and refocus the editor.
    fn close_file_finder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_file_finder = false;
        self.file_finder.close();
        let focused = self.active_ws().focused_pane;
        self.focus_pane_editor(focused, window, cx);
        cx.notify();
    }

    /// Close the command palette and refocus the editor.
    fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_palette = false;
        self.rename_mode = None;
        self.palette.close();
        let focused = self.active_ws().focused_pane;
        self.focus_pane_editor(focused, window, cx);
        cx.notify();
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
        };

        let dir = self.app.root.join(".ghostmd");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("session.json");
        if let Ok(json) = serde_json::to_string_pretty(&session) {
            std::fs::write(path, json).ok();
        }
    }

    fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let ghost = GhostTheme::default_dark();
        let tab_bar_bg = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
        let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);
        let accent = rgb_to_hsla(ghost.accent.0, ghost.accent.1, ghost.accent.2);

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

            // Check if any pane in this workspace has a dirty editor
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
            let mut tab_div = div()
                .id(ElementId::NamedInteger("ws-tab".into(), i as u64))
                .px(px(12.0))
                .py(px(6.0))
                .text_sm()
                .bg(tab_bg)
                .text_color(fg)
                .cursor_pointer()
                .on_click(cx.listener(move |this: &mut Self, _event, window, cx| {
                    this.switch_workspace(ws_idx, window, cx);
                }))
                .child(display);

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
                .text_color(rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2))
                .cursor_pointer()
                .on_click(cx.listener(|this: &mut Self, _event, window, cx| {
                    this.new_workspace_tab(window, cx);
                }))
                .child("+"),
        );

        tabs
    }

    fn render_split_node(&self, node: &SplitNode, ws: &Workspace, cx: &mut Context<Self>) -> AnyElement {
        let ghost = GhostTheme::default_dark();
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
                        pane_div = pane_div.border_1().border_color(border_color).opacity(0.5);
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
                            .child(div().text_sm().text_color(hint_fg).child("Cmd+P to search files"))
                            .child(div().text_sm().text_color(hint_fg).child("Cmd+N to create a new note")),
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

    fn render_file_finder(&self, _cx: &mut Context<Self>) -> Div {
        let ghost = GhostTheme::default_dark();
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
            .overflow_y_scroll();

        let max_display = 50.min(self.file_finder.results.len());
        for i in 0..max_display {
            let result = &self.file_finder.results[i];
            let is_selected = i == self.file_finder.selected_index;
            let bg = if is_selected { selection_bg } else { overlay_bg };

            // Strip root prefix for display
            let display_path = result.path.to_string_lossy().to_string();
            let display_path = display_path
                .strip_prefix(&root_prefix)
                .unwrap_or(&display_path)
                .trim_start_matches('/')
                .to_string();

            list = list.child(
                div()
                    .id(ElementId::NamedInteger("finder-item".into(), i as u64))
                    .w_full()
                    .px(px(12.0))
                    .py(px(4.0))
                    .bg(bg)
                    .text_color(fg)
                    .text_sm()
                    .child(display_path),
            );
        }

        let count_text = format!("{} files", self.file_finder.result_count());

        div()
            .absolute()
            .top(px(60.0))
            .left_0()
            .right_0()
            .flex()
            .justify_center()
            .child(
                div()
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
            )
    }

    fn render_command_palette(&self, cx: &mut Context<Self>) -> Div {
        let ghost = GhostTheme::default_dark();
        let overlay_bg = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
        let fg = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
        let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);
        let selection_bg = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);
        let hint_fg = rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2);

        let is_rename = self.rename_mode.is_some();
        let rename_label = match &self.rename_mode {
            Some(RenameMode::File) => "Rename file:",
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
                .flex()
                .flex_col()
                .max_h(px(300.0))
                .overflow_y_hidden();

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

        // Overlay container — absolutely positioned centered
        div()
            .absolute()
            .top(px(60.0))
            .left_0()
            .right_0()
            .flex()
            .justify_center()
            .child(
                div()
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
        let ghost = GhostTheme::default_dark();
        let bg = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
        let sidebar_visible = self.app.sidebar_visible;
        let split_root = self.active_ws().split_root.clone();
        let ws_clone = Workspace {
            id: self.active_ws().id,
            title: self.active_ws().title.clone(),
            split_root: split_root.clone(),
            panes: self.active_ws().panes.iter().map(|(&k, v)| {
                (k, Pane { active_path: v.active_path.clone(), editor: v.editor.clone() })
            }).collect(),
            focused_pane: self.active_ws().focused_pane,
        };
        let show_palette = self.show_palette;
        let show_file_finder = self.show_file_finder;

        let root = div()
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
                // Save all open editors before quitting
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
                    this.file_finder_input.update(cx, |state, cx| {
                        state.set_value("", window, cx);
                        state.focus(window, cx);
                    });
                } else {
                    this.close_file_finder(window, cx);
                }
                cx.notify();
            }))
            .on_action(cx.listener(|_this: &mut Self, _action: &keybindings::OpenContentSearch, _window, _cx| {
                // TODO: wire up content search overlay
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::OpenCommandPalette, window, cx| {
                this.show_palette = !this.show_palette;
                if this.show_palette {
                    this.palette.open();
                    // Reset and focus the palette input
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
                if this.show_file_finder {
                    this.close_file_finder(window, cx);
                } else if this.show_palette {
                    this.close_palette(window, cx);
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::PaletteUp, _window, cx| {
                if this.show_file_finder {
                    this.file_finder.select_prev();
                    cx.notify();
                } else if this.show_palette {
                    this.palette_move_up(cx);
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::PaletteDown, _window, cx| {
                if this.show_file_finder {
                    this.file_finder.select_next();
                    cx.notify();
                } else if this.show_palette {
                    this.palette_move_down(cx);
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::PaletteConfirm, window, cx| {
                if this.show_palette {
                    this.palette_confirm(window, cx);
                }
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
            // Layout: flex_col with titlebar spacer then main content
            .child(
                // Titlebar spacer — prevents content from overlapping traffic lights
                div().w_full().h(px(38.0)).flex_shrink_0()
            )
            .child(
                // Main content area fills remaining vertical space
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
                                            .when(show_palette, |d| d.child(self.render_command_palette(cx))),
                                    ),
                            ),
                    ),
            );

        root
    }
}
