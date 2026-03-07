use gpui::*;
use gpui_component::input::Input;
use gpui_component::resizable::{h_resizable, v_resizable, resizable_panel};

use crate::theme::{rgb_to_hsla, GhostTheme};

use super::*;

impl GhostAppView {
    pub(crate) fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

            let ai_busy = self.ai_loading.contains(&ws.id);
            let display = if ai_busy {
                format!("{} …", ws.title)
            } else if dirty {
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

    pub(crate) fn render_split_node(&self, node: &SplitNode, ws: &Workspace, cx: &mut Context<Self>) -> AnyElement {
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
                            ws.pane_focus_history.push(ws.focused_pane);
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
                    if is_focused && self.overlay_is(OverlayKind::Search) {
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

    pub(crate) fn render_file_finder(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let ghost = GhostTheme::from_name(self.active_theme);
        let overlay_bg = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
        let fg = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
        let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);
        let selection_bg = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);
        let hint_fg = rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2);

        let root_prefix = self.root.to_string_lossy().to_string();

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
                    let truncated = if line_preview.chars().count() > 60 {
                        let end: String = line_preview.chars().take(60).collect();
                        format!("{}…", end)
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

        let count_text = if self.folder_move_source.is_some() {
            format!("{} folders", self.file_finder.result_count())
        } else {
            format!("{} files", self.file_finder.result_count())
        };

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

    pub(crate) fn render_agentic_search(&self, cx: &mut Context<Self>) -> Stateful<Div> {
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

    pub(crate) fn render_location_picker(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let ghost = GhostTheme::from_name(self.active_theme);
        let overlay_bg = rgb_to_hsla(ghost.sidebar_bg.0, ghost.sidebar_bg.1, ghost.sidebar_bg.2);
        let fg = rgb_to_hsla(ghost.fg.0, ghost.fg.1, ghost.fg.2);
        let border_color = rgb_to_hsla(ghost.border.0, ghost.border.1, ghost.border.2);
        let selection_bg = rgb_to_hsla(ghost.selection.0, ghost.selection.1, ghost.selection.2);
        let hint_fg = rgb_to_hsla(ghost.line_number.0, ghost.line_number.1, ghost.line_number.2);

        let mut list = div().flex().flex_col();

        for (i, (label, _)) in self.location_picker_options.iter().enumerate() {
            let is_selected = i == self.location_picker_selected;
            let bg = if is_selected { selection_bg } else { overlay_bg };
            let idx = i;

            list = list.child(
                div()
                    .id(ElementId::NamedInteger("loc-item".into(), i as u64))
                    .w_full()
                    .px(px(12.0))
                    .py(px(6.0))
                    .bg(bg)
                    .text_color(fg)
                    .text_sm()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this: &mut Self, _, window, cx| {
                        this.location_picker_selected = idx;
                        this.confirm_location_picker(window, cx);
                    }))
                    .child(label.clone()),
            );
        }

        div()
            .id("location-picker-dismiss-bg")
            .absolute()
            .inset_0()
            .on_click(cx.listener(|this: &mut Self, _, window, cx| {
                this.close_location_picker(window, cx);
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
                            .id("location-picker-card")
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
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .border_b_1()
                                    .border_color(border_color)
                                    .text_sm()
                                    .text_color(hint_fg)
                                    .child("Create note in:"),
                            )
                            .child(list),
                    ),
            )
    }

    pub(crate) fn render_command_palette(&self, cx: &mut Context<Self>) -> Stateful<Div> {
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
                        this.active_overlay = None;
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
