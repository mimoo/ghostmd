use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::Root;

use crate::app::GhostApp;
use crate::editor_view::EditorView;
use crate::file_tree_view::{FileSelected, FileTreeView};
use crate::keybindings;
use crate::palette::{CommandPalette, PaletteCommand};
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

    fn leftmost_leaf(&self) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { left, .. } => left.leftmost_leaf(),
        }
    }

    fn rightmost_leaf(&self) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { right, .. } => right.leftmost_leaf(),
        }
    }

    fn topmost_leaf(&self) -> usize {
        self.leftmost_leaf()
    }

    fn bottommost_leaf(&self) -> usize {
        match self {
            SplitNode::Leaf(id) => *id,
            SplitNode::Split { right, .. } => right.bottommost_leaf(),
        }
    }

    /// Find the pane to the right of `from` in a 2D-aware manner.
    fn find_right(&self, from: usize) -> Option<usize> {
        match self {
            SplitNode::Leaf(_) => None,
            SplitNode::Split { direction, left, right } => {
                if *direction == SplitDirection::Vertical {
                    // Side-by-side: if `from` is in left, go to right's leftmost
                    if left.contains(from) {
                        // First try to find a right neighbor within the left subtree
                        if let Some(id) = left.find_right(from) {
                            return Some(id);
                        }
                        return Some(right.leftmost_leaf());
                    }
                    // If in right subtree, recurse into right
                    right.find_right(from)
                } else {
                    // Top/bottom: recurse into whichever subtree contains `from`
                    if left.contains(from) {
                        left.find_right(from)
                    } else {
                        right.find_right(from)
                    }
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
                        return Some(left.rightmost_leaf());
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
                        return Some(right.topmost_leaf());
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
                        return Some(left.bottommost_leaf());
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
}

// ---------------------------------------------------------------------------
// Pane
// ---------------------------------------------------------------------------

struct Pane {
    active_path: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------

struct Workspace {
    id: usize,
    title: String,
    title_generated: bool,
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
    editors: HashMap<PathBuf, Entity<EditorView>>,
    workspaces: Vec<Workspace>,
    active_workspace: usize,
    closed_workspaces: Vec<Workspace>,
    next_workspace_id: usize,
    next_pane_id: usize,
    show_palette: bool,
    palette: CommandPalette,
    palette_input: Entity<InputState>,
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

        let palette = CommandPalette::new(Self::palette_commands());

        let palette_input = cx.new(|cx| InputState::new(window, cx).placeholder("Type a command..."));

        // Subscribe to palette input changes (with window access for PressEnter)
        cx.subscribe_in(&palette_input, window, |this: &mut Self, _entity: &Entity<InputState>, event: &InputEvent, window, cx| {
            match event {
                InputEvent::Change => {
                    let value = this.palette_input.read(cx).value().to_string();
                    this.palette.query = value;
                    this.palette.selected_index = 0;
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => {
                    if this.show_palette {
                        this.palette_confirm(window, cx);
                    }
                }
                _ => {}
            }
        })
        .detach();

        let mut view = Self {
            app,
            file_tree,
            editors: HashMap::new(),
            workspaces: Vec::new(),
            active_workspace: 0,
            closed_workspaces: Vec::new(),
            next_workspace_id: 0,
            next_pane_id: 0,
            show_palette: false,
            palette,
            palette_input,
            focus_handle,
        };

        // Create the first workspace with a diary note
        view.new_workspace(&root, window, cx);

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
            PaletteCommand { label: "Quit".into(), shortcut_hint: Some("Cmd+Q".into()), action_id: "quit".into() },
        ]
    }

    fn active_ws(&self) -> &Workspace {
        &self.workspaces[self.active_workspace]
    }

    fn active_ws_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active_workspace]
    }

    /// Create a new workspace with one pane and a diary note.
    fn new_workspace(&mut self, root: &std::path::Path, window: &mut Window, cx: &mut Context<Self>) {
        let ws_id = self.next_workspace_id;
        self.next_workspace_id += 1;

        let pane_id = self.next_pane_id;
        self.next_pane_id += 1;

        let mut panes = HashMap::new();
        panes.insert(pane_id, Pane { active_path: None });

        let ws = Workspace {
            id: ws_id,
            title: format!("Workspace {}", ws_id + 1),
            title_generated: false,
            split_root: SplitNode::Leaf(pane_id),
            panes,
            focused_pane: pane_id,
        };

        self.workspaces.push(ws);
        self.active_workspace = self.workspaces.len() - 1;

        // Open a new diary note with a random name
        let diary_path = diary::new_diary_path(root, &random_note_name());
        let note = Note::new(diary_path.clone());
        note.ensure_dir().ok();
        note.save("").ok();
        self.file_tree.update(cx, |tree, cx| tree.refresh(cx));
        self.open_file(diary_path, window, cx);
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
        let ws = self.active_ws_mut();
        if let Some(pane) = ws.panes.get_mut(&ws.focused_pane) {
            pane.active_path = Some(path.clone());
        }
        let focused = ws.focused_pane;
        self.focus_pane_editor(focused, window, cx);
        // Sync file tree selection
        self.file_tree.update(cx, |tree, cx| {
            tree.select_file(&path, cx);
        });
        self.request_workspace_title(self.active_workspace, cx);
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
            if let Some(path) = &pane.active_path {
                if let Some(editor) = self.editors.get(path) {
                    editor.update(cx, |e, cx| {
                        e.focus_input(window, cx);
                    });
                }
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
        let current_path = ws.panes.get(&ws.focused_pane)
            .and_then(|p| p.active_path.clone());

        ws.panes.insert(new_id, Pane { active_path: current_path });
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

        // Save the file in the focused pane before closing (extract info first to avoid borrow conflict)
        let save_path = {
            let ws = &self.workspaces[self.active_workspace];
            ws.panes.get(&ws.focused_pane).and_then(|p| p.active_path.clone())
        };
        if let Some(path) = save_path {
            if let Some(editor) = self.editors.get(&path) {
                editor.update(cx, |e, _cx| {
                    e.save(_cx).ok();
                });
            }
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
            self.cleanup_unused_editors(cx);
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
        self.request_workspace_title(self.active_workspace, cx);
        self.cleanup_unused_editors(cx);
        self.sync_file_tree_selection(cx);
        cx.notify();
    }

    /// Remove editors that are not referenced by any pane in any workspace.
    fn cleanup_unused_editors(&mut self, cx: &mut Context<Self>) {
        let mut used_paths = std::collections::HashSet::new();
        for ws in &self.workspaces {
            for pane in ws.panes.values() {
                if let Some(path) = &pane.active_path {
                    used_paths.insert(path.clone());
                }
            }
        }
        self.editors.retain(|path, editor| {
            if used_paths.contains(path) {
                true
            } else {
                // Save before dropping
                editor.update(cx, |e, cx| { e.save(cx).ok(); });
                false
            }
        });
    }

    /// Request an AI-generated workspace title using the `claude` CLI.
    fn request_workspace_title(&mut self, workspace_idx: usize, cx: &mut Context<Self>) {
        if workspace_idx >= self.workspaces.len() {
            return;
        }

        let ws = &self.workspaces[workspace_idx];

        // Collect file titles from all panes
        let titles: Vec<String> = ws.panes.values()
            .filter_map(|p| p.active_path.as_ref())
            .map(|p| Note::title_from_path(p))
            .collect();

        if titles.is_empty() {
            return;
        }

        // If only one file, just use its title directly (no AI needed)
        if titles.len() == 1 {
            if let Some(ws) = self.workspaces.get_mut(workspace_idx) {
                ws.title = titles[0].clone();
                ws.title_generated = false;
            }
            cx.notify();
            return;
        }

        // Skip if already generated for this set
        if ws.title_generated {
            return;
        }

        let ws_id = ws.id;
        let prompt = format!(
            "Generate a short (2-4 word) workspace title that captures the theme of these notes: {}. Reply with ONLY the title, nothing else.",
            titles.join(", ")
        );

        // Use first file's title as immediate fallback
        let fallback_title = titles[0].clone();

        // Run the blocking claude CLI command on a background thread
        let bg_executor = cx.background_executor().clone();
        cx.spawn(async move |this: WeakEntity<GhostAppView>, cx: &mut AsyncApp| {
            let title = bg_executor.spawn(async move {
                let result = std::process::Command::new("claude")
                    .arg("-p")
                    .arg(&prompt)
                    .output();

                match result {
                    Ok(output) if output.status.success() => {
                        let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if raw.is_empty() { fallback_title } else { raw }
                    }
                    _ => fallback_title,
                }
            }).await;

            this.update(cx, |this: &mut GhostAppView, cx: &mut Context<GhostAppView>| {
                if let Some(ws) = this.workspaces.iter_mut().find(|w| w.id == ws_id) {
                    ws.title = title;
                    ws.title_generated = true;
                }
                cx.notify();
            }).ok();
        })
        .detach();
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
                if let Some(path) = self.focused_active_path() {
                    if let Some(editor) = self.editors.get(&path) {
                        editor.update(cx, |e, cx| { e.save(cx).ok(); });
                        cx.notify();
                    }
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

    /// Close the command palette and refocus the editor.
    fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_palette = false;
        self.palette.close();
        let focused = self.active_ws().focused_pane;
        self.focus_pane_editor(focused, window, cx);
        cx.notify();
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
                p.active_path.as_ref()
                    .and_then(|path| self.editors.get(path))
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
        let multi_pane = ws.panes.len() > 1;

        match node {
            SplitNode::Leaf(pane_id) => {
                let is_focused = *pane_id == ws.focused_pane;
                let pid = *pane_id;

                // Pane title bar
                let title_text = ws.panes.get(pane_id)
                    .and_then(|p| p.active_path.as_ref())
                    .map(|p| Note::title_from_path(p))
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

                pane_div = pane_div.child(title_bar);

                if let Some(pane) = ws.panes.get(pane_id) {
                    if let Some(path) = &pane.active_path {
                        if let Some(editor) = self.editors.get(path) {
                            pane_div = pane_div.child(editor.clone());
                        }
                    }
                }

                pane_div.into_any_element()
            }
            SplitNode::Split { direction, left, right } => {
                div()
                    .flex_1()
                    .flex()
                    .when(*direction == SplitDirection::Vertical, |d| d.flex_row())
                    .when(*direction == SplitDirection::Horizontal, |d| d.flex_col())
                    .child(self.render_split_node(left, ws, cx))
                    .child(self.render_split_node(right, ws, cx))
                    .into_any_element()
            }
        }
    }

    fn render_command_palette(&self, cx: &mut Context<Self>) -> Div {
        let ghost = GhostTheme::default_dark();
        let overlay_bg = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
        let fg = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
        let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);
        let selection_bg = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);
        let hint_fg = rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2);

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
                    .child(list),
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
            title_generated: self.active_ws().title_generated,
            split_root: split_root.clone(),
            panes: self.active_ws().panes.iter().map(|(&k, v)| {
                (k, Pane { active_path: v.active_path.clone() })
            }).collect(),
            focused_pane: self.active_ws().focused_pane,
        };
        let show_palette = self.show_palette;

        let root = div()
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
                this.new_workspace_tab(window, cx);
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::NewWindow, window, cx| {
                this.new_window(window, cx);
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
            .on_action(cx.listener(|_this: &mut Self, _action: &keybindings::OpenFileFinder, _window, _cx| {
                // TODO: wire up file finder overlay
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
                if this.show_palette {
                    this.close_palette(window, cx);
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::PaletteUp, _window, cx| {
                if this.show_palette {
                    this.palette_move_up(cx);
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::PaletteDown, _window, cx| {
                if this.show_palette {
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
                    .relative()
                    .child(self.render_tab_bar(cx))
                    .child(self.render_split_node(&split_root, &ws_clone, cx))
                    .when(show_palette, |d| d.child(self.render_command_palette(cx))),
            );

        root
    }
}
