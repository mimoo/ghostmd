use gpui::*;

use super::*;

impl GhostAppView {
    /// Close the file finder and refocus the editor.
    pub(crate) fn close_file_finder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.active_overlay = None;
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

    /// Dismiss whatever overlay is currently active.
    pub(crate) fn dismiss_overlays(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.active_overlay.take() {
            Some(OverlayKind::FileFinder) => {
                self.file_finder.close();
                self.folder_move_source = None;
            }
            Some(OverlayKind::Palette) => {
                self.rename_mode = None;
                self.palette.close();
            }
            Some(OverlayKind::AgenticSearch) => {
                self.agentic_loading = false;
            }
            Some(OverlayKind::LocationPicker) => {
                self.location_picker_options.clear();
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
