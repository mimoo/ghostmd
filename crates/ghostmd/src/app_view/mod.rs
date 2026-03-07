mod split_node;
mod session;
mod workspace;
mod file_ops;
mod palette_dispatch;
mod overlays;
mod ai_commands;
mod rendering;
mod fs_watcher;

pub(crate) use split_node::*;
pub(crate) use session::*;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::input::{InputEvent, InputState};
use gpui_component::resizable::{h_resizable, resizable_panel};

use crate::editor_view::EditorView;
use crate::file_tree_view::{FileSelected, FileTreeView, ItemRenamed, ItemMoved, NewItemCreated, OpenInFinderRequested, MoveToTrashRequested, ContextMenuRequested};
use crate::keybindings;
use crate::palette::CommandPalette;
use crate::search::FileFinder;
use crate::theme::{rgb_to_hsla, GhostTheme, ThemeName};

// ---------------------------------------------------------------------------
// Rename mode
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
pub(crate) enum RenameMode {
    Tab,
}

// ---------------------------------------------------------------------------
// Overlay kind — at most one overlay is active at a time
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
pub(crate) enum OverlayKind {
    Palette,
    FileFinder,
    Search,
    AgenticSearch,
    LocationPicker,
}

// ---------------------------------------------------------------------------
// Pane — each pane owns its own editor (independent scroll, cursor, state)
// ---------------------------------------------------------------------------

pub(crate) struct Pane {
    pub(crate) active_path: Option<PathBuf>,
    pub(crate) editor: Option<Entity<EditorView>>,
}

// ---------------------------------------------------------------------------
// Workspace
// ---------------------------------------------------------------------------

pub(crate) struct Workspace {
    pub(crate) id: usize,
    pub(crate) title: String,
    pub(crate) split_root: SplitNode,
    pub(crate) panes: HashMap<usize, Pane>,
    pub(crate) focused_pane: usize,
    /// Stack of previously focused pane IDs (most recent last).
    pub(crate) pane_focus_history: Vec<usize>,
}

// ---------------------------------------------------------------------------
// GhostAppView
// ---------------------------------------------------------------------------

/// Root GPUI view for the GhostMD application.
pub struct GhostAppView {
    pub(crate) root: PathBuf,
    pub(crate) sidebar_visible: bool,
    pub(crate) file_tree: Entity<FileTreeView>,
    pub(crate) workspaces: Vec<Workspace>,
    pub(crate) active_workspace: usize,
    pub(crate) closed_workspaces: Vec<Workspace>,
    pub(crate) next_workspace_id: usize,
    pub(crate) next_pane_id: usize,
    pub(crate) active_overlay: Option<OverlayKind>,
    pub(crate) palette: CommandPalette,
    pub(crate) palette_input: Entity<InputState>,
    pub(crate) rename_mode: Option<RenameMode>,
    pub(crate) file_finder: FileFinder,
    pub(crate) file_finder_input: Entity<InputState>,
    pub(crate) focus_handle: FocusHandle,
    // Search bar
    pub(crate) search_input: Entity<InputState>,
    pub(crate) search_match_count: usize,
    // Theme
    pub(crate) active_theme: ThemeName,
    // Context menu (from file tree right-click)
    pub(crate) tree_context_menu: Option<(PathBuf, Point<Pixels>)>,
    // Agentic search (cmd-shift-f)
    pub(crate) agentic_input: Entity<InputState>,
    pub(crate) agentic_results: Vec<String>,
    pub(crate) agentic_loading: bool,
    // Folder move mode (file finder shows folders instead of files)
    pub(crate) folder_move_source: Option<PathBuf>,
    // Location picker (shown when creating a new note with a folder selected)
    pub(crate) location_picker_options: Vec<(String, PathBuf)>,
    pub(crate) location_picker_selected: usize,
    // Scroll handles for overlays
    pub(crate) palette_scroll: ScrollHandle,
    pub(crate) finder_scroll: ScrollHandle,
    // Update check
    pub(crate) update_available: Option<String>,
    // File watcher for external changes
    pub(crate) _watcher: Option<notify::RecommendedWatcher>,
    pub(crate) fs_events_rx: Option<std::sync::mpsc::Receiver<notify::Event>>,
    pub(crate) last_session_write: Instant,
    // AI loading indicator: set of workspace indices with pending AI operations
    pub(crate) ai_loading: HashSet<usize>,
}

impl GhostAppView {
    pub fn new(root: PathBuf, load_session: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let file_tree = cx.new(|cx| FileTreeView::new(root.clone(), window, cx));

        // Subscribe to file selection events from the tree (with window access)
        cx.subscribe_in(&file_tree, window, |this: &mut Self, _entity, event: &FileSelected, window, cx| {
            this.open_file(event.0.clone(), window, cx);
        })
        .detach();

        // Subscribe to inline rename events from the tree
        cx.subscribe_in(&file_tree, window, |this: &mut Self, _entity, event: &ItemRenamed, _window, cx| {
            this.update_editor_paths(&event.old_path, &event.new_path, cx);
            cx.notify();
        })
        .detach();

        // Subscribe to drag-and-drop move events from the tree
        cx.subscribe_in(&file_tree, window, |this: &mut Self, _entity, event: &ItemMoved, _window, cx| {
            this.update_editor_paths(&event.old_path, &event.new_path, cx);
            this.file_tree.update(cx, |tree, cx| tree.refresh(cx));
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
                        this.active_overlay = None;
                        this.palette.close();
                        let focused = this.active_ws().focused_pane;
                        this.focus_pane_editor(focused, window, cx);
                        cx.notify();
                    } else if this.overlay_is(OverlayKind::Palette) {
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
                    if this.overlay_is(OverlayKind::FileFinder) {
                        let value = this.file_finder_input.read(cx).value().to_string();
                        if this.folder_move_source.is_some() {
                            this.file_finder.set_folder_query(&value);
                        } else {
                            this.file_finder.set_query(&value);
                        }
                        cx.notify();
                    }
                }
                InputEvent::PressEnter { .. } => {
                    if this.overlay_is(OverlayKind::FileFinder) {
                        if let Some(source) = this.folder_move_source.take() {
                            // Folder move mode: move file to selected directory
                            if let Some(target_dir) = this.file_finder.selected_path().map(|p| p.to_path_buf()) {
                                this.active_overlay = None;
                                this.file_finder.close();
                                this.move_file_to_dir(source, &target_dir, cx);
                                let focused = this.active_ws().focused_pane;
                                this.focus_pane_editor(focused, window, cx);
                                cx.notify();
                            }
                        } else if let Some(path) = this.file_finder.selected_path().map(|p| p.to_path_buf()) {
                            this.active_overlay = None;
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
                    if this.overlay_is(OverlayKind::Search) {
                        this.update_search_matches(cx);
                    }
                }
                InputEvent::PressEnter { .. } => {
                    if this.overlay_is(OverlayKind::Search) {
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
                if this.overlay_is(OverlayKind::AgenticSearch) && !this.agentic_loading {
                    this.run_agentic_search(window, cx);
                }
            }
        })
        .detach();

        // --- Load session if available ---
        let session: Option<SessionState> = if load_session {
            let session_path = root.join(".ghostmd").join("session.json");
            std::fs::read_to_string(&session_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        } else {
            None
        };

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
                    pane_focus_history: Vec::new(),
                });
            }
        }

        // Apply saved theme to file tree
        file_tree.update(cx, |tree, _cx| {
            tree.set_theme(active_theme);
        });

        let mut view = Self {
            root: root.clone(),
            sidebar_visible,
            file_tree,
            workspaces,
            active_workspace,
            closed_workspaces: Vec::new(),
            next_workspace_id,
            next_pane_id,
            active_overlay: None,
            palette,
            palette_input,
            rename_mode: None,
            file_finder,
            file_finder_input,
            focus_handle,
            search_input,
            search_match_count: 0,
            active_theme,
            tree_context_menu: None,
            agentic_input,
            agentic_results: Vec::new(),
            agentic_loading: false,
            folder_move_source: None,
            location_picker_options: Vec::new(),
            location_picker_selected: 0,
            palette_scroll: ScrollHandle::new(),
            finder_scroll: ScrollHandle::new(),
            update_available: None,
            _watcher: None,
            fs_events_rx: None,
            last_session_write: Instant::now(),
            ai_loading: HashSet::new(),
        };

        // Set up file watcher for external changes
        {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    tx.send(event).ok();
                }
            })
            .ok();
            if let Some(ref mut w) = watcher {
                use notify::Watcher as _;
                w.watch(root.as_ref(), notify::RecursiveMode::Recursive).ok();
            }
            view._watcher = watcher;
            view.fs_events_rx = Some(rx);
        }

        // If no session was loaded (or it was empty), create a default workspace
        if view.workspaces.is_empty() {
            let root_ref = view.root.clone();
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

        // Check for updates in the background
        cx.spawn(async |this: WeakEntity<GhostAppView>, cx: &mut AsyncApp| {
            let result = cx.background_executor().spawn(async {
                let output = std::process::Command::new("curl")
                    .args(["-fsSL", "--max-time", "5", "https://api.github.com/repos/mimoo/ghostmd/releases/latest"])
                    .output()
                    .ok()?;
                if !output.status.success() { return None; }
                let body = String::from_utf8(output.stdout).ok()?;
                let tag = body.lines()
                    .find(|l| l.contains("\"tag_name\""))?
                    .split('"')
                    .nth(3)?
                    .to_string();
                Some(tag)
            }).await;

            if let Some(latest_tag) = result {
                let current = env!("CARGO_PKG_VERSION");
                let latest_ver = latest_tag.trim_start_matches('v');
                if latest_ver != current {
                    let _ = this.update(cx, |this, cx| {
                        this.update_available = Some(latest_tag);
                        cx.notify();
                    });
                }
            }
        })
        .detach();

        view
    }

    pub(crate) fn active_ws(&self) -> &Workspace {
        &self.workspaces[self.active_workspace]
    }

    pub(crate) fn active_ws_mut(&mut self) -> &mut Workspace {
        &mut self.workspaces[self.active_workspace]
    }

    /// Ensure at least one workspace exists, creating one if needed.
    pub(crate) fn ensure_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspaces.is_empty() {
            let root = self.root.clone();
            self.new_workspace(&root, window, cx);
        }
    }

    /// The path currently active in the focused pane of the active workspace.
    pub(crate) fn focused_active_path(&self) -> Option<PathBuf> {
        if self.workspaces.is_empty() {
            return None;
        }
        let ws = self.active_ws();
        ws.panes.get(&ws.focused_pane)
            .and_then(|p| p.active_path.clone())
    }

    /// Check if a specific overlay is active.
    pub(crate) fn overlay_is(&self, kind: OverlayKind) -> bool {
        self.active_overlay.as_ref() == Some(&kind)
    }

    /// Clear pane editors that reference deleted files.
    pub(crate) fn clear_deleted_panes(&mut self, ws_idx: usize) {
        if ws_idx >= self.workspaces.len() { return; }
        let ws = &mut self.workspaces[ws_idx];
        for pane in ws.panes.values_mut() {
            if let Some(path) = &pane.active_path {
                if !path.exists() {
                    pane.active_path = None;
                    pane.editor = None;
                }
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
        let ghost = GhostTheme::from_name(self.active_theme);
        let bg = rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2);
        let sidebar_visible = self.sidebar_visible;
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
                pane_focus_history: Vec::new(),
            };
            (Some(sr), Some(wsc))
        } else {
            (None, None)
        };
        let show_palette = self.overlay_is(OverlayKind::Palette);
        let show_file_finder = self.overlay_is(OverlayKind::FileFinder);
        let show_agentic_search = self.overlay_is(OverlayKind::AgenticSearch);
        let show_location_picker = self.overlay_is(OverlayKind::LocationPicker);

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
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::MoveToTrash, window, cx| {
                // If sidebar is focused with selection, delete those; otherwise delete focused file
                let tree_paths: Vec<PathBuf> = this.file_tree.read(cx).selected_paths().iter().cloned().collect();
                if !tree_paths.is_empty() && this.sidebar_visible {
                    for path in tree_paths {
                        this.move_to_trash(path, window, cx);
                    }
                } else if let Some(path) = this.focused_active_path() {
                    this.move_to_trash(path, window, cx);
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::RestoreTab, window, cx| {
                if let Some(ws) = this.closed_workspaces.pop() {
                    this.workspaces.push(ws);
                    this.active_workspace = this.workspaces.len() - 1;
                    this.clear_deleted_panes(this.active_workspace);
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
                this.sidebar_visible = !this.sidebar_visible;
                cx.notify();
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::OpenFileFinder, window, cx| {
                let was_open = this.overlay_is(OverlayKind::FileFinder);
                if was_open {
                    this.close_file_finder(window, cx);
                } else {
                    this.active_overlay = Some(OverlayKind::FileFinder);
                    // Collect open files sorted by most recently edited
                    let mut open_with_time: Vec<(PathBuf, Option<Instant>)> = Vec::new();
                    let mut seen = HashSet::new();
                    for ws in &this.workspaces {
                        for pane in ws.panes.values() {
                            if let (Some(path), Some(editor)) = (&pane.active_path, &pane.editor) {
                                if seen.insert(path.clone()) {
                                    let last_edit = editor.read(cx).last_edit;
                                    open_with_time.push((path.clone(), last_edit));
                                }
                            }
                        }
                    }
                    // Sort: most recently edited first, files never edited last
                    open_with_time.sort_by(|a, b| match (&b.1, &a.1) {
                        (Some(tb), Some(ta)) => tb.cmp(ta),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    });
                    let open_files: Vec<PathBuf> = open_with_time.into_iter().map(|(p, _)| p).collect();
                    this.file_finder.set_open_files(open_files);
                    this.file_finder.open().ok();
                    this.finder_scroll = ScrollHandle::new();
                    this.file_finder_input.update(cx, |state, cx| {
                        state.set_value("", window, cx);
                        state.focus(window, cx);
                    });
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::OpenContentSearch, window, cx| {
                if this.overlay_is(OverlayKind::AgenticSearch) {
                    this.close_agentic_search(window, cx);
                } else {
                    this.open_agentic_search(window, cx);
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::OpenCommandPalette, window, cx| {
                if this.overlay_is(OverlayKind::Palette) {
                    this.close_palette(window, cx);
                } else {
                    this.active_overlay = Some(OverlayKind::Palette);
                    this.palette.open();
                    this.palette_scroll = ScrollHandle::new();
                    this.palette_input.update(cx, |state, cx| {
                        state.set_value("", window, cx);
                        state.focus(window, cx);
                    });
                }
                cx.notify();
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::Escape, window, cx| {
                let editing = this.file_tree.read(cx).is_editing();
                if editing {
                    this.file_tree.update(cx, |tree, cx| tree.cancel_rename(window, cx));
                } else if this.active_overlay.is_some() {
                    this.dismiss_overlays(window, cx);
                } else if this.tree_context_menu.is_some() {
                    this.tree_context_menu = None;
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::PaletteUp, window, cx| {
                match &this.active_overlay {
                    Some(OverlayKind::LocationPicker) => {
                        if this.location_picker_selected > 0 {
                            this.location_picker_selected -= 1;
                        }
                        cx.notify();
                    }
                    Some(OverlayKind::FileFinder) => {
                        this.file_finder.select_prev();
                        this.finder_scroll.scroll_to_item(this.file_finder.selected_index);
                        cx.notify();
                    }
                    Some(OverlayKind::AgenticSearch) => cx.notify(),
                    Some(OverlayKind::Palette) => this.palette_move_up(cx),
                    Some(OverlayKind::Search) | None => {
                        window.dispatch_action(Box::new(gpui_component::input::MoveUp), cx);
                    }
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::PaletteDown, window, cx| {
                match &this.active_overlay {
                    Some(OverlayKind::LocationPicker) => {
                        if this.location_picker_selected + 1 < this.location_picker_options.len() {
                            this.location_picker_selected += 1;
                        }
                        cx.notify();
                    }
                    Some(OverlayKind::FileFinder) => {
                        this.file_finder.select_next();
                        this.finder_scroll.scroll_to_item(this.file_finder.selected_index);
                        cx.notify();
                    }
                    Some(OverlayKind::AgenticSearch) => cx.notify(),
                    Some(OverlayKind::Palette) => this.palette_move_down(cx),
                    Some(OverlayKind::Search) | None => {
                        window.dispatch_action(Box::new(gpui_component::input::MoveDown), cx);
                    }
                }
            }))
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::PaletteConfirm, window, cx| {
                match &this.active_overlay {
                    Some(OverlayKind::LocationPicker) => this.confirm_location_picker(window, cx),
                    Some(OverlayKind::Palette) => this.palette_confirm(window, cx),
                    _ => {}
                }
            }))
            // Find in file
            .on_action(cx.listener(|this: &mut Self, _action: &keybindings::FindInFile, window, cx| {
                if this.overlay_is(OverlayKind::Search) {
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
            .child({
                // Titlebar spacer — prevents content from overlapping traffic lights
                let mut bar = div()
                    .w_full()
                    .h(px(38.0))
                    .flex_shrink_0()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_end();
                if let Some(tag) = &self.update_available {
                    let ghost = GhostTheme::from_name(self.active_theme);
                    let hint_fg = rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2);
                    let accent = rgb_to_hsla(ghost.accent.0, ghost.accent.1, ghost.accent.2);
                    bar = bar.child(
                        div()
                            .pr(px(12.0))
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(hint_fg)
                                    .child(format!("update available ({tag})"))
                            )
                            .child(
                                div()
                                    .id("update-btn")
                                    .text_xs()
                                    .text_color(accent)
                                    .cursor_pointer()
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .border_1()
                                    .border_color(accent)
                                    .rounded(px(4.0))
                                    .hover(|s| s.bg(accent).text_color(rgb_to_hsla(ghost.bg.0, ghost.bg.1, ghost.bg.2)))
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.run_update(cx);
                                    }))
                                    .child("Update & restart")
                            )
                    );
                }
                bar
            });

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
                                            .when(show_location_picker, |d| d.child(self.render_location_picker(cx)))
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
            let is_root = *path == self.root;
            let diary_dir = self.root.join("diary");
            let is_diary_path = path.starts_with(&diary_dir);

            // Determine the directory for "New Note" / "New Folder"
            let context_dir = if is_dir {
                path.clone()
            } else {
                path.parent().unwrap_or(&self.root).to_path_buf()
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

            // Rename: enabled for non-diary files and non-root/non-diary folders;
            // shown disabled (grayed out) for diary folders so users understand it's not available.
            let rename_enabled = if is_file {
                !is_diary_path
            } else if is_dir {
                !is_root && *path != diary_dir && !is_diary_path
            } else {
                false
            };
            let show_rename = rename_enabled || (is_dir && !is_root && (is_diary_path || *path == diary_dir));
            let hint_fg_ctx = rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2);
            if show_rename {
                menu = menu.child(
                    div()
                        .id("ctx-rename")
                        .px(px(12.0))
                        .py(px(6.0))
                        .text_sm()
                        .text_color(if rename_enabled { fg } else { hint_fg_ctx })
                        .when(rename_enabled, |d| {
                            d.cursor_pointer()
                                .hover(|s| s.bg(selection_bg))
                                .on_click(cx.listener(move |this: &mut Self, _event, window, cx| {
                                    this.tree_context_menu = None;
                                    this.file_tree.update(cx, |tree, cx| {
                                        tree.start_rename(&rename_path, window, cx);
                                    });
                                }))
                        })
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

            // New Folder (not available inside diary tree)
            let new_folder_in_diary = new_folder_dir.starts_with(&diary_dir);
            if !new_folder_in_diary {
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
            }

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
