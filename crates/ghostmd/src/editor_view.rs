use std::path::PathBuf;
use std::time::Instant;

use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};

use ghostmd_core::note::Note;

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
            InputState::new(window, cx)
                .code_editor("markdown")
                .soft_wrap(true)
                .line_number(false)
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

    /// Get a display title from the file path.
    #[allow(dead_code)]
    pub fn title(&self) -> String {
        Note::title_from_path(&self.path)
    }

    /// Get the text content.
    #[allow(dead_code)]
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
            .child(
                Input::new(&self.input_state)
                    .appearance(false)
                    .h_full(),
            )
    }
}
