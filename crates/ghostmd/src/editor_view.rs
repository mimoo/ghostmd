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
    input_state: Entity<InputState>,
    focus_handle: FocusHandle,
    last_edit: Option<Instant>,
}

impl EditorView {
    pub fn new(
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .code_editor("markdown")
                .soft_wrap(true)
                .line_number(false);
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
                this.dirty = true;
                this.last_edit = Some(Instant::now());
            }
        })
        .detach();

        let focus_handle = cx.focus_handle();

        Self {
            path,
            dirty: false,
            input_state,
            focus_handle,
            last_edit: None,
        }
    }

    /// Save the current content to disk.
    pub fn save(&mut self, cx: &App) -> anyhow::Result<()> {
        let text = self.input_state.read(cx).value().to_string();
        let note = Note::new(self.path.clone());
        note.ensure_dir()?;
        note.save(&text)?;
        self.dirty = false;
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
}

impl Focusable for EditorView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for EditorView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .track_focus(&self.focus_handle)
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
