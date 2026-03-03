#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// An AI-generated suggestion for organizing or improving a note.
#[derive(Debug, Serialize, Deserialize)]
pub struct Suggestion {
    pub note_path: PathBuf,
    pub suggested_title: Option<String>,
    pub suggested_location: Option<PathBuf>,
    pub reasoning: String,
    pub timestamp: String,
}

/// Manages interactions with the Claude CLI for AI-assisted note-taking features.
pub struct AiManager {
    root: PathBuf,
    suggestions_dir: PathBuf,
    claude_md_path: PathBuf,
}

impl AiManager {
    /// Creates a new AiManager rooted at the given vault directory.
    /// Suggestions are stored in `<root>/.ghostmd/suggestions/`.
    /// The CLAUDE.md file is at `<root>/CLAUDE.md`.
    pub fn new(root: PathBuf) -> Self {
        let suggestions_dir = root.join(".ghostmd").join("suggestions");
        let claude_md_path = root.join("CLAUDE.md");
        AiManager {
            root,
            suggestions_dir,
            claude_md_path,
        }
    }

    /// Returns the root directory this manager operates on.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the directory where suggestions are stored.
    pub fn suggestions_dir(&self) -> &Path {
        &self.suggestions_dir
    }

    /// Returns the path to the CLAUDE.md file.
    pub fn claude_md_path(&self) -> &Path {
        &self.claude_md_path
    }

    /// Checks whether the `claude` CLI is available on PATH.
    pub fn is_available() -> bool {
        which::which("claude").is_ok()
    }

    /// Generates the content for a CLAUDE.md file tailored to the vault structure.
    pub fn generate_claude_md_content(root: &Path) -> Result<String> {
        let root_display = root.display();
        let content = format!(
            "# GhostMD Vault\n\
             \n\
             This is a GhostMD note vault rooted at `{root_display}`.\n\
             \n\
             ## Structure\n\
             \n\
             - `diary/` - Daily journal entries organized by date (YYYY/MM/DD)\n\
             - `notes/` - General notes\n\
             - `.ghostmd/` - Application metadata (do not edit manually)\n\
             - `.ghostmd/suggestions/` - AI-generated suggestions\n"
        );
        Ok(content)
    }

    /// Persists a suggestion as a JSON file under the suggestions directory.
    /// Creates the directory if it does not exist.
    pub fn write_suggestion(&self, suggestion: &Suggestion) -> Result<()> {
        std::fs::create_dir_all(&self.suggestions_dir)?;
        let filename = format!("{}.json", suggestion.timestamp.replace(':', "-"));
        let path = self.suggestions_dir.join(filename);
        let json = serde_json::to_string_pretty(suggestion)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Loads a suggestion from a JSON file on disk.
    pub fn load_suggestion(path: &Path) -> Result<Suggestion> {
        let content = std::fs::read_to_string(path)?;
        let suggestion: Suggestion = serde_json::from_str(&content)?;
        Ok(suggestion)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn new_sets_paths_correctly() {
        let root = PathBuf::from("/tmp/vault");
        let mgr = AiManager::new(root.clone());
        assert_eq!(mgr.root(), Path::new("/tmp/vault"));
        assert_eq!(
            mgr.suggestions_dir(),
            Path::new("/tmp/vault/.ghostmd/suggestions")
        );
        assert_eq!(mgr.claude_md_path(), Path::new("/tmp/vault/CLAUDE.md"));
    }

    #[test]
    fn write_and_load_suggestion_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let mgr = AiManager::new(tmp.path().to_path_buf());

        let suggestion = Suggestion {
            note_path: PathBuf::from("notes/test.md"),
            suggested_title: Some("Better Title".to_string()),
            suggested_location: Some(PathBuf::from("notes/organized/test.md")),
            reasoning: "The note is about testing.".to_string(),
            timestamp: "2024-03-15T10-30-00".to_string(),
        };

        mgr.write_suggestion(&suggestion).unwrap();

        // Find the written file
        let entries: Vec<_> = std::fs::read_dir(mgr.suggestions_dir())
            .unwrap()
            .collect();
        assert_eq!(entries.len(), 1);

        let file_path = entries[0].as_ref().unwrap().path();
        let loaded = AiManager::load_suggestion(&file_path).unwrap();

        assert_eq!(loaded.note_path, suggestion.note_path);
        assert_eq!(loaded.suggested_title, suggestion.suggested_title);
        assert_eq!(loaded.suggested_location, suggestion.suggested_location);
        assert_eq!(loaded.reasoning, suggestion.reasoning);
        assert_eq!(loaded.timestamp, suggestion.timestamp);
    }

    #[test]
    fn write_suggestion_creates_directory() {
        let tmp = TempDir::new().unwrap();
        let mgr = AiManager::new(tmp.path().to_path_buf());

        assert!(!mgr.suggestions_dir().exists());

        let suggestion = Suggestion {
            note_path: PathBuf::from("test.md"),
            suggested_title: None,
            suggested_location: None,
            reasoning: "test".to_string(),
            timestamp: "2024-01-01T00-00-00".to_string(),
        };

        mgr.write_suggestion(&suggestion).unwrap();
        assert!(mgr.suggestions_dir().exists());
    }

    #[test]
    fn generate_claude_md_content_format() {
        let root = Path::new("/home/user/vault");
        let content = AiManager::generate_claude_md_content(root).unwrap();

        assert!(content.contains("# GhostMD Vault"));
        assert!(content.contains("/home/user/vault"));
        assert!(content.contains("diary/"));
        assert!(content.contains("notes/"));
        assert!(content.contains(".ghostmd/suggestions/"));
    }

    #[test]
    fn load_suggestion_nonexistent_returns_error() {
        let result = AiManager::load_suggestion(Path::new("/nonexistent/suggestion.json"));
        assert!(result.is_err());
    }

    #[test]
    fn suggestion_json_serialization_matches_schema() {
        let suggestion = Suggestion {
            note_path: PathBuf::from("notes/test.md"),
            suggested_title: Some("My Title".to_string()),
            suggested_location: Some(PathBuf::from("notes/organized/")),
            reasoning: "Relevant reasoning".to_string(),
            timestamp: "2024-06-01T12-00-00".to_string(),
        };
        let json = serde_json::to_string(&suggestion).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let obj = value.as_object().unwrap();

        assert!(obj.contains_key("note_path"));
        assert!(obj.contains_key("suggested_title"));
        assert!(obj.contains_key("suggested_location"));
        assert!(obj.contains_key("reasoning"));
        assert!(obj.contains_key("timestamp"));
        assert_eq!(obj.len(), 5);
    }

    #[test]
    fn generate_claude_md_content_empty_directory() {
        let tmp = TempDir::new().unwrap();
        // Empty directory should still produce valid content
        let content = AiManager::generate_claude_md_content(tmp.path()).unwrap();
        assert!(content.contains("# GhostMD Vault"));
        assert!(content.contains(&tmp.path().display().to_string()));
        assert!(content.contains("## Structure"));
    }

    #[test]
    fn generate_claude_md_content_nested_structure_shows_hierarchy() {
        let tmp = TempDir::new().unwrap();
        // Create a nested directory structure
        std::fs::create_dir_all(tmp.path().join("notes/sub")).unwrap();
        std::fs::create_dir_all(tmp.path().join("diary/2024/01")).unwrap();
        std::fs::write(tmp.path().join("notes/sub/deep.md"), "content").unwrap();

        let content = AiManager::generate_claude_md_content(tmp.path()).unwrap();
        // Should still reference the standard structure sections
        assert!(content.contains("diary/"));
        assert!(content.contains("notes/"));
        assert!(content.contains(".ghostmd/"));
        assert!(content.contains(".ghostmd/suggestions/"));
    }

    #[test]
    fn write_suggestion_creates_suggestions_directory() {
        let tmp = TempDir::new().unwrap();
        let mgr = AiManager::new(tmp.path().to_path_buf());

        // .ghostmd/suggestions/ should not exist yet
        assert!(!mgr.suggestions_dir().exists());

        let suggestion = Suggestion {
            note_path: PathBuf::from("test.md"),
            suggested_title: None,
            suggested_location: None,
            reasoning: "auto-test".to_string(),
            timestamp: "2024-07-01T08-00-00".to_string(),
        };

        mgr.write_suggestion(&suggestion).unwrap();

        // Both .ghostmd/ and .ghostmd/suggestions/ should now exist
        assert!(tmp.path().join(".ghostmd").exists());
        assert!(mgr.suggestions_dir().exists());
    }
}
