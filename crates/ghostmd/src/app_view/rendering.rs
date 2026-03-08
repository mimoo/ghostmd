use std::path::Path;

use gpui::*;
use gpui_component::input::Input;
use gpui_component::resizable::{h_resizable, v_resizable, resizable_panel};

use super::*;

impl GhostAppView {
    pub(crate) fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = &self.theme;

        let mut tabs = div()
            .w_full()
            .h(px(36.0))
            .flex()
            .flex_row()
            .items_center()
            .bg(t.bg)
            .border_b_1()
            .border_color(t.border)
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
                const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                let frame = SPINNER[self.ai_anim_frame % SPINNER.len()];
                format!("{} {}", ws.title, frame)
            } else if dirty {
                format!("{} *", ws.title)
            } else {
                ws.title.clone()
            };

            let tab_bg = if is_active { t.tab_active } else { t.tab_inactive };

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
                .text_color(t.fg)
                .cursor_pointer()
                .on_click(cx.listener(move |this: &mut Self, _event, window, cx| {
                    this.switch_workspace(ws_idx, window, cx);
                }))
                .child(display)
                .child(
                    div()
                        .id(ElementId::NamedInteger("ws-close".into(), i as u64))
                        .text_xs()
                        .text_color(t.hint)
                        .opacity(0.0)
                        .group_hover(SharedString::from(format!("tab-{}", i)), |s| s.opacity(1.0))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this: &mut Self, _event: &ClickEvent, window, cx| {
                            this.close_workspace(close_idx, window, cx);
                        }))
                        .child("\u{00d7}"),
                );

            if is_active {
                tab_div = tab_div.border_b_2().border_color(t.accent);
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
                .text_color(t.hint)
                .cursor_pointer()
                .on_click(cx.listener(|this: &mut Self, _event, window, cx| {
                    this.new_workspace_tab(window, cx);
                }))
                .child("+"),
        );

        tabs
    }

    pub(crate) fn render_split_node(&self, node: &SplitNode, ws: &Workspace, cx: &mut Context<Self>) -> AnyElement {
        let t = &self.theme;
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
                    .min_h(px(0.0))
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .bg(t.bg)
                    .text_color(t.fg)
                    .capture_any_mouse_down(cx.listener(move |this: &mut Self, _event: &MouseDownEvent, window, cx| {
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
                        pane_div = pane_div.border_2().border_color(t.accent);
                    } else {
                        pane_div = pane_div.border_2().border_color(hsla(0., 0., 0., 0.)).opacity(0.85);
                    }
                }

                if has_editor {
                    // Title bar + editor — split path into dir (muted) + filename (bright)
                    let active_path = pane.and_then(|p| p.active_path.as_ref());
                    let pane_dirty = pane
                        .and_then(|p| p.editor.as_ref())
                        .map(|e| e.read(cx).dirty)
                        .unwrap_or(false);

                    let (dir_part, file_part) = active_path
                        .map(|p| {
                            let full = p.display().to_string();
                            match full.rfind('/') {
                                Some(i) => (full[..=i].to_string(), full[i+1..].to_string()),
                                None => (String::new(), full),
                            }
                        })
                        .unwrap_or_else(|| (String::new(), "untitled".to_string()));

                    let file_part = if pane_dirty {
                        format!("{} ●", file_part)
                    } else {
                        file_part
                    };

                    // Check for active move transition on this pane's path
                    let move_old = self.move_transition.as_ref().and_then(|(old, new, started)| {
                        if active_path == Some(new) {
                            let elapsed = started.elapsed().as_millis() as f32;
                            let fade = (1.0 - elapsed / 4000.0).max(0.0);
                            if fade > 0.0 {
                                Some((old.display().to_string(), fade))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    });

                    let mut title_row = div().flex().flex_row();
                    if let Some((old_path_str, fade)) = move_old {
                        title_row = title_row
                            .child(div().text_color(t.error.opacity(fade)).child(old_path_str))
                            .child(div().text_color(t.pane_title_fg).child(" → "));
                    }
                    title_row = title_row
                        .child(div().text_color(t.pane_title_fg).child(dir_part))
                        .child(div().text_color(t.fg).child(file_part));

                    let title_bar = div()
                        .w_full()
                        .h(px(24.0))
                        .flex()
                        .items_center()
                        .px(px(8.0))
                        .bg(t.pane_title_bg)
                        .text_xs()
                        .child(title_row);

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
                            .bg(t.sidebar_bg)
                            .child(div().text_lg().text_color(t.hint).child("No file open"))
                            .child(div().text_sm().text_color(t.hint).child(format!("{}+N  Create a new note", if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" })))
                            .child(div().text_sm().text_color(t.hint).child(format!("{}+P  Search files", if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" }))),
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
        let t = &self.theme;
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
            let bg = if is_selected { t.selection } else { t.sidebar_bg };

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
                    .text_color(t.fg)
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
                            .bg(t.sidebar_bg)
                            .border_1()
                            .border_color(t.border)
                            .rounded(px(8.0))
                            .shadow_lg()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(6.0))
                                    .border_b_1()
                                    .border_color(t.border)
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
                                    .text_color(t.hint)
                                    .child(count_text),
                            ),
                    ),
            )
    }

    pub(crate) fn render_agentic_search(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let t = &self.theme;
        let root_display = self.root.to_string_lossy().to_string();

        let mut results_div = div()
            .id("agentic-results")
            .flex()
            .flex_col()
            .max_h(px(400.0))
            .overflow_y_scroll()
            .track_scroll(&self.agentic_scroll);

        if self.agentic_loading {
            results_div = results_div.child(
                div()
                    .px(px(12.0))
                    .py(px(8.0))
                    .text_sm()
                    .text_color(t.accent)
                    .child("Searching with Claude..."),
            );
        } else {
            for (i, m) in self.agentic_results.iter().enumerate() {
                let selected = i == self.agentic_selected;
                let is_error = m.file.is_empty();

                let display_path = if is_error {
                    m.quote.clone()
                } else {
                    let short = m.file.strip_prefix(&root_display)
                        .or_else(|| m.file.strip_prefix("/"))
                        .unwrap_or(&m.file)
                        .trim_start_matches('/')
                        .to_string();
                    if m.line > 0 {
                        format!("{}:{}", short, m.line)
                    } else {
                        short
                    }
                };

                let idx = i;
                let row = div()
                    .id(ElementId::NamedInteger("agentic-line".into(), i as u64))
                    .w_full()
                    .px(px(12.0))
                    .py(px(4.0))
                    .when(selected, |d| d.bg(t.selection))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this: &mut Self, _, window, cx| {
                        this.open_agentic_result(idx, window, cx);
                    }))
                    .child(
                        div()
                            .text_sm()
                            .text_color(if is_error { t.accent } else { t.fg })
                            .child(display_path),
                    )
                    .when(!is_error && !m.quote.is_empty(), |d| {
                        let quote = m.quote.chars().take(120).collect::<String>();
                        d.child(
                            div()
                                .text_xs()
                                .text_color(t.hint)
                                .child(quote),
                        )
                    });
                results_div = results_div.child(row);
            }
        }

        let status = if self.agentic_loading {
            "Running...".to_string()
        } else if self.agentic_results.is_empty() {
            "Press Enter to search".to_string()
        } else {
            format!("{} results", self.agentic_results.len())
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
                            .bg(t.sidebar_bg)
                            .border_1()
                            .border_color(t.border)
                            .rounded(px(8.0))
                            .shadow_lg()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(6.0))
                                    .border_b_1()
                                    .border_color(t.border)
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
                                    .text_color(t.hint)
                                    .child(status),
                            ),
                    ),
            )
    }

    pub(crate) fn render_location_picker(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let t = &self.theme;

        let mut list = div().flex().flex_col();

        for (i, (label, _)) in self.location_picker_options.iter().enumerate() {
            let is_selected = i == self.location_picker_selected;
            let bg = if is_selected { t.selection } else { t.sidebar_bg };
            let idx = i;

            list = list.child(
                div()
                    .id(ElementId::NamedInteger("loc-item".into(), i as u64))
                    .w_full()
                    .px(px(12.0))
                    .py(px(6.0))
                    .bg(bg)
                    .text_color(t.fg)
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
                            .bg(t.sidebar_bg)
                            .border_1()
                            .border_color(t.border)
                            .rounded(px(8.0))
                            .shadow_lg()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .px(px(12.0))
                                    .py(px(6.0))
                                    .border_b_1()
                                    .border_color(t.border)
                                    .text_sm()
                                    .text_color(t.hint)
                                    .child("Create note in:"),
                            )
                            .child(list),
                    ),
            )
    }

    pub(crate) fn render_command_palette(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let t = &self.theme;

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
                    .text_color(t.hint)
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
                let bg = if is_selected { t.selection } else { t.sidebar_bg };
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
                    .text_color(t.fg)
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
                            .text_color(t.hint)
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
                            .bg(t.sidebar_bg)
                            .border_1()
                            .border_color(t.border)
                            .rounded(px(8.0))
                            .shadow_lg()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .px(px(8.0))
                                    .py(px(6.0))
                                    .border_b_1()
                                    .border_color(t.border)
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

    pub(crate) fn render_context_menu(&self, path: &Path, position: Point<Pixels>, cx: &mut Context<Self>) -> Div {
        let t = &self.theme;
        let is_file = path.is_file();
        let is_dir = path.is_dir();
        let is_root = *path == self.root;
        let diary_dir = self.root.join("diary");
        let is_diary_path = path.starts_with(&diary_dir);

        let context_dir = if is_dir {
            path.to_path_buf()
        } else {
            path.parent().unwrap_or(&self.root).to_path_buf()
        };

        let rename_path = path.to_path_buf();
        let new_note_dir = context_dir.clone();
        let new_folder_dir = context_dir;
        let finder_path = path.to_path_buf();
        let trash_path = path.to_path_buf();

        let mut menu = div()
            .absolute()
            .top(position.y)
            .left(position.x)
            .bg(t.sidebar_bg)
            .border_1()
            .border_color(t.border)
            .rounded(px(4.0))
            .shadow_lg()
            .min_w(px(160.0))
            .flex()
            .flex_col();

        let rename_enabled = if is_file {
            !is_diary_path
        } else if is_dir {
            !is_root && *path != diary_dir && !is_diary_path
        } else {
            false
        };
        let show_rename = rename_enabled || (is_dir && !is_root && (is_diary_path || *path == diary_dir));
        if show_rename {
            menu = menu.child(
                div()
                    .id("ctx-rename")
                    .px(px(12.0))
                    .py(px(6.0))
                    .text_sm()
                    .text_color(if rename_enabled { t.fg } else { t.hint })
                    .when(rename_enabled, |d| {
                        d.cursor_pointer()
                            .hover(|s| s.bg(t.selection))
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

        menu = menu.child(
            div()
                .id("ctx-new-note")
                .px(px(12.0))
                .py(px(6.0))
                .text_sm()
                .text_color(t.fg)
                .cursor_pointer()
                .hover(|s| s.bg(t.selection))
                .on_click(cx.listener(move |this: &mut Self, _event, window, cx| {
                    this.tree_context_menu = None;
                    this.new_note_in_dir(new_note_dir.clone(), window, cx);
                }))
                .child("New Note"),
        );

        let new_folder_in_diary = new_folder_dir.starts_with(&diary_dir);
        if !new_folder_in_diary {
            menu = menu.child(
                div()
                    .id("ctx-new-folder")
                    .px(px(12.0))
                    .py(px(6.0))
                    .text_sm()
                    .text_color(t.fg)
                    .cursor_pointer()
                    .hover(|s| s.bg(t.selection))
                    .on_click(cx.listener(move |this: &mut Self, _event, window, cx| {
                        this.tree_context_menu = None;
                        this.create_new_folder(new_folder_dir.clone(), window, cx);
                    }))
                    .child("New Folder"),
            );
        }

        menu = menu.child(
            div()
                .id("ctx-open-finder")
                .px(px(12.0))
                .py(px(6.0))
                .text_sm()
                .text_color(t.fg)
                .cursor_pointer()
                .hover(|s| s.bg(t.selection))
                .on_click(cx.listener(move |this: &mut Self, _event, _window, cx| {
                    this.tree_context_menu = None;
                    std::process::Command::new("open").arg("-R").arg(&finder_path).spawn().ok();
                    cx.notify();
                }))
                .child("Open in Finder"),
        );

        if !is_root {
            menu = menu.child(
                div()
                    .id("ctx-move-to-trash")
                    .px(px(12.0))
                    .py(px(6.0))
                    .text_sm()
                    .text_color(t.error)
                    .cursor_pointer()
                    .hover(|s| s.bg(t.selection))
                    .on_click(cx.listener(move |this: &mut Self, _event, window, cx| {
                        this.tree_context_menu = None;
                        this.move_to_trash(trash_path.clone(), window, cx);
                    }))
                    .child("Move to Trash"),
            );
        }

        menu
    }
}
