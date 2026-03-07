use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use gpui::*;
use serde::{Serialize, Deserialize};

use crate::editor_view::EditorView;
use crate::theme::ThemeName;

use super::*;

// ---------------------------------------------------------------------------
// Session persistence types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
pub(crate) struct SessionState {
    pub(crate) workspaces: Vec<SessionWorkspace>,
    pub(crate) active_workspace: usize,
    pub(crate) sidebar_visible: bool,
    #[serde(default)]
    pub(crate) theme: Option<ThemeName>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SessionWorkspace {
    pub(crate) title: String,
    pub(crate) split_root: SessionSplitNode,
    pub(crate) focused_pane_idx: usize,
}

#[derive(Serialize, Deserialize)]
pub(crate) enum SessionSplitNode {
    Leaf { path: Option<String> },
    Split {
        direction: String,
        left: Box<SessionSplitNode>,
        right: Box<SessionSplitNode>,
    },
}

impl SessionSplitNode {
    /// Return the first leaf path (for workspace identity matching).
    pub(crate) fn first_path(&self) -> Option<String> {
        match self {
            SessionSplitNode::Leaf { path } => path.clone(),
            SessionSplitNode::Split { left, .. } => left.first_path(),
        }
    }
}

/// Reconstruct a SplitNode tree from a serialized session, creating EditorView entities for each pane.
pub(crate) fn restore_split_node(
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

impl GhostAppView {
    /// Save session state to disk.
    pub(crate) fn save_session(&mut self) {
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
            sidebar_visible: self.sidebar_visible,
            theme: Some(self.active_theme),
        };

        let dir = self.root.join(".ghostmd");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("session.json");
        if let Ok(json) = serde_json::to_string_pretty(&session) {
            std::fs::write(path, json).ok();
            self.last_session_write = Instant::now();
        }
    }

    pub(crate) fn reload_session_titles(&mut self) {
        // Skip if we wrote session.json recently (avoid overwriting our own changes)
        if self.last_session_write.elapsed().as_millis() < 2000 {
            return;
        }
        let path = self.root.join(".ghostmd").join("session.json");
        let session: Option<SessionState> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());
        if let Some(session) = session {
            // Only reload if workspace count matches (same window wrote this session)
            if session.workspaces.len() != self.workspaces.len() {
                return;
            }
            // Match by first pane path to avoid overwriting unrelated workspaces
            for (i, sws) in session.workspaces.iter().enumerate() {
                let session_path = sws.split_root.first_path();
                let local_path = self.workspaces[i].panes.values()
                    .find_map(|p| p.active_path.as_ref().map(|ap| ap.to_string_lossy().to_string()));
                if session_path == local_path {
                    self.workspaces[i].title = sws.title.clone();
                }
            }
        }
    }
}
