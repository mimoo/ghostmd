#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use ghostmd_core::buffer::UndoBuffer;
use ghostmd_core::note::Note;

/// State for a single editor panel (one open note).
pub struct EditorPanel {
    /// Path of the file currently being edited.
    pub path: PathBuf,
    /// The text buffer with undo/redo history.
    pub buffer: UndoBuffer,
    /// Whether the buffer has unsaved changes.
    pub dirty: bool,
    /// Current cursor byte offset in the buffer.
    pub cursor_offset: usize,
    /// Current scroll line offset for the viewport.
    pub scroll_line: usize,
    /// Whether the editor is in focus.
    pub focused: bool,
    /// When the last edit occurred (for auto-save debounce).
    last_edit_time: Option<Instant>,
    /// Whether a save is pending (dirty + debounce elapsed).
    save_pending: bool,
}

impl EditorPanel {
    /// Creates a new editor panel for the given file with an empty buffer.
    pub fn new(path: PathBuf) -> Self {
        EditorPanel {
            path,
            buffer: UndoBuffer::new(),
            dirty: false,
            cursor_offset: 0,
            scroll_line: 0,
            focused: false,
            last_edit_time: None,
            save_pending: false,
        }
    }

    /// Opens a file from disk, loading its content into the buffer.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let (_note, content) = Note::load(path)?;
        let mut panel = Self {
            path: path.to_path_buf(),
            buffer: UndoBuffer::from_str(&content),
            dirty: false,
            cursor_offset: 0,
            scroll_line: 0,
            focused: false,
            last_edit_time: None,
            save_pending: false,
        };
        // Place cursor at the end of the loaded content.
        panel.cursor_offset = content.len();
        Ok(panel)
    }

    /// Inserts text at the current cursor position, advances cursor, marks dirty.
    pub fn insert_at_cursor(&mut self, text: &str) {
        self.buffer.insert(self.cursor_offset, text);
        self.cursor_offset += text.len();
        self.dirty = true;
        self.mark_edited();
    }

    /// Deletes `n` bytes backward from cursor (backspace behavior).
    /// Clamps to position 0 if `n` exceeds the cursor offset.
    pub fn delete_backward(&mut self, n: usize) {
        let delete_count = n.min(self.cursor_offset);
        if delete_count == 0 {
            return;
        }
        let start = self.cursor_offset - delete_count;
        self.buffer.delete(start..self.cursor_offset);
        self.cursor_offset = start;
        self.dirty = true;
        self.mark_edited();
    }

    /// Undoes the last edit. Returns `true` if an undo was performed.
    /// Updates cursor position and dirty state.
    pub fn undo(&mut self) -> bool {
        if !self.buffer.undo() {
            return false;
        }
        // Clamp cursor to the new buffer length.
        let len = self.buffer.text().len();
        if self.cursor_offset > len {
            self.cursor_offset = len;
        }
        // Dirty if the buffer still has undo history (i.e., differs from saved state).
        // A more precise check would track the "saved" history position, but for now
        // we mark dirty whenever there's undo history remaining.
        self.dirty = self.buffer.can_undo();
        true
    }

    /// Redoes the last undone edit. Returns `true` if a redo was performed.
    /// Updates cursor position and marks dirty.
    pub fn redo(&mut self) -> bool {
        if !self.buffer.redo() {
            return false;
        }
        let len = self.buffer.text().len();
        if self.cursor_offset > len {
            self.cursor_offset = len;
        }
        self.dirty = true;
        true
    }

    /// Saves the current buffer content to disk and marks the editor as clean.
    pub fn save(&mut self) -> anyhow::Result<()> {
        let note = Note::new(self.path.clone());
        note.save(&self.buffer.text())?;
        self.dirty = false;
        self.save_pending = false;
        Ok(())
    }

    /// Returns `true` if an auto-save should trigger: the buffer is dirty and
    /// enough time has passed since the last edit (debounce).
    pub fn should_auto_save(&self, debounce_ms: u128) -> bool {
        if !self.dirty {
            return false;
        }
        match self.last_edit_time {
            Some(t) => t.elapsed().as_millis() >= debounce_ms,
            None => false,
        }
    }

    /// Records that an edit just happened (for debounce tracking).
    pub fn mark_edited(&mut self) {
        self.last_edit_time = Some(Instant::now());
        self.save_pending = true;
    }

    /// Returns the full buffer content as a String.
    pub fn text(&self) -> String {
        self.buffer.text()
    }

    /// Moves the cursor to a byte offset, clamped to the buffer length.
    pub fn set_cursor(&mut self, offset: usize) {
        let len = self.buffer.text().len();
        self.cursor_offset = offset.min(len);
    }

    /// Returns the note title derived from the file path.
    pub fn title(&self) -> String {
        Note::title_from_path(&self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    // ---------------------------------------------------------------
    // 1. new() defaults
    // ---------------------------------------------------------------
    #[test]
    fn new_has_empty_buffer_and_defaults() {
        let panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        assert_eq!(panel.text(), "");
        assert!(!panel.dirty);
        assert_eq!(panel.cursor_offset, 0);
        assert_eq!(panel.scroll_line, 0);
        assert!(!panel.focused);
    }

    // ---------------------------------------------------------------
    // 2. open() loads file content
    // ---------------------------------------------------------------
    #[test]
    fn open_loads_file_content() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("note.md");
        std::fs::write(&path, "hello world").unwrap();

        let panel = EditorPanel::open(&path).unwrap();
        assert_eq!(panel.text(), "hello world");
        assert!(!panel.dirty);
        assert_eq!(panel.path, path);
    }

    // ---------------------------------------------------------------
    // 3. open() nonexistent returns error
    // ---------------------------------------------------------------
    #[test]
    fn open_nonexistent_returns_error() {
        let result = EditorPanel::open(Path::new("/nonexistent/path/note.md"));
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // 4. insert_at_cursor: text inserted, cursor advances, dirty=true
    // ---------------------------------------------------------------
    #[test]
    fn insert_at_cursor_basic() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        panel.insert_at_cursor("hello");
        assert_eq!(panel.text(), "hello");
        assert_eq!(panel.cursor_offset, 5);
        assert!(panel.dirty);
    }

    // ---------------------------------------------------------------
    // 5. insert_at_cursor multiple times builds text correctly
    // ---------------------------------------------------------------
    #[test]
    fn insert_at_cursor_multiple_times() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        panel.insert_at_cursor("hello");
        panel.insert_at_cursor(" ");
        panel.insert_at_cursor("world");
        assert_eq!(panel.text(), "hello world");
        assert_eq!(panel.cursor_offset, 11);
    }

    // ---------------------------------------------------------------
    // 6. delete_backward: removes chars, cursor moves back, dirty=true
    // ---------------------------------------------------------------
    #[test]
    fn delete_backward_basic() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        panel.insert_at_cursor("hello");
        panel.dirty = false; // reset to test delete sets dirty

        panel.delete_backward(2);
        assert_eq!(panel.text(), "hel");
        assert_eq!(panel.cursor_offset, 3);
        assert!(panel.dirty);
    }

    // ---------------------------------------------------------------
    // 7. delete_backward at position 0: no-op, no panic
    // ---------------------------------------------------------------
    #[test]
    fn delete_backward_at_zero_is_noop() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        panel.delete_backward(5);
        assert_eq!(panel.text(), "");
        assert_eq!(panel.cursor_offset, 0);
        assert!(!panel.dirty);
    }

    // ---------------------------------------------------------------
    // 8. delete_backward more than available: clamps to 0
    // ---------------------------------------------------------------
    #[test]
    fn delete_backward_clamps_to_zero() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        panel.insert_at_cursor("hi");
        panel.delete_backward(100);
        assert_eq!(panel.text(), "");
        assert_eq!(panel.cursor_offset, 0);
    }

    // ---------------------------------------------------------------
    // 9. undo after insert: text reverts
    // ---------------------------------------------------------------
    #[test]
    fn undo_after_insert_reverts_text() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        panel.insert_at_cursor("hello");
        assert_eq!(panel.text(), "hello");

        let undone = panel.undo();
        assert!(undone);
        assert_eq!(panel.text(), "");
        // After undoing all changes, dirty should be false (no undo history left).
        assert!(!panel.dirty);
    }

    // ---------------------------------------------------------------
    // 10. redo after undo: text restores
    // ---------------------------------------------------------------
    #[test]
    fn redo_after_undo_restores_text() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        panel.insert_at_cursor("hello");
        panel.undo();
        assert_eq!(panel.text(), "");

        let redone = panel.redo();
        assert!(redone);
        assert_eq!(panel.text(), "hello");
        assert!(panel.dirty);
    }

    // ---------------------------------------------------------------
    // 11. save writes to disk
    // ---------------------------------------------------------------
    #[test]
    fn save_writes_to_disk() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("note.md");

        let mut panel = EditorPanel::new(path.clone());
        panel.insert_at_cursor("saved content");
        panel.save().unwrap();

        let on_disk = std::fs::read_to_string(&path).unwrap();
        assert_eq!(on_disk, "saved content");
    }

    // ---------------------------------------------------------------
    // 12. save clears dirty flag
    // ---------------------------------------------------------------
    #[test]
    fn save_clears_dirty_flag() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("note.md");

        let mut panel = EditorPanel::new(path);
        panel.insert_at_cursor("content");
        assert!(panel.dirty);

        panel.save().unwrap();
        assert!(!panel.dirty);
    }

    // ---------------------------------------------------------------
    // 13. should_auto_save: dirty + time elapsed > debounce = true
    // ---------------------------------------------------------------
    #[test]
    fn should_auto_save_after_debounce() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        panel.insert_at_cursor("edit");
        // Wait a little to let debounce elapse.
        thread::sleep(Duration::from_millis(50));
        assert!(panel.should_auto_save(10));
    }

    // ---------------------------------------------------------------
    // 14. should_auto_save when clean: returns false
    // ---------------------------------------------------------------
    #[test]
    fn should_auto_save_when_clean_returns_false() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        panel.insert_at_cursor("edit");
        panel.dirty = false; // simulate save
        thread::sleep(Duration::from_millis(50));
        assert!(!panel.should_auto_save(10));
    }

    // ---------------------------------------------------------------
    // 15. should_auto_save too soon: dirty but time < debounce = false
    // ---------------------------------------------------------------
    #[test]
    fn should_auto_save_too_soon_returns_false() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        panel.insert_at_cursor("edit");
        // Immediately check with a large debounce window.
        assert!(!panel.should_auto_save(60_000));
    }

    // ---------------------------------------------------------------
    // 16. set_cursor clamps to buffer length
    // ---------------------------------------------------------------
    #[test]
    fn set_cursor_clamps_to_buffer_length() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        panel.insert_at_cursor("hello");
        panel.set_cursor(1000);
        assert_eq!(panel.cursor_offset, 5);

        panel.set_cursor(2);
        assert_eq!(panel.cursor_offset, 2);
    }

    // ---------------------------------------------------------------
    // 17. title() extracts from path, stripping timestamp prefix
    // ---------------------------------------------------------------
    #[test]
    fn title_strips_timestamp_prefix() {
        let panel = EditorPanel::new(PathBuf::from("/notes/143022-meeting-notes.md"));
        assert_eq!(panel.title(), "meeting-notes");
    }

    #[test]
    fn title_without_timestamp() {
        let panel = EditorPanel::new(PathBuf::from("/notes/my-note.md"));
        assert_eq!(panel.title(), "my-note");
    }

    // ---------------------------------------------------------------
    // 18. full lifecycle: open -> edit -> save -> edit -> undo -> save
    // ---------------------------------------------------------------
    #[test]
    fn full_lifecycle() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("lifecycle.md");
        std::fs::write(&path, "initial").unwrap();

        // Open
        let mut panel = EditorPanel::open(&path).unwrap();
        assert_eq!(panel.text(), "initial");
        assert!(!panel.dirty);

        // Edit: append " content"
        panel.insert_at_cursor(" content");
        assert_eq!(panel.text(), "initial content");
        assert!(panel.dirty);

        // Save
        panel.save().unwrap();
        assert!(!panel.dirty);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "initial content");

        // Edit again: append " more"
        panel.insert_at_cursor(" more");
        assert_eq!(panel.text(), "initial content more");

        // Undo the last edit
        panel.undo();
        assert_eq!(panel.text(), "initial content");

        // Save again
        panel.save().unwrap();
        assert!(!panel.dirty);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "initial content");
    }

    // ---------------------------------------------------------------
    // Additional edge cases
    // ---------------------------------------------------------------
    #[test]
    fn undo_on_fresh_panel_returns_false() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        assert!(!panel.undo());
    }

    #[test]
    fn redo_without_undo_returns_false() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        assert!(!panel.redo());
    }

    #[test]
    fn insert_at_middle_of_text() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        panel.insert_at_cursor("helo");
        panel.set_cursor(2);
        panel.insert_at_cursor("l");
        assert_eq!(panel.text(), "hello");
        assert_eq!(panel.cursor_offset, 3);
    }

    #[test]
    fn delete_backward_then_insert() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        panel.insert_at_cursor("abcd");
        panel.delete_backward(2);
        assert_eq!(panel.text(), "ab");
        panel.insert_at_cursor("xy");
        assert_eq!(panel.text(), "abxy");
        assert_eq!(panel.cursor_offset, 4);
    }

    #[test]
    fn open_sets_cursor_at_end() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("note.md");
        std::fs::write(&path, "hello").unwrap();

        let panel = EditorPanel::open(&path).unwrap();
        assert_eq!(panel.cursor_offset, 5);
    }

    #[test]
    fn undo_clamps_cursor() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        panel.insert_at_cursor("abcdef");
        // cursor is at 6
        assert_eq!(panel.cursor_offset, 6);

        // Undo removes "abcdef" -> empty string, cursor must clamp to 0
        panel.undo();
        assert_eq!(panel.cursor_offset, 0);
    }

    #[test]
    fn set_cursor_at_zero() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        panel.insert_at_cursor("hello");
        panel.set_cursor(0);
        assert_eq!(panel.cursor_offset, 0);
    }

    #[test]
    fn set_cursor_exact_length() {
        let mut panel = EditorPanel::new(PathBuf::from("/tmp/test.md"));
        panel.insert_at_cursor("hello");
        panel.set_cursor(5);
        assert_eq!(panel.cursor_offset, 5);
    }
}
