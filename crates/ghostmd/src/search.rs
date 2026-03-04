use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ghostmd_core::search::{ContentMatch, ContentSearch, FuzzySearch, SearchResult};

/// Escape regex special characters for literal string matching.
fn regex_escape(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for c in s.chars() {
        if r"\.+*?()|[]{}^$".contains(c) {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

/// A unified result shown in the file finder.
#[derive(Debug, Clone)]
pub enum FinderResult {
    /// Filename match (fuzzy).
    File(SearchResult),
    /// Content match (ripgrep).
    Content(ContentMatch),
}

impl FinderResult {
    pub fn path(&self) -> &Path {
        match self {
            FinderResult::File(r) => &r.path,
            FinderResult::Content(m) => &m.path,
        }
    }
}

/// State for the file-finder overlay (Cmd+P fuzzy file + content search).
pub struct FileFinder {
    /// Whether the finder is currently visible.
    pub visible: bool,
    /// The current search query.
    pub query: String,
    /// Index of the currently highlighted result.
    pub selected_index: usize,
    /// Current search results (filename matches + content matches).
    pub results: Vec<FinderResult>,
    /// Underlying fuzzy search engine.
    fuzzy: FuzzySearch,
    /// Underlying content search engine.
    content: ContentSearch,
    /// Open files to prioritize (most recently edited first).
    open_files: Vec<PathBuf>,
}

impl FileFinder {
    /// Creates a new file finder for the given root directory.
    pub fn new(root: PathBuf) -> Self {
        FileFinder {
            visible: false,
            query: String::new(),
            selected_index: 0,
            results: Vec::new(),
            fuzzy: FuzzySearch::new(root.clone()),
            content: ContentSearch::new(root),
            open_files: Vec::new(),
        }
    }

    /// Set the list of open files (most recently edited first) for prioritization.
    pub fn set_open_files(&mut self, files: Vec<PathBuf>) {
        self.open_files = files;
    }

    /// Open the finder, reset query and results, refresh file cache.
    pub fn open(&mut self) -> anyhow::Result<()> {
        self.visible = true;
        self.query.clear();
        self.selected_index = 0;
        self.fuzzy.refresh_cache()?;
        let all_files = self.fuzzy.search_files("");
        self.results = self.prioritize(all_files.into_iter().map(FinderResult::File).collect());
        Ok(())
    }

    /// Close the finder.
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// Update the query and re-run fuzzy + content search.
    pub fn set_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.selected_index = 0;

        // Fuzzy filename matches first
        let file_results = self.fuzzy.search_files(query);
        let mut seen_paths: HashSet<PathBuf> = HashSet::new();
        let mut merged: Vec<FinderResult> = Vec::new();

        for r in file_results {
            seen_paths.insert(r.path.clone());
            merged.push(FinderResult::File(r));
        }

        // Content search (only for non-empty queries, >=2 chars to avoid noise)
        if query.len() >= 2 {
            // Escape regex special characters for literal search
            let escaped = regex_escape(query);
            if let Ok(content_matches) = self.content.search(&escaped) {
                for m in content_matches {
                    if !seen_paths.contains(&m.path) {
                        seen_paths.insert(m.path.clone());
                    }
                    merged.push(FinderResult::Content(m));
                }
            }
        }

        self.results = self.prioritize(merged);
    }

    /// Reorder results so open files come first (in open_files order), rest after.
    fn prioritize(&self, results: Vec<FinderResult>) -> Vec<FinderResult> {
        if self.open_files.is_empty() {
            return results;
        }
        let open_set: HashSet<&PathBuf> = self.open_files.iter().collect();
        let mut open_results: Vec<FinderResult> = Vec::new();
        let mut rest: Vec<FinderResult> = Vec::new();
        let mut seen_open: HashSet<PathBuf> = HashSet::new();

        // First pass: pull out open file matches, preserving open_files order
        for result in &results {
            let p = result.path().to_path_buf();
            if open_set.contains(&p) && !seen_open.contains(&p) {
                seen_open.insert(p);
            }
        }

        // Add open files in their priority order
        for open_path in &self.open_files {
            if seen_open.contains(open_path) {
                // Find the matching result
                if let Some(r) = results.iter().find(|r| r.path() == open_path.as_path()) {
                    open_results.push(r.clone());
                }
            }
        }

        // Add remaining results
        for result in results {
            if !open_set.contains(&result.path().to_path_buf()) {
                rest.push(result);
            }
        }

        open_results.extend(rest);
        open_results
    }

    /// Move selection down (wraps around).
    pub fn select_next(&mut self) {
        if !self.results.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.results.len();
        }
    }

    /// Move selection up (wraps around).
    pub fn select_prev(&mut self) {
        if !self.results.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.results.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    /// Get the path of the currently selected result.
    pub fn selected_path(&self) -> Option<&Path> {
        self.results.get(self.selected_index).map(|r| r.path())
    }

    /// Get current result count.
    pub fn result_count(&self) -> usize {
        self.results.len()
    }
}

/// State for the content search panel (Cmd+Shift+F grep-like search).
#[allow(dead_code)]
pub struct ContentSearchPanel {
    /// Whether the panel is currently visible.
    pub visible: bool,
    /// The current search query.
    pub query: String,
    /// Index of the currently highlighted result.
    pub selected_index: usize,
    /// Current search results.
    pub results: Vec<ContentMatch>,
    /// Underlying content search engine.
    searcher: ContentSearch,
}

#[allow(dead_code)]
impl ContentSearchPanel {
    /// Creates a new content search panel for the given root.
    pub fn new(root: PathBuf) -> Self {
        ContentSearchPanel {
            visible: false,
            query: String::new(),
            selected_index: 0,
            results: Vec::new(),
            searcher: ContentSearch::new(root),
        }
    }

    /// Opens the search panel and resets state.
    pub fn open(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected_index = 0;
        self.results.clear();
    }

    /// Closes the search panel.
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// Update query and re-run content search.
    pub fn set_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.selected_index = 0;
        if query.is_empty() {
            self.results.clear();
        } else {
            self.results = self.searcher.search(query).unwrap_or_default();
        }
    }

    /// Move selection down (wraps around).
    pub fn select_next(&mut self) {
        if !self.results.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.results.len();
        }
    }

    /// Move selection up (wraps around).
    pub fn select_prev(&mut self) {
        if !self.results.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.results.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    /// Get the selected match (path + line number).
    pub fn selected_match(&self) -> Option<&ContentMatch> {
        self.results.get(self.selected_index)
    }

    /// Get current result count.
    pub fn result_count(&self) -> usize {
        self.results.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_files(tmp: &TempDir) {
        let root = tmp.path();
        fs::create_dir_all(root.join("notes")).unwrap();
        fs::write(root.join("notes/meeting.md"), "Meeting notes\nAction items").unwrap();
        fs::write(root.join("notes/todo.md"), "Buy groceries\nClean house").unwrap();
        fs::write(root.join("readme.md"), "Project readme\nNothing special").unwrap();
    }

    // ── FileFinder tests ──────────────────────────────────────────────

    #[test]
    fn file_finder_new_defaults() {
        let tmp = TempDir::new().unwrap();
        let finder = FileFinder::new(tmp.path().to_path_buf());

        assert!(!finder.visible);
        assert!(finder.query.is_empty());
        assert_eq!(finder.selected_index, 0);
        assert!(finder.results.is_empty());
    }

    #[test]
    fn file_finder_open_sets_visible_clears_query_refreshes_cache() {
        let tmp = TempDir::new().unwrap();
        create_test_files(&tmp);
        let mut finder = FileFinder::new(tmp.path().to_path_buf());

        // Set some state before opening
        finder.query = "old query".to_string();
        finder.selected_index = 5;

        finder.open().unwrap();

        assert!(finder.visible);
        assert!(finder.query.is_empty());
        assert_eq!(finder.selected_index, 0);
        // After open, results should be populated (empty query returns all files)
        assert!(!finder.results.is_empty());
    }

    #[test]
    fn file_finder_close_sets_not_visible() {
        let tmp = TempDir::new().unwrap();
        let mut finder = FileFinder::new(tmp.path().to_path_buf());
        finder.open().unwrap();
        assert!(finder.visible);

        finder.close();
        assert!(!finder.visible);
    }

    #[test]
    fn file_finder_set_query_updates_results() {
        let tmp = TempDir::new().unwrap();
        create_test_files(&tmp);
        let mut finder = FileFinder::new(tmp.path().to_path_buf());
        finder.open().unwrap();

        finder.set_query("meeting");
        assert!(!finder.results.is_empty());
        assert!(finder
            .results
            .iter()
            .any(|r| r.path().to_string_lossy().contains("meeting")));
    }

    #[test]
    fn file_finder_set_query_empty_returns_all() {
        let tmp = TempDir::new().unwrap();
        create_test_files(&tmp);
        let mut finder = FileFinder::new(tmp.path().to_path_buf());
        finder.open().unwrap();

        let total = finder.result_count();
        finder.set_query("meeting");
        finder.set_query("");
        assert_eq!(finder.result_count(), total);
    }

    #[test]
    fn file_finder_set_query_resets_selected_index() {
        let tmp = TempDir::new().unwrap();
        create_test_files(&tmp);
        let mut finder = FileFinder::new(tmp.path().to_path_buf());
        finder.open().unwrap();

        finder.selected_index = 2;
        finder.set_query("meeting");
        assert_eq!(finder.selected_index, 0);
    }

    #[test]
    fn file_finder_select_next_advances_and_wraps() {
        let tmp = TempDir::new().unwrap();
        create_test_files(&tmp);
        let mut finder = FileFinder::new(tmp.path().to_path_buf());
        finder.open().unwrap();

        let count = finder.result_count();
        assert!(count >= 2, "Need at least 2 results to test wrapping");

        assert_eq!(finder.selected_index, 0);
        finder.select_next();
        assert_eq!(finder.selected_index, 1);

        // Advance to end and verify wrap
        for _ in 0..count - 1 {
            finder.select_next();
        }
        assert_eq!(finder.selected_index, 0);
    }

    #[test]
    fn file_finder_select_prev_decreases_and_wraps() {
        let tmp = TempDir::new().unwrap();
        create_test_files(&tmp);
        let mut finder = FileFinder::new(tmp.path().to_path_buf());
        finder.open().unwrap();

        let count = finder.result_count();
        assert!(count >= 2, "Need at least 2 results to test wrapping");

        assert_eq!(finder.selected_index, 0);
        // Wraps to last
        finder.select_prev();
        assert_eq!(finder.selected_index, count - 1);
        // Back to second-to-last
        finder.select_prev();
        assert_eq!(finder.selected_index, count - 2);
    }

    #[test]
    fn file_finder_selected_path_returns_correct_path() {
        let tmp = TempDir::new().unwrap();
        create_test_files(&tmp);
        let mut finder = FileFinder::new(tmp.path().to_path_buf());
        finder.open().unwrap();

        let path = finder.selected_path().unwrap();
        assert_eq!(path, finder.results[0].path());

        finder.select_next();
        let path = finder.selected_path().unwrap();
        assert_eq!(path, finder.results[1].path());
    }

    #[test]
    fn file_finder_selected_path_no_results_returns_none() {
        let tmp = TempDir::new().unwrap();
        // Empty directory - no files
        let finder = FileFinder::new(tmp.path().to_path_buf());

        assert!(finder.selected_path().is_none());
    }

    #[test]
    fn file_finder_result_count_matches_results_len() {
        let tmp = TempDir::new().unwrap();
        create_test_files(&tmp);
        let mut finder = FileFinder::new(tmp.path().to_path_buf());
        finder.open().unwrap();

        assert_eq!(finder.result_count(), finder.results.len());

        finder.set_query("meeting");
        assert_eq!(finder.result_count(), finder.results.len());
    }

    // ── ContentSearchPanel tests ──────────────────────────────────────

    #[test]
    fn content_search_panel_new_defaults() {
        let tmp = TempDir::new().unwrap();
        let panel = ContentSearchPanel::new(tmp.path().to_path_buf());

        assert!(!panel.visible);
        assert!(panel.query.is_empty());
        assert_eq!(panel.selected_index, 0);
        assert!(panel.results.is_empty());
    }

    #[test]
    fn content_search_panel_open_close_visibility() {
        let tmp = TempDir::new().unwrap();
        let mut panel = ContentSearchPanel::new(tmp.path().to_path_buf());

        assert!(!panel.visible);
        panel.open();
        assert!(panel.visible);
        panel.close();
        assert!(!panel.visible);
    }

    #[test]
    fn content_search_panel_set_query_finds_matches() {
        let tmp = TempDir::new().unwrap();
        create_test_files(&tmp);
        let mut panel = ContentSearchPanel::new(tmp.path().to_path_buf());
        panel.open();

        panel.set_query("groceries");
        assert!(!panel.results.is_empty());
        assert!(panel
            .results
            .iter()
            .any(|m| m.line_text.contains("groceries")));
    }

    #[test]
    fn content_search_panel_set_query_no_matches() {
        let tmp = TempDir::new().unwrap();
        create_test_files(&tmp);
        let mut panel = ContentSearchPanel::new(tmp.path().to_path_buf());
        panel.open();

        panel.set_query("xyznonsense");
        assert!(panel.results.is_empty());
    }

    #[test]
    fn content_search_panel_set_query_resets_selected_index() {
        let tmp = TempDir::new().unwrap();
        create_test_files(&tmp);
        let mut panel = ContentSearchPanel::new(tmp.path().to_path_buf());
        panel.open();

        panel.set_query("Meeting");
        panel.selected_index = 5;
        panel.set_query("groceries");
        assert_eq!(panel.selected_index, 0);
    }

    #[test]
    fn content_search_panel_select_next_prev_wrapping() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join("a.md"), "apple\nbanana\napple pie").unwrap();
        let mut panel = ContentSearchPanel::new(root.to_path_buf());
        panel.open();

        panel.set_query("apple");
        let count = panel.result_count();
        assert!(count >= 2, "Need at least 2 results");

        // select_next
        assert_eq!(panel.selected_index, 0);
        panel.select_next();
        assert_eq!(panel.selected_index, 1);

        // Wrap forward
        for _ in 0..count - 1 {
            panel.select_next();
        }
        assert_eq!(panel.selected_index, 0);

        // select_prev wraps backward
        panel.select_prev();
        assert_eq!(panel.selected_index, count - 1);
        panel.select_prev();
        assert_eq!(panel.selected_index, count - 2);
    }

    #[test]
    fn content_search_panel_selected_match_returns_correct() {
        let tmp = TempDir::new().unwrap();
        create_test_files(&tmp);
        let mut panel = ContentSearchPanel::new(tmp.path().to_path_buf());
        panel.open();

        panel.set_query("groceries");
        let m = panel.selected_match().unwrap();
        assert!(m.line_text.contains("groceries"));
        assert!(m.path.to_string_lossy().contains("todo"));
    }

    #[test]
    fn content_search_panel_selected_match_no_results_returns_none() {
        let tmp = TempDir::new().unwrap();
        let panel = ContentSearchPanel::new(tmp.path().to_path_buf());

        assert!(panel.selected_match().is_none());
    }

    #[test]
    fn content_search_panel_result_count() {
        let tmp = TempDir::new().unwrap();
        create_test_files(&tmp);
        let mut panel = ContentSearchPanel::new(tmp.path().to_path_buf());
        panel.open();

        panel.set_query("groceries");
        assert_eq!(panel.result_count(), panel.results.len());
        assert!(panel.result_count() > 0);
    }
}
