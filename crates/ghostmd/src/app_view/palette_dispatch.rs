use gpui::*;

use crate::palette::PaletteCommand;
use crate::theme::ThemeName;

use super::*;

/// Returns the platform-specific modifier key name for display in UI hints.
fn mod_key() -> &'static str {
    if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" }
}

impl GhostAppView {
    pub(crate) fn palette_commands() -> Vec<PaletteCommand> {
        let m = mod_key();
        vec![
            PaletteCommand { label: "New Note".into(), shortcut_hint: Some(format!("{m}+N")), action_id: "new_note".into() },
            PaletteCommand { label: "New Workspace".into(), shortcut_hint: Some(format!("{m}+T")), action_id: "new_workspace".into() },
            PaletteCommand { label: "New Window".into(), shortcut_hint: Some(format!("{m}+Shift+N")), action_id: "new_window".into() },
            PaletteCommand { label: "Save".into(), shortcut_hint: Some(format!("{m}+S")), action_id: "save".into() },
            PaletteCommand { label: "Close Pane".into(), shortcut_hint: Some(format!("{m}+W")), action_id: "close_pane".into() },
            PaletteCommand { label: "Restore Workspace".into(), shortcut_hint: Some(format!("{m}+Shift+T")), action_id: "restore_workspace".into() },
            PaletteCommand { label: "Split Right".into(), shortcut_hint: Some(format!("{m}+D")), action_id: "split_right".into() },
            PaletteCommand { label: "Split Down".into(), shortcut_hint: Some(format!("{m}+Shift+D")), action_id: "split_down".into() },
            PaletteCommand { label: "Toggle Sidebar".into(), shortcut_hint: Some(format!("{m}+B")), action_id: "toggle_sidebar".into() },
            PaletteCommand { label: "Rename File...".into(), shortcut_hint: None, action_id: "rename_file".into() },
            PaletteCommand { label: "Rename Tab...".into(), shortcut_hint: None, action_id: "rename_tab".into() },
            PaletteCommand {
                label: if cfg!(target_os = "macos") { "Open in Finder" } else { "Open in File Manager" }.into(),
                shortcut_hint: None,
                action_id: "open_in_finder".into(),
            },
            PaletteCommand { label: "Collapse All Folders".into(), shortcut_hint: None, action_id: "collapse_all".into() },
            PaletteCommand { label: "Expand All Folders".into(), shortcut_hint: None, action_id: "expand_all".into() },
            PaletteCommand { label: "Theme: Ayu Dark (dark)".into(), shortcut_hint: None, action_id: "theme_ayu_dark".into() },
            PaletteCommand { label: "Theme: Catppuccin (dark)".into(), shortcut_hint: None, action_id: "theme_catppuccin".into() },
            PaletteCommand { label: "Theme: Dracula (dark)".into(), shortcut_hint: None, action_id: "theme_dracula".into() },
            PaletteCommand { label: "Theme: Everforest (dark)".into(), shortcut_hint: None, action_id: "theme_everforest".into() },
            PaletteCommand { label: "Theme: Gruvbox (dark)".into(), shortcut_hint: None, action_id: "theme_gruvbox".into() },
            PaletteCommand { label: "Theme: Kanagawa (dark)".into(), shortcut_hint: None, action_id: "theme_kanagawa".into() },
            PaletteCommand { label: "Theme: Catppuccin Latte (light)".into(), shortcut_hint: None, action_id: "theme_catppuccin_latte".into() },
            PaletteCommand { label: "Theme: GitHub Light (light)".into(), shortcut_hint: None, action_id: "theme_github_light".into() },
            PaletteCommand { label: "Theme: Light (light)".into(), shortcut_hint: None, action_id: "theme_light".into() },
            PaletteCommand { label: "Theme: Rosé Pine Dawn (light)".into(), shortcut_hint: None, action_id: "theme_rose_pine_dawn".into() },
            PaletteCommand { label: "Theme: Solarized Light (light)".into(), shortcut_hint: None, action_id: "theme_solarized_light".into() },
            PaletteCommand { label: "Theme: Ayu Light (light)".into(), shortcut_hint: None, action_id: "theme_ayu_light".into() },
            PaletteCommand { label: "Theme: Gruvbox Light (light)".into(), shortcut_hint: None, action_id: "theme_gruvbox_light".into() },
            PaletteCommand { label: "Theme: Everforest Light (light)".into(), shortcut_hint: None, action_id: "theme_everforest_light".into() },
            PaletteCommand { label: "Theme: Nord Light (light)".into(), shortcut_hint: None, action_id: "theme_nord_light".into() },
            PaletteCommand { label: "Theme: Tokyo Night Day (light)".into(), shortcut_hint: None, action_id: "theme_tokyo_night_day".into() },
            PaletteCommand { label: "Theme: Moonlight (dark)".into(), shortcut_hint: None, action_id: "theme_moonlight".into() },
            PaletteCommand { label: "Theme: Nord (dark)".into(), shortcut_hint: None, action_id: "theme_nord".into() },
            PaletteCommand { label: "Theme: One Dark (dark)".into(), shortcut_hint: None, action_id: "theme_one_dark".into() },
            PaletteCommand { label: "Theme: Palenight (dark)".into(), shortcut_hint: None, action_id: "theme_palenight".into() },
            PaletteCommand { label: "Theme: Rose Pine (dark)".into(), shortcut_hint: None, action_id: "theme_rose_pine".into() },
            PaletteCommand { label: "Theme: Solarized (dark)".into(), shortcut_hint: None, action_id: "theme_solarized".into() },
            PaletteCommand { label: "Theme: Tokyo Night (dark)".into(), shortcut_hint: None, action_id: "theme_tokyo_night".into() },
            PaletteCommand { label: "Theme: Vesper (dark)".into(), shortcut_hint: None, action_id: "theme_vesper".into() },
            PaletteCommand { label: "AI: Rename Tab".into(), shortcut_hint: None, action_id: "ai_rename_tab".into() },
            PaletteCommand { label: "AI: Rename All Tabs".into(), shortcut_hint: None, action_id: "ai_rename_all_tabs".into() },
            PaletteCommand { label: "AI: Rename File".into(), shortcut_hint: None, action_id: "ai_rename_file".into() },
            PaletteCommand { label: "AI: Suggest Folder".into(), shortcut_hint: None, action_id: "ai_suggest_folder".into() },
            PaletteCommand { label: "Share as Gist".into(), shortcut_hint: None, action_id: "share_gist".into() },
            PaletteCommand { label: "Move to Folder...".into(), shortcut_hint: None, action_id: "move_to_folder".into() },
            PaletteCommand { label: "Toggle Syntax Highlighting".into(), shortcut_hint: None, action_id: "toggle_syntax_highlight".into() },
            PaletteCommand { label: "Delete Current File".into(), shortcut_hint: Some(format!("{m}+\u{232b}")), action_id: "delete_file".into() },
            PaletteCommand { label: "Quit".into(), shortcut_hint: Some(format!("{m}+Q")), action_id: "quit".into() },
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
                    #[cfg(target_os = "macos")]
                    { std::process::Command::new("open").arg("-R").arg(&path).spawn().ok(); }
                    #[cfg(not(target_os = "macos"))]
                    {
                        let dir = path.parent().unwrap_or(&path);
                        std::process::Command::new("xdg-open").arg(dir).spawn().ok();
                    }
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
            "theme_gruvbox" => self.switch_theme(ThemeName::Gruvbox, cx),
            "theme_catppuccin" => self.switch_theme(ThemeName::Catppuccin, cx),
            "theme_tokyo_night" => self.switch_theme(ThemeName::TokyoNight, cx),
            "theme_kanagawa" => self.switch_theme(ThemeName::Kanagawa, cx),
            "theme_everforest" => self.switch_theme(ThemeName::Everforest, cx),
            "theme_one_dark" => self.switch_theme(ThemeName::OneDark, cx),
            "theme_moonlight" => self.switch_theme(ThemeName::Moonlight, cx),
            "theme_ayu_dark" => self.switch_theme(ThemeName::AyuDark, cx),
            "theme_palenight" => self.switch_theme(ThemeName::Palenight, cx),
            "theme_vesper" => self.switch_theme(ThemeName::Vesper, cx),
            "theme_solarized_light" => self.switch_theme(ThemeName::SolarizedLight, cx),
            "theme_catppuccin_latte" => self.switch_theme(ThemeName::CatppuccinLatte, cx),
            "theme_rose_pine_dawn" => self.switch_theme(ThemeName::RosePineDawn, cx),
            "theme_github_light" => self.switch_theme(ThemeName::GithubLight, cx),
            "theme_ayu_light" => self.switch_theme(ThemeName::AyuLight, cx),
            "theme_gruvbox_light" => self.switch_theme(ThemeName::GruvboxLight, cx),
            "theme_everforest_light" => self.switch_theme(ThemeName::EverforestLight, cx),
            "theme_nord_light" => self.switch_theme(ThemeName::NordLight, cx),
            "theme_tokyo_night_day" => self.switch_theme(ThemeName::TokyoNightDay, cx),
            "ai_rename_tab" => self.ai_rename_tab(cx),
            "ai_rename_all_tabs" => self.ai_rename_all_tabs(cx),
            "ai_rename_file" => self.ai_rename_file(cx),
            "ai_suggest_folder" => self.ai_suggest_folder(cx),
            "share_gist" => self.share_as_gist(cx),
            "move_to_folder" => self.start_move_to_folder(window, cx),
            "toggle_syntax_highlight" => self.toggle_syntax_highlight(window, cx),
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
