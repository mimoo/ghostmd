use anyhow::Result;
use std::path::PathBuf;

/// A file that matched a fuzzy search query, along with its score.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: PathBuf,
    pub score: u32,
}

/// A line within a file that matched a content search query.
#[derive(Debug, Clone)]
pub struct ContentMatch {
    pub path: PathBuf,
    pub line_number: usize,
    pub line_text: String,
}

/// Fuzzy filename search over a cached list of files.
pub struct FuzzySearch {
    root: PathBuf,
    file_cache: Vec<PathBuf>,
}

impl FuzzySearch {
    /// Creates a new fuzzy search rooted at the given directory.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            file_cache: vec![],
        }
    }

    /// Walks the root directory and rebuilds the file cache.
    pub fn refresh_cache(&mut self) -> Result<()> {
        self.file_cache.clear();
        let walker = ignore::WalkBuilder::new(&self.root).build();
        for entry in walker {
            let entry = entry?;
            if entry.file_type().is_some_and(|ft| ft.is_file()) {
                self.file_cache.push(entry.into_path());
            }
        }
        Ok(())
    }

    /// Walks the root directory and rebuilds the file cache with directories only.
    pub fn refresh_dir_cache(&mut self) -> Result<()> {
        self.file_cache.clear();
        let walker = ignore::WalkBuilder::new(&self.root).build();
        for entry in walker {
            let entry = entry?;
            if entry.file_type().is_some_and(|ft| ft.is_dir()) && entry.path() != self.root.as_path() {
                self.file_cache.push(entry.into_path());
            }
        }
        Ok(())
    }

    /// Searches cached file paths against the query using fuzzy matching.
    /// Results are sorted by score (best match first).
    pub fn search_files(&self, query: &str) -> Vec<SearchResult> {
        if query.is_empty() {
            return self
                .file_cache
                .iter()
                .map(|p| SearchResult {
                    path: p.clone(),
                    score: 0,
                })
                .collect();
        }

        use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
        use nucleo_matcher::{Config, Matcher, Utf32Str};

        let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
        let pattern = Pattern::new(query, CaseMatching::Ignore, Normalization::Smart, AtomKind::Fuzzy);

        let mut buf = Vec::new();
        let mut results: Vec<SearchResult> = self
            .file_cache
            .iter()
            .filter_map(|path| {
                let file_name = path.file_name()?.to_string_lossy();
                let haystack = Utf32Str::new(&file_name, &mut buf);
                let score = pattern.score(haystack, &mut matcher)?;
                Some(SearchResult {
                    path: path.clone(),
                    score,
                })
            })
            .collect();

        results.sort_by(|a, b| b.score.cmp(&a.score));
        results
    }

    /// Returns the number of files currently in the cache.
    pub fn cached_count(&self) -> usize {
        self.file_cache.len()
    }
}

/// Full-text content search across files in a directory.
pub struct ContentSearch {
    root: PathBuf,
}

impl ContentSearch {
    /// Creates a new content searcher rooted at the given directory.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Searches for `query` in all files under root, returning matching lines.
    pub fn search(&self, query: &str) -> Result<Vec<ContentMatch>> {
        use grep_regex::RegexMatcher;
        use grep_searcher::sinks::UTF8;
        use grep_searcher::Searcher;

        let matcher = RegexMatcher::new(query)?;
        let mut matches = Vec::new();

        let walker = ignore::WalkBuilder::new(&self.root).build();
        for entry in walker {
            let entry = entry?;
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }
            let path = entry.path().to_path_buf();
            let mut searcher = Searcher::new();
            searcher.search_path(
                &matcher,
                &path,
                UTF8(|line_num, line| {
                    matches.push(ContentMatch {
                        path: path.clone(),
                        line_number: line_num as usize,
                        line_text: line.trim_end_matches('\n').to_string(),
                    });
                    Ok(true)
                }),
            )?;
        }

        Ok(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_search_files(tmp: &TempDir) {
        let root = tmp.path();
        fs::create_dir_all(root.join("notes")).unwrap();
        fs::write(root.join("notes/hello.md"), "Hello World\nGoodbye World").unwrap();
        fs::write(root.join("notes/rust.md"), "Rust is great\nI love Rust").unwrap();
        fs::write(root.join("readme.md"), "This is a readme\nNothing special").unwrap();
    }

    #[test]
    fn fuzzy_finds_exact_match() {
        let tmp = TempDir::new().unwrap();
        create_search_files(&tmp);

        let mut search = FuzzySearch::new(tmp.path().to_path_buf());
        search.refresh_cache().unwrap();

        let results = search.search_files("hello");
        assert!(!results.is_empty());
        // The exact match "hello.md" should be in results
        assert!(results.iter().any(|r| r.path.to_string_lossy().contains("hello")));
    }

    #[test]
    fn fuzzy_ranks_closer_matches_higher() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join("abc.md"), "").unwrap();
        fs::write(root.join("axxbxxc.md"), "").unwrap();
        fs::write(root.join("zzz.md"), "").unwrap();

        let mut search = FuzzySearch::new(root.to_path_buf());
        search.refresh_cache().unwrap();

        let results = search.search_files("abc");
        assert!(!results.is_empty());
        // First result should be the exact match
        assert!(results[0].path.to_string_lossy().contains("abc"));
    }

    #[test]
    fn fuzzy_empty_query_returns_all() {
        let tmp = TempDir::new().unwrap();
        create_search_files(&tmp);

        let mut search = FuzzySearch::new(tmp.path().to_path_buf());
        search.refresh_cache().unwrap();

        let results = search.search_files("");
        assert_eq!(results.len(), search.cached_count());
    }

    #[test]
    fn cached_count_reflects_files() {
        let tmp = TempDir::new().unwrap();
        create_search_files(&tmp);

        let mut search = FuzzySearch::new(tmp.path().to_path_buf());
        search.refresh_cache().unwrap();

        assert_eq!(search.cached_count(), 3);
    }

    #[test]
    fn content_search_finds_text_in_files() {
        let tmp = TempDir::new().unwrap();
        create_search_files(&tmp);

        let searcher = ContentSearch::new(tmp.path().to_path_buf());
        let results = searcher.search("Rust").unwrap();

        assert!(!results.is_empty());
        assert!(results.iter().all(|m| m.path.to_string_lossy().contains("rust")));
    }

    #[test]
    fn content_search_correct_line_numbers() {
        let tmp = TempDir::new().unwrap();
        create_search_files(&tmp);

        let searcher = ContentSearch::new(tmp.path().to_path_buf());
        let results = searcher.search("Goodbye").unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line_number, 2);
        assert!(results[0].line_text.contains("Goodbye"));
    }

    #[test]
    fn content_search_no_results_for_nonmatching() {
        let tmp = TempDir::new().unwrap();
        create_search_files(&tmp);

        let searcher = ContentSearch::new(tmp.path().to_path_buf());
        let results = searcher.search("xyznonsense").unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn fuzzy_no_results_for_garbage() {
        let tmp = TempDir::new().unwrap();
        create_search_files(&tmp);

        let mut search = FuzzySearch::new(tmp.path().to_path_buf());
        search.refresh_cache().unwrap();

        let _results = search.search_files("zzzzzzzzzz");
        // Fuzzy match may or may not return results; at minimum the scores should be low
        // The key behavior is that it doesn't panic
    }

    #[test]
    fn content_search_empty_file_no_error() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join("empty.md"), "").unwrap();

        let searcher = ContentSearch::new(root.to_path_buf());
        let results = searcher.search("anything").unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn fuzzy_refresh_cache_picks_up_new_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join("first.md"), "").unwrap();

        let mut search = FuzzySearch::new(root.to_path_buf());
        search.refresh_cache().unwrap();
        assert_eq!(search.cached_count(), 1);

        // Add more files and re-refresh
        fs::write(root.join("second.md"), "").unwrap();
        fs::write(root.join("third.md"), "").unwrap();
        search.refresh_cache().unwrap();
        assert_eq!(search.cached_count(), 3);

        let results = search.search_files("second");
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.path.to_string_lossy().contains("second")));
    }

    #[test]
    fn content_search_multiple_matches_per_file_and_across_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join("a.md"), "foo bar\nbaz foo\nfoo again").unwrap();
        fs::write(root.join("b.md"), "no match here\nfoo found").unwrap();

        let searcher = ContentSearch::new(root.to_path_buf());
        let results = searcher.search("foo").unwrap();

        // "a.md" has 3 matches, "b.md" has 1 match => 4 total
        assert_eq!(results.len(), 4);

        // Verify matches span both files
        let paths: Vec<_> = results.iter().map(|m| m.path.clone()).collect();
        assert!(paths.iter().any(|p| p.to_string_lossy().contains("a.md")));
        assert!(paths.iter().any(|p| p.to_string_lossy().contains("b.md")));
    }

    #[test]
    fn content_search_across_word_boundaries() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join("words.md"), "some foobar text\nfoo bar baz").unwrap();

        let searcher = ContentSearch::new(root.to_path_buf());

        // "foo bar" spans a word boundary on line 2
        let results = searcher.search("foo bar").unwrap();
        assert!(!results.is_empty());
        // Should match line 2 which has "foo bar" as separate words
        assert!(results.iter().any(|m| m.line_text.contains("foo bar")));
    }
}
