use std::path::Path;

use gpui::*;
use gpui_component::input::Input;
use gpui_component::resizable::{h_resizable, v_resizable, resizable_panel};

use super::*;

/// A single entry in a context menu (used for both tab and tree menus).
pub(crate) struct ContextMenuEntry {
    pub id: &'static str,
    pub label: String,
    pub shortcut: Option<&'static str>,
    pub color: Hsla,
}

/// Approximate height of one context-menu row (text_sm + py(4)). Used for off-screen flipping.
const CONTEXT_MENU_ROW_HEIGHT: f32 = 24.0;
/// Vertical padding of the menu container (py(4) top + py(4) bottom).
const CONTEXT_MENU_OUTER_PAD: f32 = 8.0;
/// Approximate width of a context menu (min_w is 180 but rows with shortcuts can be wider).
const CONTEXT_MENU_WIDTH: f32 = 240.0;
/// Minimum gap to keep between the menu edge and the window edge.
const CONTEXT_MENU_EDGE_GAP: f32 = 4.0;

/// Shift the menu's top-left so it stays inside the window. If the menu would overflow
/// the bottom edge, flip it so its bottom is at `position.y` (matches native popup behavior).
pub(crate) fn clamp_menu_position(
    position: Point<Pixels>,
    entry_count: usize,
    window: &Window,
) -> Point<Pixels> {
    let viewport = window.viewport_size();
    let menu_height = px(CONTEXT_MENU_OUTER_PAD + entry_count as f32 * CONTEXT_MENU_ROW_HEIGHT);
    let menu_width = px(CONTEXT_MENU_WIDTH);
    let gap = px(CONTEXT_MENU_EDGE_GAP);

    let max_y = viewport.height - menu_height - gap;
    let y = if position.y + menu_height + gap > viewport.height {
        // Flip upward so the menu opens above the cursor.
        let flipped = position.y - menu_height;
        flipped.max(gap).min(max_y.max(gap))
    } else {
        position.y
    };

    let max_x = viewport.width - menu_width - gap;
    let x = position.x.min(max_x.max(gap));

    Point::new(x, y)
}

/// Small label shown under the cursor while dragging a tab.
struct DraggedTab {
    title: String,
    fg: Hsla,
    bg: Hsla,
}

impl Render for DraggedTab {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(12.0))
            .py(px(6.0))
            .text_sm()
            .bg(self.bg)
            .text_color(self.fg)
            .rounded(px(4.0))
            .shadow_md()
            .child(self.title.clone())
    }
}

impl GhostAppView {
    pub(crate) fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let t = &self.theme;
        let entity = cx.entity().clone();

        let mut tabs = div()
            .id("tab-bar")
            .w_full()
            .h(px(36.0))
            .flex()
            .flex_row()
            .items_center()
            .bg(t.bg)
            .border_b_1()
            .border_color(t.border)
            .overflow_x_hidden()
            // Dropping a tab on empty tab bar area cancels the drag (no tear-off)
            .on_drop(cx.listener(|this: &mut Self, _payload: &TabDragPayload, _window, _cx| {
                this.tab_drag_active = None;
            }));

        for (i, ws) in self.workspaces.iter().enumerate() {
            let is_active = i == self.active_workspace;

            let ai_busy = self.ai_loading.contains(&ws.id);
            let display = if ai_busy {
                const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
                let frame = SPINNER[self.ai_anim_frame % SPINNER.len()];
                format!("{} {}", ws.title, frame)
            } else {
                ws.title.clone()
            };

            let tab_bg = if is_active { t.tab_active } else { t.tab_inactive };

            let ws_idx = i;
            let close_idx = i;
            let drag_idx = i;
            let drop_idx = i;
            let drag_title = ws.title.clone();
            let drag_fg = t.fg;
            let drag_bg = t.tab_active;
            let accent = t.accent;
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
                .on_drag(TabDragPayload(drag_idx), {
                    let entity = entity.clone();
                    move |_payload, _offset, _window, cx| {
                        entity.update(cx, |this, _cx| {
                            this.tab_drag_active = Some(drag_idx);
                        });
                        cx.new(|_| DraggedTab {
                            title: drag_title.clone(),
                            fg: drag_fg,
                            bg: drag_bg,
                        })
                    }
                })
                .drag_over::<TabDragPayload>(move |style, _, _, _| {
                    style.border_l_2().border_color(accent)
                })
                .on_drop(cx.listener(move |this: &mut Self, payload: &TabDragPayload, window, cx| {
                    this.tab_drag_active = None;
                    this.reorder_workspace(payload.0, drop_idx, window, cx);
                }))
                .on_click(cx.listener(move |this: &mut Self, event: &ClickEvent, window, cx| {
                    this.switch_workspace(ws_idx, window, cx);
                    if event.click_count() >= 2 {
                        this.enter_rename_mode(RenameMode::Tab, window, cx);
                    }
                }))
                .on_mouse_down(MouseButton::Right, cx.listener(move |this: &mut Self, event: &MouseDownEvent, _window, cx| {
                    this.tree_context_menu = None;
                    this.tab_context_menu = Some((ws_idx, event.position));
                    this.context_menu_selected = 0;
                    cx.stop_propagation();
                    cx.notify();
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

    pub(crate) fn render_split_node(&self, node: &SplitNode, ws: &Workspace, tree_focused: bool, cx: &mut Context<Self>) -> AnyElement {
        let t = &self.theme;
        let multi_pane = ws.panes.len() > 1;

        match node {
            SplitNode::Leaf(pane_id) => {
                // When the sidebar tree owns keyboard focus, don't highlight the
                // pane as focused — otherwise it looks like both are active.
                let is_focused = *pane_id == ws.focused_pane && !tree_focused;
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
                    let pane_missing = pane
                        .and_then(|p| p.editor.as_ref())
                        .map(|e| e.read(cx).missing)
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

                    let mut title_row = div().flex().flex_row().items_center().gap(px(6.0));
                    if pane_missing {
                        title_row = title_row.child(
                            div().text_color(t.error).child("⚠ deleted"),
                        );
                    }
                    if let Some((old_path_str, fade)) = move_old {
                        title_row = title_row
                            .child(div().text_color(t.error.opacity(fade)).child(old_path_str))
                            .child(div().text_color(t.pane_title_fg).child(" → "));
                    }
                    let path_row = if pane_missing {
                        div().flex().flex_row().line_through().text_color(t.error)
                            .child(div().child(dir_part))
                            .child(div().child(file_part))
                    } else {
                        div().flex().flex_row()
                            .child(div().text_color(t.pane_title_fg).child(dir_part))
                            .child(div().text_color(t.fg).child(file_part))
                    };
                    title_row = title_row.child(path_row);

                    let reveal_path = active_path.cloned();
                    let ctx_path = active_path.cloned();
                    let mut title_bar = div()
                        .id(ElementId::NamedInteger("pane-title".into(), pid as u64))
                        .w_full()
                        .h(px(24.0))
                        .flex()
                        .items_center()
                        .px(px(8.0))
                        .bg(t.pane_title_bg)
                        .text_xs()
                        .cursor_pointer()
                        .on_click(cx.listener(move |this: &mut Self, _, _window, cx| {
                            if let Some(ref path) = reveal_path {
                                this.sidebar_visible = true;
                                this.file_tree.update(cx, |tree, cx| {
                                    tree.reveal_file(path, cx);
                                });
                                cx.notify();
                            }
                        }))
                        .child(title_row);
                    if let Some(p) = ctx_path {
                        title_bar = title_bar.on_mouse_down(
                            MouseButton::Right,
                            cx.listener(move |this: &mut Self, event: &MouseDownEvent, _window, cx| {
                                this.tab_context_menu = None;
                                // Ensure the sidebar is visible so inline rename (if chosen)
                                // is actually on screen.
                                this.sidebar_visible = true;
                                this.tree_context_menu = Some((p.clone(), event.position));
                                this.context_menu_selected = 0;
                                cx.stop_propagation();
                                cx.notify();
                            }),
                        );
                    }

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
                let left_el = self.render_split_node(left, ws, tree_focused, cx);
                let right_el = self.render_split_node(right, ws, tree_focused, cx);
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

    /// Shared overlay shell: backdrop + centered card with input header and optional footer.
    #[allow(clippy::too_many_arguments)]
    fn overlay_shell(
        &self,
        bg_id: &'static str,
        card_id: &'static str,
        width: f32,
        input: &Entity<InputState>,
        body: AnyElement,
        footer: Option<AnyElement>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let t = &self.theme;
        let mut card = div()
            .id(card_id)
            .on_click(cx.listener(|_this: &mut Self, _, _window, cx| {
                cx.stop_propagation();
            }))
            .w(px(width))
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
                    .child(Input::new(input).appearance(false).w_full()),
            )
            .child(body);
        if let Some(f) = footer {
            card = card.child(f);
        }
        div()
            .id(bg_id)
            .absolute()
            .inset_0()
            .on_click(cx.listener(|this: &mut Self, _, window, cx| {
                this.dismiss_overlays(window, cx);
            }))
            .child(
                div()
                    .absolute()
                    .top(px(60.0))
                    .left_0()
                    .right_0()
                    .flex()
                    .justify_center()
                    .child(card),
            )
    }

    /// Render a footer line for an overlay.
    fn overlay_footer(&self, text: String) -> AnyElement {
        let t = &self.theme;
        div()
            .px(px(12.0))
            .py(px(4.0))
            .text_xs()
            .text_color(t.hint)
            .child(text)
            .into_any_element()
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

            let full_path = result.path().to_string_lossy().to_string();
            let display_path = full_path
                .strip_prefix(&root_prefix)
                .unwrap_or(&full_path)
                .trim_start_matches('/')
                .to_string();

            // Split "13/april/meeting.md" → ("meeting.md", "13/april") so we can render
            // the filename bright and the parent path dim.
            let (mut name_part, parent_part) = match display_path.rfind('/') {
                Some(idx) => (display_path[idx + 1..].to_string(), display_path[..idx].to_string()),
                None => (display_path.clone(), String::new()),
            };
            // Mark directories with a trailing slash so they're distinguishable from
            // similarly-named files. The path comes from the FS cache, so an is_dir()
            // syscall here is cheap (already cached by the kernel from the walk).
            let is_folder = matches!(result, crate::search::FinderResult::File(_))
                && result.path().is_dir();
            if is_folder && !name_part.ends_with('/') {
                name_part.push('/');
            }

            let line_suffix: Option<String> = match result {
                crate::search::FinderResult::File(_) => None,
                crate::search::FinderResult::Content(m) => {
                    let line_preview = m.line_text.trim();
                    let truncated = if line_preview.chars().count() > 60 {
                        let end: String = line_preview.chars().take(60).collect();
                        format!("{}...", end)
                    } else {
                        line_preview.to_string()
                    };
                    Some(format!(":{} — {}", m.line_number, truncated))
                }
            };

            let click_idx = i;
            let mut name_row = div()
                .flex()
                .flex_row()
                .items_baseline()
                .gap(px(8.0))
                .child(
                    div()
                        .text_color(t.fg)
                        .child(name_part),
                );
            if !parent_part.is_empty() {
                name_row = name_row.child(
                    div()
                        .text_color(t.hint)
                        .text_xs()
                        .child(parent_part),
                );
            }
            if let Some(suffix) = line_suffix {
                name_row = name_row.child(
                    div()
                        .text_color(t.hint)
                        .text_xs()
                        .child(suffix),
                );
            }

            list = list.child(
                div()
                    .id(ElementId::NamedInteger("finder-item".into(), i as u64))
                    .w_full()
                    .px(px(12.0))
                    .py(px(4.0))
                    .bg(bg)
                    .text_color(t.fg)
                    .text_sm()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this: &mut Self, _, window, cx| {
                        this.file_finder.selected_index = click_idx;
                        if let Some(source) = this.folder_move_source.take() {
                            if let Some(target_dir) = this.file_finder.selected_path().map(|p| p.to_path_buf()) {
                                this.active_overlay = None;
                                this.file_finder.close();
                                this.move_file_to_dir(source, &target_dir, cx);
                                let focused = this.active_ws().focused_pane;
                                this.focus_pane_editor(focused, window, cx);
                                cx.notify();
                            }
                        } else {
                            this.confirm_finder_selection(window, cx);
                        }
                    }))
                    .child(name_row),
            );
        }

        let footer_text = if self.folder_move_source.is_some() {
            format!("{} folders  |  Enter: select  Esc: cancel", self.file_finder.result_count())
        } else {
            format!("{} results  |  Enter: open  {}+Enter: open in split", self.file_finder.result_count(), if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" })
        };

        self.overlay_shell(
            "finder-dismiss-bg",
            "finder-card",
            500.0,
            &self.file_finder_input,
            list.into_any_element(),
            Some(self.overlay_footer(footer_text)),
            cx,
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
                div().px(px(12.0)).py(px(8.0)).text_sm().text_color(t.accent).child("Searching with Claude..."),
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
                    if m.line > 0 { format!("{}:{}", short, m.line) } else { short }
                };
                let idx = i;
                let row = div()
                    .id(ElementId::NamedInteger("agentic-line".into(), i as u64))
                    .w_full().px(px(12.0)).py(px(4.0))
                    .when(selected, |d| d.bg(t.selection))
                    .rounded(px(4.0)).cursor_pointer()
                    .on_click(cx.listener(move |this: &mut Self, _, window, cx| {
                        this.open_agentic_result(idx, window, cx);
                    }))
                    .child(div().text_sm().text_color(if is_error { t.accent } else { t.fg }).child(display_path))
                    .when(!is_error && !m.quote.is_empty(), |d| {
                        d.child(div().text_xs().text_color(t.hint).child(m.quote.chars().take(120).collect::<String>()))
                    });
                results_div = results_div.child(row);
            }
        }

        let status = if self.agentic_loading { "Running...".into() }
            else if self.agentic_results.is_empty() { "Press Enter to search".into() }
            else { format!("{} results", self.agentic_results.len()) };

        self.overlay_shell("agentic-dismiss-bg", "agentic-card", 600.0, &self.agentic_input,
            results_div.into_any_element(), Some(self.overlay_footer(status)), cx)
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
            body = body.child(div().px(px(12.0)).py(px(6.0)).text_sm().text_color(t.hint).child(rename_label));
        } else {
            let filtered = self.palette.filtered_commands();
            let mut list = div()
                .id("palette-list").flex().flex_col().max_h(px(300.0))
                .overflow_y_scroll().track_scroll(&self.palette_scroll);

            for (i, cmd) in filtered.iter().enumerate() {
                let is_selected = i == self.palette.selected_index;
                let bg = if is_selected { t.selection } else { t.sidebar_bg };
                let action_id = cmd.action_id.clone();
                let mut row = div()
                    .id(ElementId::NamedInteger("palette-item".into(), i as u64))
                    .w_full().px(px(12.0)).py(px(6.0)).flex().flex_row().justify_between()
                    .bg(bg).text_color(t.fg).text_sm().cursor_pointer()
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
                    row = row.child(div().text_color(t.hint).text_xs().child(hint.clone()));
                }
                list = list.child(row);
            }
            body = body.child(list);
        }

        self.overlay_shell("palette-dismiss-bg", "palette-card", 400.0, &self.palette_input,
            body.into_any_element(), None, cx)
    }

    pub(crate) fn render_note_switcher(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let t = &self.theme;
        let mut list = div()
            .id("note-switcher-results").flex().flex_col().max_h(px(400.0))
            .overflow_y_scroll().track_scroll(&self.note_switcher_scroll);

        let max_display = 50.min(self.note_switcher_results.len());
        for i in 0..max_display {
            let result = &self.note_switcher_results[i];
            let is_selected = i == self.note_switcher_selected;
            let bg = if is_selected { t.selection } else { t.sidebar_bg };
            let idx = i;
            let mut row = div()
                .id(ElementId::NamedInteger("note-switch-item".into(), i as u64))
                .w_full().px(px(12.0)).py(px(4.0)).bg(bg).cursor_pointer()
                .on_click(cx.listener(move |this: &mut Self, _, window, cx| {
                    this.note_switcher_selected = idx;
                    this.confirm_note_switcher(window, cx);
                }))
                .child(
                    div().flex().flex_row().items_center().gap(px(8.0))
                        .child(div().text_sm().text_color(t.fg).child(result.filename.clone()))
                        .child(div().text_xs().text_color(t.hint).child(result.ws_title.clone())),
                );
            if let Some(ref snippet) = result.content_snippet {
                if !result.is_title_match {
                    row = row.child(div().text_xs().text_color(t.hint).child(snippet.clone()));
                }
            }
            list = list.child(row);
        }

        let footer = format!("{} open notes", self.note_switcher_results.len());
        self.overlay_shell("note-switcher-dismiss-bg", "note-switcher-card", 500.0,
            &self.note_switcher_input, list.into_any_element(), Some(self.overlay_footer(footer)), cx)
    }

    pub(crate) fn tab_menu_entries(&self) -> Vec<ContextMenuEntry> {
        let t = &self.theme;
        let multi = self.workspaces.len() > 1;
        let mut items = vec![
            ContextMenuEntry { id: "rename", label: "Rename Tab".into(), shortcut: None, color: t.fg },
            ContextMenuEntry { id: "ai-rename", label: "AI: Rename Tab".into(), shortcut: None, color: t.accent },
        ];
        if multi {
            items.push(ContextMenuEntry { id: "ai-rename-all", label: "AI: Rename All Tabs".into(), shortcut: None, color: t.accent });
            items.push(ContextMenuEntry { id: "tearoff", label: "Move to New Window".into(), shortcut: None, color: t.fg });
            items.push(ContextMenuEntry { id: "close-others", label: "Close Other Tabs".into(), shortcut: None, color: t.fg });
        }
        items.push(ContextMenuEntry { id: "close", label: "Close Tab".into(), shortcut: Some("\u{2318}W"), color: t.error });
        items
    }

    pub(crate) fn dispatch_tab_menu(&mut self, ws_idx: usize, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.tab_context_menu = None;
        match id {
            "rename" => { self.switch_workspace(ws_idx, window, cx); self.enter_rename_mode(RenameMode::Tab, window, cx); }
            "ai-rename" => { self.switch_workspace(ws_idx, window, cx); self.ai_rename_tab(cx); }
            "ai-rename-all" => { self.ai_rename_all_tabs(cx); }
            "tearoff" => self.tear_off_tab(ws_idx, window, cx),
            "close-others" => self.close_other_workspaces(ws_idx, window, cx),
            "close" => self.close_workspace(ws_idx, window, cx),
            _ => {}
        }
    }

    pub(crate) fn render_tab_context_menu(&self, ws_idx: usize, position: Point<Pixels>, window: &Window, cx: &mut Context<Self>) -> Div {
        let entries = self.tab_menu_entries();
        let position = clamp_menu_position(position, entries.len(), window);
        self.render_menu_from_entries(&entries, position, "tab-ctx", ws_idx, cx)
    }

    pub(crate) fn tree_menu_entries(&self, path: &Path) -> Vec<ContextMenuEntry> {
        let t = &self.theme;
        let is_file = path.is_file();
        let is_dir = path.is_dir();
        let is_root = *path == self.root;
        let diary_dir = self.root.join("diary");
        let is_diary_path = path.starts_with(&diary_dir);

        let rename_enabled = if is_file { !is_diary_path }
            else if is_dir { !is_root && *path != diary_dir && !is_diary_path }
            else { false };
        let show_rename = rename_enabled || (is_dir && !is_root && (is_diary_path || *path == diary_dir));

        let context_dir = if is_dir { path.to_path_buf() } else { path.parent().unwrap_or(&self.root).to_path_buf() };
        let new_folder_in_diary = context_dir.starts_with(&diary_dir);

        let mut items = Vec::new();
        if show_rename {
            items.push(ContextMenuEntry {
                id: "rename", label: "Rename".into(), shortcut: None,
                color: if rename_enabled { t.fg } else { t.hint },
            });
        }
        items.push(ContextMenuEntry { id: "new-note", label: "New Note".into(), shortcut: Some("\u{2318}N"), color: t.fg });
        if !new_folder_in_diary {
            items.push(ContextMenuEntry { id: "new-folder", label: "New Folder".into(), shortcut: None, color: t.fg });
        }
        if is_file && !is_diary_path {
            items.push(ContextMenuEntry { id: "duplicate", label: "Duplicate".into(), shortcut: None, color: t.fg });
        }
        if is_file {
            items.push(ContextMenuEntry { id: "ai-rename", label: "AI: Rename File".into(), shortcut: None, color: t.accent });
            items.push(ContextMenuEntry { id: "ai-suggest-folder", label: "AI: Suggest Folder".into(), shortcut: None, color: t.accent });
        }
        items.push(ContextMenuEntry { id: "open-finder", label: "Open in Finder".into(), shortcut: None, color: t.fg });
        items.push(ContextMenuEntry { id: "copy-path", label: "Copy Path".into(), shortcut: None, color: t.fg });
        if path.file_name().is_some() {
            items.push(ContextMenuEntry { id: "copy-name", label: "Copy Name".into(), shortcut: None, color: t.fg });
        }
        if !is_root {
            items.push(ContextMenuEntry { id: "trash", label: "Move to Trash".into(), shortcut: Some("\u{2318}\u{232b}"), color: t.error });
        }
        items
    }

    pub(crate) fn dispatch_tree_menu(&mut self, path: &Path, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let path = path.to_path_buf();
        self.tree_context_menu = None;
        let context_dir = if path.is_dir() { path.clone() } else { path.parent().unwrap_or(&self.root).to_path_buf() };
        match id {
            "rename" => {
                self.file_tree.update(cx, |tree, cx| tree.start_rename(&path, window, cx));
            }
            "new-note" => self.new_note_in_dir(context_dir, window, cx),
            "new-folder" => self.create_new_folder(context_dir, window, cx),
            "duplicate" => self.duplicate_file(&path, window, cx),
            "ai-rename" => {
                self.open_file(path, window, cx);
                self.ai_rename_file(cx);
            }
            "ai-suggest-folder" => {
                self.open_file(path, window, cx);
                self.ai_suggest_folder(cx);
            }
            "open-finder" => { std::process::Command::new("open").arg("-R").arg(&path).spawn().ok(); cx.notify(); }
            "copy-path" => { cx.write_to_clipboard(ClipboardItem::new_string(path.display().to_string())); cx.notify(); }
            "copy-name" => {
                if let Some(n) = path.file_name() {
                    cx.write_to_clipboard(ClipboardItem::new_string(n.to_string_lossy().to_string()));
                }
                cx.notify();
            }
            "trash" => self.move_to_trash(path, window, cx),
            _ => {}
        }
    }

    pub(crate) fn render_context_menu(&self, path: &Path, position: Point<Pixels>, window: &Window, cx: &mut Context<Self>) -> Div {
        let entries = self.tree_menu_entries(path);
        let position = clamp_menu_position(position, entries.len(), window);
        let path_owned = path.to_path_buf();
        self.render_menu_from_entries_tree(&entries, position, &path_owned, cx)
    }

    /// Shared renderer for context menu entries (tab menu — dispatches by ws_idx).
    fn render_menu_from_entries(
        &self,
        entries: &[ContextMenuEntry],
        position: Point<Pixels>,
        id_prefix: &'static str,
        ws_idx: usize,
        cx: &mut Context<Self>,
    ) -> Div {
        let t = &self.theme;
        let selected = self.context_menu_selected;
        let mut menu = div()
            .absolute().top(position.y).left(position.x)
            .bg(t.sidebar_bg).border_1().border_color(t.border)
            .rounded(px(4.0)).shadow_lg().min_w(px(180.0)).py(px(4.0))
            .flex().flex_col();
        for (i, entry) in entries.iter().enumerate() {
            let is_sel = i == selected;
            let bg = if is_sel { t.selection } else { t.sidebar_bg };
            let entry_id = entry.id;
            let mut row = div()
                .id(ElementId::NamedInteger(id_prefix.into(), i as u64))
                .px(px(12.0)).py(px(4.0)).flex().flex_row().justify_between()
                .bg(bg).text_sm().text_color(entry.color).cursor_pointer()
                .hover(|s| s.bg(t.selection))
                .on_click(cx.listener(move |this: &mut Self, _, window, cx| {
                    this.dispatch_tab_menu(ws_idx, entry_id, window, cx);
                }))
                .child(entry.label.clone());
            if let Some(hint) = entry.shortcut {
                row = row.child(div().text_xs().text_color(t.hint).pl(px(16.0)).child(hint));
            }
            menu = menu.child(row);
        }
        menu
    }

    /// Shared renderer for context menu entries (tree menu — dispatches by path).
    fn render_menu_from_entries_tree(
        &self,
        entries: &[ContextMenuEntry],
        position: Point<Pixels>,
        path: &Path,
        cx: &mut Context<Self>,
    ) -> Div {
        let t = &self.theme;
        let selected = self.context_menu_selected;
        let mut menu = div()
            .absolute().top(position.y).left(position.x)
            .bg(t.sidebar_bg).border_1().border_color(t.border)
            .rounded(px(4.0)).shadow_lg().min_w(px(180.0)).py(px(4.0))
            .flex().flex_col();
        for (i, entry) in entries.iter().enumerate() {
            let is_sel = i == selected;
            let bg = if is_sel { t.selection } else { t.sidebar_bg };
            let entry_id = entry.id;
            let p = path.to_path_buf();
            let mut row = div()
                .id(ElementId::NamedInteger("tree-ctx".into(), i as u64))
                .px(px(12.0)).py(px(4.0)).flex().flex_row().justify_between()
                .bg(bg).text_sm().text_color(entry.color).cursor_pointer()
                .hover(|s| s.bg(t.selection))
                .on_click(cx.listener(move |this: &mut Self, _, window, cx| {
                    this.dispatch_tree_menu(&p, entry_id, window, cx);
                }))
                .child(entry.label.clone());
            if let Some(hint) = entry.shortcut {
                row = row.child(div().text_xs().text_color(t.hint).pl(px(16.0)).child(hint));
            }
            menu = menu.child(row);
        }
        menu
    }
}
