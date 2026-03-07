use gpui::*;

use crate::palette::PaletteCommand;
use crate::theme::ThemeName;

use super::*;

impl GhostAppView {
    pub(crate) fn palette_commands() -> Vec<PaletteCommand> {
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
            PaletteCommand { label: "Collapse All Folders".into(), shortcut_hint: None, action_id: "collapse_all".into() },
            PaletteCommand { label: "Expand All Folders".into(), shortcut_hint: None, action_id: "expand_all".into() },
            PaletteCommand { label: "Theme: Rose Pine".into(), shortcut_hint: None, action_id: "theme_rose_pine".into() },
            PaletteCommand { label: "Theme: Nord".into(), shortcut_hint: None, action_id: "theme_nord".into() },
            PaletteCommand { label: "Theme: Solarized".into(), shortcut_hint: None, action_id: "theme_solarized".into() },
            PaletteCommand { label: "Theme: Dracula".into(), shortcut_hint: None, action_id: "theme_dracula".into() },
            PaletteCommand { label: "Theme: Light".into(), shortcut_hint: None, action_id: "theme_light".into() },
            PaletteCommand { label: "AI: Rename Tab".into(), shortcut_hint: None, action_id: "ai_rename_tab".into() },
            PaletteCommand { label: "AI: Rename All Tabs".into(), shortcut_hint: None, action_id: "ai_rename_all_tabs".into() },
            PaletteCommand { label: "AI: Rename File".into(), shortcut_hint: None, action_id: "ai_rename_file".into() },
            PaletteCommand { label: "AI: Suggest Folder".into(), shortcut_hint: None, action_id: "ai_suggest_folder".into() },
            PaletteCommand { label: "Move to Folder...".into(), shortcut_hint: None, action_id: "move_to_folder".into() },
            PaletteCommand { label: "Delete Current File".into(), shortcut_hint: Some("Cmd+\u{232b}".into()), action_id: "delete_file".into() },
            PaletteCommand { label: "Quit".into(), shortcut_hint: Some("Cmd+Q".into()), action_id: "quit".into() },
        ]
    }

    /// Dispatch a palette command by action_id.
    pub(crate) fn dispatch_palette_action(&mut self, action_id: &str, window: &mut Window, cx: &mut Context<Self>) {
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
                    self.clear_deleted_panes(self.active_workspace);
                    let focused = self.workspaces[self.active_workspace].focused_pane;
                    self.focus_pane_editor(focused, window, cx);
                    cx.notify();
                }
            }
            "split_right" => self.split(SplitDirection::Vertical, window, cx),
            "split_down" => self.split(SplitDirection::Horizontal, window, cx),
            "toggle_sidebar" => { self.sidebar_visible = !self.sidebar_visible; cx.notify(); }
            "rename_file" => {
                if let Some(path) = self.focused_active_path() {
                    if !self.sidebar_visible {
                        self.sidebar_visible = !self.sidebar_visible;
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
            "collapse_all" => {
                self.file_tree.update(cx, |tree, cx| tree.collapse_all(cx));
            }
            "expand_all" => {
                self.file_tree.update(cx, |tree, cx| tree.expand_all(cx));
            }
            "theme_rose_pine" => self.switch_theme(ThemeName::RosePine, cx),
            "theme_nord" => self.switch_theme(ThemeName::Nord, cx),
            "theme_solarized" => self.switch_theme(ThemeName::Solarized, cx),
            "theme_dracula" => self.switch_theme(ThemeName::Dracula, cx),
            "theme_light" => self.switch_theme(ThemeName::Light, cx),
            "ai_rename_tab" => self.ai_rename_tab(cx),
            "ai_rename_all_tabs" => self.ai_rename_all_tabs(cx),
            "ai_rename_file" => self.ai_rename_file(cx),
            "ai_suggest_folder" => self.ai_suggest_folder(cx),
            "move_to_folder" => self.start_move_to_folder(window, cx),
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
    pub(crate) fn palette_move_up(&mut self, cx: &mut Context<Self>) {
        if self.palette.selected_index > 0 {
            self.palette.selected_index -= 1;
            self.palette_scroll.scroll_to_item(self.palette.selected_index);
            cx.notify();
        }
    }

    /// Move palette selection down.
    pub(crate) fn palette_move_down(&mut self, cx: &mut Context<Self>) {
        let count = self.palette.filtered_commands().len();
        if count > 0 && self.palette.selected_index < count - 1 {
            self.palette.selected_index += 1;
            self.palette_scroll.scroll_to_item(self.palette.selected_index);
            cx.notify();
        }
    }

    /// Confirm the selected palette command.
    pub(crate) fn palette_confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let filtered = self.palette.filtered_commands();
        if let Some(cmd) = filtered.get(self.palette.selected_index) {
            let action_id = cmd.action_id.clone();
            self.active_overlay = None;
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
    pub(crate) fn enter_rename_mode(&mut self, _mode: RenameMode, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspaces.is_empty() { return; }
        let current_value = self.active_ws().title.clone();
        self.rename_mode = Some(RenameMode::Tab);
        self.active_overlay = Some(OverlayKind::Palette);
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
    pub(crate) fn apply_rename(&mut self, new_name: &str, _mode: &RenameMode, _window: &mut Window, _cx: &mut Context<Self>) {
        if self.workspaces.is_empty() { return; }
        self.active_ws_mut().title = new_name.to_string();
    }
}
