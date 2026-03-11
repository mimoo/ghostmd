use std::path::PathBuf;
use std::rc::Rc;
use std::time::Instant;

use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState, DefinitionProvider, RopeExt as _};
use ropey::Rope;

use ghostmd_core::note::Note;

/// Detects URLs in text and provides them as "definitions" for Cmd+click.
struct UrlDefinitionProvider;

impl DefinitionProvider for UrlDefinitionProvider {
    fn definitions(
        &self,
        text: &Rope,
        offset: usize,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<Vec<lsp_types::LocationLink>>> {
        let text_str = text.to_string();

        // Find all URLs and check if offset falls within one
        let mut search_start = 0;
        while let Some(start) = text_str[search_start..].find("http://")
            .or_else(|| text_str[search_start..].find("https://"))
        {
            let abs_start = search_start + start;
            // Find end of URL (whitespace or certain delimiters)
            let end = text_str[abs_start..]
                .find(|c: char| c.is_whitespace() || matches!(c, '>' | ')' | ']' | '"' | '\'' | '`'))
                .map(|e| abs_start + e)
                .unwrap_or(text_str.len());

            if offset >= abs_start && offset < end {
                let url = &text_str[abs_start..end];
                let start_pos = text.offset_to_position(abs_start);
                let end_pos = text.offset_to_position(end);

                if let Ok(uri) = url.parse::<lsp_types::Uri>() {
                    return Task::ready(Ok(vec![lsp_types::LocationLink {
                        origin_selection_range: Some(lsp_types::Range {
                            start: start_pos,
                            end: end_pos,
                        }),
                        target_uri: uri,
                        target_range: lsp_types::Range::default(),
                        target_selection_range: lsp_types::Range::default(),
                    }]));
                }
                break;
            }
            search_start = end;
        }
        Task::ready(Ok(vec![]))
    }
}

/// GPUI view wrapping an InputState for editing a single note file.
/// InputState owns the text buffer (rope, undo, clipboard, IME, cursor, selection).
/// EditorView tracks metadata: path, dirty flag, auto-save timing.
pub struct EditorView {
    pub path: PathBuf,
    pub dirty: bool,
    pub needs_reload: bool,
    /// When true, the next Change event from InputState is suppressed (used during reload).
    skip_next_change: bool,
    input_state: Entity<InputState>,
    focus_handle: FocusHandle,
    pub last_edit: Option<Instant>,
    /// Tracks when we last wrote to disk, so the file watcher can ignore our own saves.
    pub last_save: Option<Instant>,
    /// Deferred scroll: line (1-based) + remaining retry attempts.
    pending_scroll: Option<(usize, u8)>,
    /// Flash highlight: start time for a brief border flash after navigating from search.
    pub highlight_start: Option<Instant>,
    /// Whether syntax highlighting is enabled for this editor.
    pub syntax_highlight: bool,
}

impl EditorView {
    pub fn new(
        path: PathBuf,
        syntax_highlight: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input_state = cx.new(|cx| {
            let mut state = if syntax_highlight {
                InputState::new(window, cx)
                    .code_editor("markdown")
                    .line_number(false)
                    .indent_guides(false)
                    .soft_wrap(true)
            } else {
                InputState::new(window, cx)
                    .multi_line(true)
                    .soft_wrap(true)
                    .searchable(true)
            };
            state.lsp.definition_provider = Some(Rc::new(UrlDefinitionProvider));
            state
        });

        // Load existing file content if it exists
        if path.exists() {
            if let Ok((_note, content)) = Note::load(&path) {
                if !content.is_empty() {
                    input_state.update(cx, |state, cx| {
                        state.set_value(content, window, cx);
                    });
                }
            }
        }

        // Subscribe to text changes
        cx.subscribe(&input_state, |this: &mut Self, _entity: Entity<InputState>, event: &InputEvent, _cx: &mut Context<Self>| {
            if matches!(event, InputEvent::Change) {
                if this.skip_next_change {
                    this.skip_next_change = false;
                } else {
                    this.dirty = true;
                    this.last_edit = Some(Instant::now());
                }
            }
        })
        .detach();

        let focus_handle = cx.focus_handle();

        Self {
            path,
            dirty: false,
            needs_reload: false,
            skip_next_change: false,
            input_state,
            focus_handle,
            last_edit: None,
            last_save: None,
            pending_scroll: None,
            highlight_start: None,
            syntax_highlight,
        }
    }

    /// Switch this editor to a different file, reusing the existing InputState
    /// (preserves layout metrics so soft wrap doesn't flicker).
    pub fn load_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        // Save current file first
        self.save(cx).ok();

        self.path = path.clone();
        self.dirty = false;
        self.needs_reload = false;
        self.last_edit = None;
        self.last_save = None;
        self.pending_scroll = None;
        self.highlight_start = None;

        // Load new content, suppressing the Change event
        self.skip_next_change = true;
        let content = if path.exists() {
            Note::load(&path)
                .ok()
                .map(|(_, c)| c)
                .unwrap_or_default()
        } else {
            String::new()
        };
        self.input_state.update(cx, |state, cx| {
            state.set_value(content, window, cx);
        });
    }

    /// Save the current content to disk.
    pub fn save(&mut self, cx: &App) -> anyhow::Result<()> {
        let text = self.input_state.read(cx).value().to_string();
        let note = Note::new(self.path.clone());
        note.ensure_dir()?;
        note.save(&text)?;
        self.dirty = false;
        self.last_save = Some(Instant::now());
        Ok(())
    }

    /// Returns true if dirty and enough time has passed since the last edit.
    pub fn should_auto_save(&self, debounce_ms: u128) -> bool {
        if !self.dirty {
            return false;
        }
        match self.last_edit {
            Some(t) => t.elapsed().as_millis() >= debounce_ms,
            None => false,
        }
    }

    /// Get the current text content.
    pub fn text(&self, cx: &App) -> String {
        self.input_state.read(cx).value().to_string()
    }

    /// Focus this editor's input for typing.
    pub fn focus_input(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.input_state.update(cx, |state, cx| {
            state.focus(window, cx);
        });
    }

    /// Toggle syntax highlighting, recreating the InputState to switch modes.
    pub fn set_syntax_highlight(&mut self, enabled: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.syntax_highlight == enabled {
            return;
        }
        self.syntax_highlight = enabled;
        let content = self.input_state.read(cx).value().to_string();

        let new_input = cx.new(|cx| {
            let mut state = if enabled {
                InputState::new(window, cx)
                    .code_editor("markdown")
                    .line_number(false)
                    .indent_guides(false)
                    .soft_wrap(true)
            } else {
                InputState::new(window, cx)
                    .multi_line(true)
                    .soft_wrap(true)
                    .searchable(true)
            };
            state.lsp.definition_provider = Some(Rc::new(UrlDefinitionProvider));
            state
        });

        if !content.is_empty() {
            self.skip_next_change = true;
            new_input.update(cx, |state, cx| {
                state.set_value(content, window, cx);
            });
        }

        // Re-subscribe to change events
        cx.subscribe(&new_input, |this: &mut Self, _entity: Entity<InputState>, event: &InputEvent, _cx: &mut Context<Self>| {
            if matches!(event, InputEvent::Change) {
                if this.skip_next_change {
                    this.skip_next_change = false;
                } else {
                    this.dirty = true;
                    this.last_edit = Some(Instant::now());
                }
            }
        })
        .detach();

        self.input_state = new_input;
        cx.notify();
    }

    /// Scroll to a specific line (1-based) and place the cursor there.
    /// Retries on subsequent renders to ensure the viewport actually scrolls
    /// (InputState's scroll_to is a no-op until layout has been computed).
    pub fn scroll_to_line(&mut self, line: usize, window: &mut Window, cx: &mut Context<Self>) {
        // Apply immediately (cursor offset is set even if scroll doesn't work yet)
        let line_0 = if line > 0 { line - 1 } else { 0 } as u32;
        self.input_state.update(cx, |state, cx| {
            state.set_cursor_position(
                lsp_types::Position { line: line_0, character: 0 },
                window,
                cx,
            );
        });
        // Schedule retries to ensure scroll works after layout
        self.pending_scroll = Some((line, 3));
    }

    /// Re-apply a pending scroll. Called from render when layout is available.
    fn retry_scroll(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (line, retries) = match self.pending_scroll {
            Some((l, r)) if r > 0 => (l, r),
            _ => { self.pending_scroll = None; return; }
        };
        let line_0 = if line > 0 { line - 1 } else { 0 } as u32;
        self.input_state.update(cx, |state, cx| {
            state.set_cursor_position(
                lsp_types::Position { line: line_0, character: 0 },
                window,
                cx,
            );
        });
        self.pending_scroll = Some((line, retries - 1));
    }

    /// Scroll to a line and briefly flash the editor to indicate the match location.
    pub fn highlight_line(&mut self, line: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.scroll_to_line(line, window, cx);
        self.highlight_start = Some(Instant::now());
    }
}

impl Focusable for EditorView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for EditorView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.needs_reload && !self.dirty {
            self.needs_reload = false;
            if let Ok((_note, content)) = Note::load(&self.path) {
                self.skip_next_change = true;
                self.input_state.update(cx, |state, cx| {
                    state.set_value(content, window, cx);
                });
            }
        }
        // Retry deferred scroll (layout may now be available)
        if self.pending_scroll.is_some() {
            self.retry_scroll(window, cx);
        }

        // Compute highlight flash opacity (fades from 0.3 to 0 over 1.5s)
        let highlight_opacity = if let Some(start) = self.highlight_start {
            let elapsed = start.elapsed().as_secs_f32();
            let duration = 1.5_f32;
            if elapsed >= duration {
                self.highlight_start = None;
                0.0
            } else {
                // Schedule repaint for smooth fade
                cx.spawn(async |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                    cx.background_executor().timer(std::time::Duration::from_millis(30)).await;
                    this.update(cx, |_this, cx| cx.notify()).ok();
                }).detach();
                0.3 * (1.0 - elapsed / duration)
            }
        } else {
            0.0
        };

        let mut container = div()
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .overflow_hidden()
            .track_focus(&self.focus_handle);

        if highlight_opacity > 0.0 {
            container = container
                .bg(hsla(210.0 / 360.0, 0.7, 0.5, highlight_opacity))
                .border_2()
                .border_color(hsla(210.0 / 360.0, 0.8, 0.6, highlight_opacity * 1.5))
                .rounded(px(4.0));
        }

        container
            .capture_any_mouse_down(|event: &MouseDownEvent, _window, cx| {
                if event.button == MouseButton::Right {
                    cx.stop_propagation();
                }
            })
            .child(
                Input::new(&self.input_state)
                    .appearance(false)
                    .h_full(),
            )
    }
}
