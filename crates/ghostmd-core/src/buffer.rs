use ropey::{LineType, Rope};
use std::ops::Range;
use std::time::Instant;
use undo::{Action, History, Merged};

/// A single text edit operation that can be undone/redone.
pub struct TextEdit {
    /// Byte range in the old text that is being replaced.
    pub range: Range<usize>,
    /// The text that was originally in `range`.
    pub old_text: String,
    /// The replacement text.
    pub new_text: String,
    /// When this edit was created.
    pub timestamp: Instant,
}

impl Action for TextEdit {
    type Target = Rope;
    type Error = anyhow::Error;

    fn apply(&mut self, target: &mut Rope) -> undo::Result<Self> {
        target.remove(self.range.clone());
        target.insert(self.range.start, &self.new_text);
        Ok(())
    }

    fn undo(&mut self, target: &mut Rope) -> undo::Result<Self> {
        let new_text_end = self.range.start + self.new_text.len();
        target.remove(self.range.start..new_text_end);
        target.insert(self.range.start, &self.old_text);
        Ok(())
    }

    fn merge(&mut self, _other: &mut Self) -> Merged {
        Merged::No
    }
}

/// A text buffer backed by a rope with full undo/redo history.
pub struct UndoBuffer {
    rope: Rope,
    history: History<TextEdit>,
}

impl UndoBuffer {
    /// Creates a new empty buffer.
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            history: History::new(),
        }
    }

    /// Creates a buffer pre-filled with the given text.
    pub fn from_str(s: &str) -> Self {
        Self {
            rope: Rope::from_str(s),
            history: History::new(),
        }
    }

    /// Applies a pre-built TextEdit to the buffer.
    pub fn apply_edit(&mut self, edit: TextEdit) {
        self.history.apply(&mut self.rope, edit).unwrap();
    }

    /// Convenience: insert `text` at byte position `pos`.
    pub fn insert(&mut self, pos: usize, text: &str) {
        let edit = TextEdit {
            range: pos..pos,
            old_text: String::new(),
            new_text: text.to_string(),
            timestamp: Instant::now(),
        };
        self.apply_edit(edit);
    }

    /// Convenience: delete the byte range from the buffer.
    pub fn delete(&mut self, range: Range<usize>) {
        let old_text = self.rope.slice(range.clone()).to_string();
        let edit = TextEdit {
            range: range.clone(),
            old_text,
            new_text: String::new(),
            timestamp: Instant::now(),
        };
        self.apply_edit(edit);
    }

    /// Undo the last edit. Returns `true` if an undo was performed.
    pub fn undo(&mut self) -> bool {
        if !self.history.can_undo() {
            return false;
        }
        self.history.undo(&mut self.rope).is_ok()
    }

    /// Redo the last undone edit. Returns `true` if a redo was performed.
    pub fn redo(&mut self) -> bool {
        if !self.history.can_redo() {
            return false;
        }
        self.history.redo(&mut self.rope).is_ok()
    }

    /// Whether there is an edit to undo.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Whether there is an edit to redo.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Returns the full buffer content as a `String`.
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// Returns the number of characters in the buffer.
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Returns the number of lines in the buffer.
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines(LineType::LF)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_buffer_is_empty() {
        let buf = UndoBuffer::new();
        assert_eq!(buf.text(), "");
        assert_eq!(buf.len_chars(), 0);
    }

    #[test]
    fn from_str_has_correct_content() {
        let buf = UndoBuffer::from_str("hello world");
        assert_eq!(buf.text(), "hello world");
        assert_eq!(buf.len_chars(), 11);
    }

    #[test]
    fn from_str_line_count() {
        let buf = UndoBuffer::from_str("line1\nline2\nline3");
        assert_eq!(buf.len_lines(), 3);
    }

    #[test]
    fn insert_at_beginning() {
        let mut buf = UndoBuffer::from_str("world");
        buf.insert(0, "hello ");
        assert_eq!(buf.text(), "hello world");
    }

    #[test]
    fn insert_at_end() {
        let mut buf = UndoBuffer::from_str("hello");
        buf.insert(5, " world");
        assert_eq!(buf.text(), "hello world");
    }

    #[test]
    fn insert_in_middle() {
        let mut buf = UndoBuffer::from_str("helo");
        buf.insert(2, "l");
        assert_eq!(buf.text(), "hello");
    }

    #[test]
    fn delete_range() {
        let mut buf = UndoBuffer::from_str("hello world");
        buf.delete(5..11);
        assert_eq!(buf.text(), "hello");
    }

    #[test]
    fn undo_reverses_last_edit() {
        let mut buf = UndoBuffer::from_str("hello");
        buf.insert(5, " world");
        assert_eq!(buf.text(), "hello world");

        let did_undo = buf.undo();
        assert!(did_undo);
        assert_eq!(buf.text(), "hello");
    }

    #[test]
    fn redo_reapplies_undone_edit() {
        let mut buf = UndoBuffer::from_str("hello");
        buf.insert(5, " world");
        buf.undo();
        assert_eq!(buf.text(), "hello");

        let did_redo = buf.redo();
        assert!(did_redo);
        assert_eq!(buf.text(), "hello world");
    }

    #[test]
    fn can_undo_and_can_redo_states() {
        let mut buf = UndoBuffer::new();
        assert!(!buf.can_undo());
        assert!(!buf.can_redo());

        buf.insert(0, "a");
        assert!(buf.can_undo());
        assert!(!buf.can_redo());

        buf.undo();
        assert!(!buf.can_undo());
        assert!(buf.can_redo());
    }

    #[test]
    fn branching_undo_then_new_edit_loses_redo() {
        let mut buf = UndoBuffer::from_str("a");
        buf.insert(1, "b");
        buf.insert(2, "c");
        assert_eq!(buf.text(), "abc");

        buf.undo();
        assert_eq!(buf.text(), "ab");

        // Insert something new, branching the history
        buf.insert(2, "x");
        assert_eq!(buf.text(), "abx");

        // Cannot redo the old "c" edit on this branch
        assert!(!buf.can_redo());
    }

    #[test]
    fn text_returns_current_state_after_multiple_edits() {
        let mut buf = UndoBuffer::new();
        buf.insert(0, "foo");
        buf.insert(3, " bar");
        buf.delete(0..4);
        assert_eq!(buf.text(), "bar");
    }

    #[test]
    fn undo_on_empty_returns_false() {
        let mut buf = UndoBuffer::new();
        assert!(!buf.undo());
    }

    #[test]
    fn redo_without_undo_returns_false() {
        let mut buf = UndoBuffer::from_str("hello");
        assert!(!buf.redo());
    }

    #[test]
    fn multiple_undos() {
        let mut buf = UndoBuffer::new();
        buf.insert(0, "a");
        buf.insert(1, "b");
        buf.insert(2, "c");
        assert_eq!(buf.text(), "abc");

        assert!(buf.undo());
        assert_eq!(buf.text(), "ab");
        assert!(buf.undo());
        assert_eq!(buf.text(), "a");
        assert!(buf.undo());
        assert_eq!(buf.text(), "");
        assert!(!buf.undo());
    }

    #[test]
    fn insert_unicode_multibyte_chars() {
        let mut buf = UndoBuffer::new();
        buf.insert(0, "café");
        assert_eq!(buf.text(), "café");
        assert_eq!(buf.len_chars(), 4);
    }

    #[test]
    fn insert_unicode_emoji_and_cjk() {
        let mut buf = UndoBuffer::new();
        buf.insert(0, "hello 🌍 世界");
        assert_eq!(buf.text(), "hello 🌍 世界");
    }

    #[test]
    fn delete_across_unicode_byte_boundaries() {
        let mut buf = UndoBuffer::from_str("café");
        // "é" is 2 bytes in UTF-8; delete "fé" (bytes 2..5)
        buf.delete(2..5);
        assert_eq!(buf.text(), "ca");
    }

    #[test]
    fn insert_empty_string_is_noop() {
        let mut buf = UndoBuffer::from_str("hello");
        buf.insert(3, "");
        assert_eq!(buf.text(), "hello");
    }

    #[test]
    fn large_buffer_insert_and_delete() {
        let big = "x".repeat(1_000_000);
        let mut buf = UndoBuffer::from_str(&big);
        assert_eq!(buf.len_chars(), 1_000_000);

        buf.insert(500_000, "INSERTED");
        assert_eq!(buf.len_chars(), 1_000_008);

        buf.delete(500_000..500_008);
        assert_eq!(buf.len_chars(), 1_000_000);
        assert_eq!(buf.text(), big);
    }

    #[test]
    fn multiple_undo_then_multiple_redo() {
        let mut buf = UndoBuffer::new();
        buf.insert(0, "a");
        buf.insert(1, "b");
        buf.insert(2, "c");
        buf.insert(3, "d");
        assert_eq!(buf.text(), "abcd");

        // Undo two of four edits
        assert!(buf.undo());
        assert!(buf.undo());
        assert_eq!(buf.text(), "ab");

        // Redo both
        assert!(buf.redo());
        assert_eq!(buf.text(), "abc");
        assert!(buf.redo());
        assert_eq!(buf.text(), "abcd");
    }

    #[test]
    fn undo_all_redo_all_restores_original() {
        let mut buf = UndoBuffer::new();
        buf.insert(0, "one");
        buf.insert(3, " two");
        buf.insert(7, " three");
        let final_text = buf.text();
        assert_eq!(final_text, "one two three");

        // Undo everything
        while buf.can_undo() {
            buf.undo();
        }
        assert_eq!(buf.text(), "");

        // Redo everything
        while buf.can_redo() {
            buf.redo();
        }
        assert_eq!(buf.text(), final_text);
    }
}
