use gpui::*;

use super::*;

impl GhostAppView {
    /// Close the file finder and refocus the editor.
    pub(crate) fn close_file_finder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.active_overlay = None;
        self.file_finder.close();
        self.folder_move_source = None;
        self.clear_finder_match_highlights(cx);
        if !self.workspaces.is_empty() {
            let focused = self.active_ws().focused_pane;
            self.focus_pane_editor(focused, window, cx);
        }
        cx.notify();
    }

    /// Push the current set of finder result paths into the file tree so it can
    /// render an accent indicator on matching nodes (and ancestor dirs). Clears
    /// highlights when the finder query is empty so the tree doesn't light up
    /// every file on open.
    pub(crate) fn sync_finder_match_highlights(&mut self, cx: &mut Context<Self>) {
        let query = self.file_finder.query.clone();
        let paths: Vec<std::path::PathBuf> = if query.is_empty() {
            Vec::new()
        } else {
            self.file_finder
                .results
                .iter()
                .map(|r| r.path().to_path_buf())
                .collect()
        };
        self.file_tree
            .update(cx, |tree, cx| tree.set_match_paths(paths, cx));
    }

    /// Clear all fuzzy-finder match highlights in the file tree.
    pub(crate) fn clear_finder_match_highlights(&mut self, cx: &mut Context<Self>) {
        self.file_tree.update(cx, |tree, cx| {
            tree.set_match_paths(std::iter::empty(), cx)
        });
    }

    /// Confirm the currently-selected finder result. Folders reveal+focus the
    /// tree; files open in the active pane. Returns true if a result was
    /// confirmed (so callers can skip their default action).
    pub(crate) fn confirm_finder_selection(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(path) = self.file_finder.selected_path().map(|p| p.to_path_buf()) else {
            return false;
        };
        self.active_overlay = None;
        self.file_finder.close();
        self.clear_finder_match_highlights(cx);
        if path.is_dir() {
            self.file_tree.update(cx, |tree, cx| tree.reveal_dir(&path, cx));
            self.focus_file_tree(window, cx);
        } else {
            self.open_file(path, window, cx);
        }
        true
    }

    /// Move keyboard focus into the sidebar file tree. Auto-shows the sidebar
    /// if hidden and ensures a row is selected so arrow keys have somewhere to
    /// start from.
    pub(crate) fn focus_file_tree(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.sidebar_visible {
            self.sidebar_visible = true;
        }
        self.dismiss_overlays(window, cx);
        self.file_tree.update(cx, |tree, cx| {
            tree.ensure_keyboard_cursor(cx);
        });
        let handle = self.file_tree.read(cx).focus_handle(cx);
        handle.focus(window);
        cx.notify();
    }

    /// Close the command palette and refocus the editor.
    pub(crate) fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.active_overlay = None;
        self.rename_mode = None;
        self.palette.close();
        if !self.workspaces.is_empty() {
            let focused = self.active_ws().focused_pane;
            self.focus_pane_editor(focused, window, cx);
        }
        cx.notify();
    }

    pub(crate) fn open_agentic_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.active_overlay = Some(OverlayKind::AgenticSearch);
        self.agentic_results.clear();
        self.agentic_loading = false;
        self.agentic_selected = 0;
        self.agentic_scroll = ScrollHandle::new();
        self.agentic_input.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        cx.notify();
        let input = self.agentic_input.clone();
        cx.defer_in(window, move |_this, window, cx| {
            input.update(cx, |state, cx| state.focus(window, cx));
        });
    }

    pub(crate) fn close_agentic_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.active_overlay = None;
        self.agentic_loading = false;
        if !self.workspaces.is_empty() {
            let focused = self.active_ws().focused_pane;
            self.focus_pane_editor(focused, window, cx);
        }
        cx.notify();
    }

    pub(crate) fn open_note_switcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.active_overlay = Some(OverlayKind::NoteSwitcher);
        self.note_switcher_selected = 0;
        self.note_switcher_scroll = ScrollHandle::new();
        // Collect all open notes across all workspaces
        self.note_switcher_all = self.collect_open_notes(cx);
        self.note_switcher_results = self.note_switcher_all.clone();
        self.note_switcher_input.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        cx.notify();
        let input = self.note_switcher_input.clone();
        cx.defer_in(window, move |_this, window, cx| {
            input.update(cx, |state, cx| state.focus(window, cx));
        });
    }

    pub(crate) fn close_note_switcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.active_overlay = None;
        self.note_switcher_results.clear();
        self.note_switcher_all.clear();
        if !self.workspaces.is_empty() {
            let focused = self.active_ws().focused_pane;
            self.focus_pane_editor(focused, window, cx);
        }
        cx.notify();
    }

    pub(crate) fn confirm_note_switcher(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(result) = self.note_switcher_results.get(self.note_switcher_selected).cloned() {
            self.active_overlay = None;
            self.note_switcher_results.clear();
            self.note_switcher_all.clear();
            // Switch to the workspace tab
            if result.ws_index < self.workspaces.len() {
                self.switch_workspace(result.ws_index, window, cx);
                // Focus the specific pane
                let ws = &mut self.workspaces[self.active_workspace];
                if ws.panes.contains_key(&result.pane_id) {
                    ws.pane_focus_history.push(ws.focused_pane);
                    ws.focused_pane = result.pane_id;
                }
                self.focus_pane_editor(result.pane_id, window, cx);
                self.sync_file_tree_selection(cx);
            }
            cx.notify();
        }
    }

    fn collect_open_notes(&self, cx: &App) -> Vec<NoteSwitcherResult> {
        let mut results = Vec::new();
        for (ws_idx, ws) in self.workspaces.iter().enumerate() {
            for (&pane_id, pane) in &ws.panes {
                if let Some(path) = &pane.active_path {
                    let filename = path.file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    results.push(NoteSwitcherResult {
                        ws_index: ws_idx,
                        pane_id,
                        path: path.clone(),
                        ws_title: ws.title.clone(),
                        filename,
                        is_title_match: true,
                        content_snippet: None,
                    });
                }
            }
        }
        // Read content for all open editors (for content search)
        for result in &mut results {
            for ws in &self.workspaces {
                if let Some(pane) = ws.panes.get(&result.pane_id) {
                    if let Some(editor) = &pane.editor {
                        let text = editor.read(cx).text(cx);
                        if !text.is_empty() {
                            result.content_snippet = Some(text);
                        }
                    }
                    break;
                }
            }
        }
        results
    }

    pub(crate) fn filter_note_switcher(&mut self, query: &str, _cx: &mut Context<Self>) {
        use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
        use nucleo_matcher::{Config, Matcher, Utf32Str};

        self.note_switcher_selected = 0;

        if query.is_empty() {
            self.note_switcher_results = self.note_switcher_all.clone();
            return;
        }

        let mut matcher = Matcher::new(Config::DEFAULT);
        let pattern = Pattern::new(
            query,
            CaseMatching::Ignore,
            Normalization::Smart,
            AtomKind::Fuzzy,
        );

        // Score by title match
        let mut title_matches: Vec<(u32, usize)> = Vec::new();
        let mut content_matches: Vec<(u32, usize)> = Vec::new();
        let mut title_matched_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();

        for (i, result) in self.note_switcher_all.iter().enumerate() {
            let mut buf = Vec::new();
            let haystack = Utf32Str::new(&result.filename, &mut buf);
            if let Some(score) = pattern.score(haystack, &mut matcher) {
                title_matches.push((score, i));
                title_matched_indices.insert(i);
            }
        }

        // Score by content match (only for items not already title-matched)
        for (i, result) in self.note_switcher_all.iter().enumerate() {
            if title_matched_indices.contains(&i) { continue; }
            if let Some(ref content) = result.content_snippet {
                // Search line-by-line for a matching snippet
                let query_lower = query.to_lowercase();
                for line in content.lines() {
                    if line.to_lowercase().contains(&query_lower) {
                        let snippet = line.trim();
                        let truncated = if snippet.chars().count() > 80 {
                            format!("{}...", snippet.chars().take(80).collect::<String>())
                        } else {
                            snippet.to_string()
                        };
                        content_matches.push((0, i));
                        // Store the snippet on a cloned result
                        // We'll set it when building final results
                        let _ = truncated; // used below
                        break;
                    }
                }
            }
        }

        title_matches.sort_by_key(|m| std::cmp::Reverse(m.0));
        // Don't need to sort content_matches since they all have score 0

        self.note_switcher_results = Vec::new();
        for &(_, i) in &title_matches {
            let mut r = self.note_switcher_all[i].clone();
            r.is_title_match = true;
            r.content_snippet = None; // Don't show snippet for title matches
            self.note_switcher_results.push(r);
        }
        for &(_, i) in &content_matches {
            let mut r = self.note_switcher_all[i].clone();
            r.is_title_match = false;
            // Find and set the matching snippet
            if let Some(ref content) = self.note_switcher_all[i].content_snippet {
                let query_lower = query.to_lowercase();
                for line in content.lines() {
                    if line.to_lowercase().contains(&query_lower) {
                        let snippet = line.trim();
                        r.content_snippet = Some(if snippet.chars().count() > 80 {
                            format!("{}...", snippet.chars().take(80).collect::<String>())
                        } else {
                            snippet.to_string()
                        });
                        break;
                    }
                }
            }
            self.note_switcher_results.push(r);
        }
    }

    /// Dismiss whatever overlay is currently active.
    pub(crate) fn dismiss_overlays(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.active_overlay.take() {
            Some(OverlayKind::FileFinder) => {
                self.file_finder.close();
                self.folder_move_source = None;
                self.clear_finder_match_highlights(cx);
            }
            Some(OverlayKind::Palette) => {
                self.rename_mode = None;
                self.palette.close();
            }
            Some(OverlayKind::AgenticSearch) => {
                self.agentic_loading = false;
            }
            Some(OverlayKind::NoteSwitcher) => {
                self.note_switcher_results.clear();
                self.note_switcher_all.clear();
            }
            None => return,
        }
        if !self.workspaces.is_empty() {
            let focused = self.active_ws().focused_pane;
            self.focus_pane_editor(focused, window, cx);
        }
        cx.notify();
    }
}
