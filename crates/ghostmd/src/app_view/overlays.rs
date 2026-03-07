use gpui::*;

use super::*;

impl GhostAppView {
    /// Close the file finder and refocus the editor.
    pub(crate) fn close_file_finder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_file_finder = false;
        self.file_finder.close();
        self.folder_move_source = None;
        if !self.workspaces.is_empty() {
            let focused = self.active_ws().focused_pane;
            self.focus_pane_editor(focused, window, cx);
        }
        cx.notify();
    }

    /// Close the command palette and refocus the editor.
    pub(crate) fn close_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
    pub(crate) fn open_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_search = true;
        self.search_match_count = 0;
        self.search_input.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }

    /// Close the search bar and refocus the editor.
    pub(crate) fn close_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_search = false;
        self.search_match_count = 0;
        if !self.workspaces.is_empty() {
            let focused = self.active_ws().focused_pane;
            self.focus_pane_editor(focused, window, cx);
        }
        cx.notify();
    }

    pub(crate) fn open_agentic_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_agentic_search = true;
        self.agentic_results.clear();
        self.agentic_loading = false;
        self.agentic_input.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }

    pub(crate) fn close_agentic_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_agentic_search = false;
        self.agentic_loading = false;
        if !self.workspaces.is_empty() {
            let focused = self.active_ws().focused_pane;
            self.focus_pane_editor(focused, window, cx);
        }
        cx.notify();
    }

    /// Dismiss any open overlays (palette, finder, agentic search, location picker).
    pub(crate) fn dismiss_overlays(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.show_file_finder {
            self.close_file_finder(window, cx);
        }
        if self.show_agentic_search {
            self.close_agentic_search(window, cx);
        }
        if self.show_palette {
            self.close_palette(window, cx);
        }
        if self.show_location_picker {
            self.close_location_picker(window, cx);
        }
    }
}
