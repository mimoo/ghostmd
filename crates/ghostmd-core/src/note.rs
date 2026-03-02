use anyhow::Result;
use std::path::{Path, PathBuf};

/// Represents a single note file on disk.
pub struct Note {
    pub path: PathBuf,
    pub title: String,
    pub modified: bool,
}

impl Note {
    /// Creates a new `Note` with the given path, deriving the title from it.
    pub fn new(path: PathBuf) -> Self {
        let title = Self::title_from_path(&path);
        Self {
            path,
            title,
            modified: false,
        }
    }

    /// Loads a note from disk, returning the `Note` metadata and its content.
    pub fn load(path: &Path) -> Result<(Self, String)> {
        let content = std::fs::read_to_string(path)?;
        let note = Self::new(path.to_path_buf());
        Ok((note, content))
    }

    /// Saves the given content to the note's path.
    pub fn save(&self, content: &str) -> Result<()> {
        std::fs::write(&self.path, content)?;
        Ok(())
    }

    /// Ensures the parent directory of this note's path exists.
    pub fn ensure_dir(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    /// Derives a human-readable title from a file path.
    ///
    /// Strips the extension and any leading timestamp prefix (e.g. `20240101-`).
    pub fn title_from_path(path: &Path) -> String {
        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        // Strip leading digits followed by a dash
        if let Some(pos) = stem.find('-') {
            if stem[..pos].chars().all(|c| c.is_ascii_digit()) && !stem[..pos].is_empty() {
                return stem[pos + 1..].to_string();
            }
        }
        stem
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.md");

        let note = Note::new(path.clone());
        note.save("# Hello\nWorld").unwrap();

        let (loaded, content) = Note::load(&path).unwrap();
        assert_eq!(content, "# Hello\nWorld");
        assert_eq!(loaded.path, path);
    }

    #[test]
    fn ensure_dir_creates_directories() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("a").join("b").join("note.md");

        let note = Note::new(nested.clone());
        note.ensure_dir().unwrap();

        assert!(nested.parent().unwrap().exists());
    }

    #[test]
    fn title_from_path_strips_extension() {
        let path = Path::new("/notes/my-note.md");
        assert_eq!(Note::title_from_path(path), "my-note");
    }

    #[test]
    fn title_from_path_strips_timestamp_prefix() {
        let path = Path::new("/notes/20240315-meeting-notes.md");
        assert_eq!(Note::title_from_path(path), "meeting-notes");
    }

    #[test]
    fn title_from_path_no_timestamp_no_ext() {
        let path = Path::new("/notes/readme");
        assert_eq!(Note::title_from_path(path), "readme");
    }

    #[test]
    fn load_nonexistent_returns_error() {
        let result = Note::load(Path::new("/nonexistent/path/note.md"));
        assert!(result.is_err());
    }

    #[test]
    fn save_creates_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("created.md");

        let note = Note::new(path.clone());
        note.save("content").unwrap();

        assert!(path.exists());
    }

    #[test]
    fn new_note_is_not_modified() {
        let note = Note::new(PathBuf::from("/tmp/test.md"));
        assert!(!note.modified);
    }

    #[test]
    fn save_overwrites_existing_content() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("overwrite.md");

        let note = Note::new(path.clone());
        note.save("first content").unwrap();
        note.save("second content").unwrap();

        let (_, content) = Note::load(&path).unwrap();
        assert_eq!(content, "second content");
    }

    #[test]
    fn path_with_spaces_and_special_chars() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("my notes (draft) [v2].md");

        let note = Note::new(path.clone());
        note.save("special path content").unwrap();

        let (loaded, content) = Note::load(&path).unwrap();
        assert_eq!(content, "special path content");
        assert_eq!(loaded.path, path);
    }

    #[test]
    fn empty_content_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("empty.md");

        let note = Note::new(path.clone());
        note.save("").unwrap();

        let (_, content) = Note::load(&path).unwrap();
        assert_eq!(content, "");
    }

    #[test]
    fn title_from_path_strips_hhmmss_timestamp_prefix() {
        let path = Path::new("/notes/143022-meeting-notes.md");
        assert_eq!(Note::title_from_path(path), "meeting-notes");
    }
}
