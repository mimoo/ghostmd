use chrono::{Local, NaiveDate};
use std::path::{Path, PathBuf};

/// Returns the default ghostmd root directory.
/// On macOS: `~/Documents/ghostmd`
/// On Linux: `$XDG_DATA_HOME/ghostmd` (defaults to `~/.local/share/ghostmd`)
pub fn ghostmd_root() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME environment variable not set");

    #[cfg(target_os = "macos")]
    {
        PathBuf::from(home).join("Documents").join("ghostmd")
    }

    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            PathBuf::from(xdg).join("ghostmd")
        } else {
            PathBuf::from(home).join(".local").join("share").join("ghostmd")
        }
    }
}

/// Returns the diary directory for a specific date, e.g. `<root>/diary/2024/03/15/`.
pub fn diary_dir(root: &Path, date: NaiveDate) -> PathBuf {
    root.join("diary")
        .join(date.format("%Y").to_string())
        .join(format!("{}-{}", date.format("%m"), date.format("%B").to_string().to_lowercase()))
        .join(date.format("%d").to_string())
}

/// Returns the diary directory for today.
pub fn today_diary_dir(root: &Path) -> PathBuf {
    diary_dir(root, Local::now().date_naive())
}

/// Creates a new diary note path with a slugified title under today's diary directory.
///
/// Format: `<root>/diary/YYYY/MM/DD/<timestamp>-<slug>.md`
pub fn new_diary_path(root: &Path, title: &str) -> PathBuf {
    let dir = today_diary_dir(root);
    let now = Local::now();
    let timestamp = now.format("%H%M%S").to_string();
    let slug = slugify(title);
    dir.join(format!("{}-{}.md", timestamp, slug))
}

/// Converts a string into a URL/filename-safe slug.
///
/// - Lowercases the input
/// - Replaces spaces and non-alphanumeric characters with hyphens
/// - Collapses multiple hyphens into one
/// - Trims leading/trailing hyphens
/// - Returns "untitled" for empty input
pub fn slugify(s: &str) -> String {
    let lowered = s.to_lowercase();
    let replaced: String = lowered
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else {
                '-'
            }
        })
        .collect();
    // Collapse multiple hyphens
    let mut collapsed = String::new();
    let mut prev_hyphen = false;
    for c in replaced.chars() {
        if c == '-' {
            if !prev_hyphen {
                collapsed.push('-');
            }
            prev_hyphen = true;
        } else {
            collapsed.push(c);
            prev_hyphen = false;
        }
    }
    // Trim leading/trailing hyphens
    let trimmed = collapsed.trim_matches('-');
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Find the most recent day folder in the diary directory, excluding `exclude`.
/// Walks `diary/YYYY/MM-month/DD/` and returns the lexicographically last day directory
/// that is not `exclude` (typically today's dir, so we carry over from a previous day).
pub fn last_diary_day_dir(root: &Path, exclude: &Path) -> Option<PathBuf> {
    let diary = root.join("diary");
    if !diary.is_dir() {
        return None;
    }

    let mut day_dirs: Vec<PathBuf> = Vec::new();
    collect_day_dirs(&diary, 0, &mut day_dirs);

    // Lexicographic sort works because paths are YYYY/MM-month/DD
    day_dirs.sort();
    while let Some(dir) = day_dirs.pop() {
        if dir != exclude {
            return Some(dir);
        }
    }
    None
}

/// Recursively collect leaf directories (day folders) at depth 3 from diary root.
fn collect_day_dirs(dir: &Path, depth: usize, dirs: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if depth >= 2 {
                    // depth 0=YYYY, 1=MM-month, 2=DD — this is a day folder
                    dirs.push(path);
                } else {
                    collect_day_dirs(&path, depth + 1, dirs);
                }
            }
        }
    }
}

/// Collect all pending checkbox items (`- [ ] ...`) with their header hierarchy
/// from all `.md` files in a directory.
pub fn pending_items_in_dir(dir: &Path) -> Vec<String> {
    let mut items = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut paths: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
            .collect();
        paths.sort();
        for path in paths {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let file_items = extract_pending_with_headers(&content);
                if !file_items.is_empty() {
                    if !items.is_empty() {
                        items.push(String::new()); // blank line between files
                    }
                    items.extend(file_items);
                }
            }
        }
    }
    items
}

/// Extract pending items with their full header hierarchy preserved.
///
/// Tracks a stack of headings (`#`, `##`, etc.). When a `- [ ]` item is found,
/// emits any not-yet-emitted headers from the stack first. Headers with no
/// pending items underneath are skipped entirely.
pub fn extract_pending_with_headers(content: &str) -> Vec<String> {
    // Stack of (heading level, heading line, emitted)
    let mut header_stack: Vec<(usize, String, bool)> = Vec::new();
    let mut result: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|&c| c == '#').count();
            // Pop headers at same level or deeper
            while header_stack.last().map(|(l, _, _)| *l >= level).unwrap_or(false) {
                header_stack.pop();
            }
            header_stack.push((level, line.to_string(), false));
        } else if trimmed.starts_with("- [ ]") {
            // Emit any unemitted headers from the stack
            let mut needs_separator = !result.is_empty();
            for entry in header_stack.iter_mut() {
                if !entry.2 {
                    if needs_separator {
                        result.push(String::new());
                        needs_separator = false;
                    }
                    result.push(entry.1.clone());
                    entry.2 = true;
                }
            }
            result.push(line.to_string());
        }
    }
    result
}

/// Extract pending checkbox items (`- [ ] ...`) from note content.
pub fn extract_pending_items(content: &str) -> Vec<String> {
    content
        .lines()
        .filter(|line| line.trim_start().starts_with("- [ ]"))
        .map(|line| line.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn diary_dir_for_specific_date() {
        let root = Path::new("/notes");
        let date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        let dir = diary_dir(root, date);
        assert_eq!(dir, PathBuf::from("/notes/diary/2024/03-march/15"));
    }

    #[test]
    fn diary_dir_for_january() {
        let root = Path::new("/notes");
        let date = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap();
        let dir = diary_dir(root, date);
        assert_eq!(dir, PathBuf::from("/notes/diary/2024/01-january/05"));
    }

    #[test]
    fn today_diary_dir_uses_current_date() {
        let root = Path::new("/notes");
        let dir = today_diary_dir(root);
        let today = Local::now().date_naive();
        let expected = diary_dir(root, today);
        assert_eq!(dir, expected);
    }

    #[test]
    fn slugify_simple_text() {
        assert_eq!(slugify("Hello World"), "hello-world");
    }

    #[test]
    fn slugify_special_chars() {
        assert_eq!(slugify("Hello, World! #2024"), "hello-world-2024");
    }

    #[test]
    fn slugify_empty_string() {
        assert_eq!(slugify(""), "untitled");
    }

    #[test]
    fn slugify_whitespace_only() {
        assert_eq!(slugify("   "), "untitled");
    }

    #[test]
    fn slugify_already_clean() {
        assert_eq!(slugify("clean-slug"), "clean-slug");
    }

    #[test]
    fn slugify_multiple_spaces() {
        assert_eq!(slugify("a   b   c"), "a-b-c");
    }

    #[test]
    fn new_diary_path_format() {
        let root = Path::new("/notes");
        let path = new_diary_path(root, "Meeting Notes");
        // Should be under today's diary directory
        let today = Local::now().date_naive();
        let expected_dir = diary_dir(root, today);
        assert!(path.starts_with(&expected_dir));

        // Should contain the slug and .md extension
        let filename = path.file_name().unwrap().to_string_lossy();
        assert!(filename.contains("meeting-notes"));
        assert!(filename.ends_with(".md"));
    }

    #[test]
    fn ghostmd_root_ends_with_ghostmd() {
        let root = ghostmd_root();
        let root_str = root.to_string_lossy();
        assert!(root_str.ends_with("ghostmd"));
    }

    #[test]
    fn slugify_unicode_accented_chars() {
        // Accented characters are non-alphanumeric ASCII, so they get replaced
        assert_eq!(slugify("café résumé"), "caf-r-sum");
    }

    #[test]
    fn slugify_cjk_characters() {
        // CJK characters are non-ASCII alphanumeric, so they get replaced with hyphens
        // and collapsed/trimmed, resulting in "untitled"
        assert_eq!(slugify("会議ノート"), "untitled");
    }

    #[test]
    fn slugify_leading_trailing_special_chars() {
        assert_eq!(slugify("---hello---"), "hello");
        assert_eq!(slugify("***test!!!"), "test");
        assert_eq!(slugify("...dots..."), "dots");
    }

    #[test]
    fn new_diary_path_empty_title_produces_untitled() {
        let root = Path::new("/notes");
        let path = new_diary_path(root, "");
        let filename = path.file_name().unwrap().to_string_lossy();
        assert!(filename.contains("untitled"));
        assert!(filename.ends_with(".md"));
    }

    #[test]
    fn diary_dir_leap_year_feb_29() {
        let root = Path::new("/notes");
        let date = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        let dir = diary_dir(root, date);
        assert_eq!(dir, PathBuf::from("/notes/diary/2024/02-february/29"));
    }

    #[test]
    fn diary_dir_dec_31() {
        let root = Path::new("/notes");
        let date = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let dir = diary_dir(root, date);
        assert_eq!(dir, PathBuf::from("/notes/diary/2024/12-december/31"));
    }

    #[test]
    fn diary_dir_new_years_day() {
        let root = Path::new("/notes");
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let dir = diary_dir(root, date);
        assert_eq!(dir, PathBuf::from("/notes/diary/2025/01-january/01"));
    }

    #[test]
    fn extract_pending_items_basic() {
        let content = "# Notes\n- [x] done\n- [ ] buy milk\n- [ ] call dentist\nsome text\n";
        let items = extract_pending_items(content);
        assert_eq!(items, vec!["- [ ] buy milk", "- [ ] call dentist"]);
    }

    #[test]
    fn extract_pending_items_indented() {
        let content = "  - [ ] indented item\n- [ ] normal item\n";
        let items = extract_pending_items(content);
        assert_eq!(items, vec!["  - [ ] indented item", "- [ ] normal item"]);
    }

    #[test]
    fn extract_pending_items_empty() {
        let items = extract_pending_items("no checkboxes here\n- [x] all done\n");
        assert!(items.is_empty());
    }

    #[test]
    fn last_diary_day_dir_finds_most_recent_excluding_today() {
        let tmp = std::env::temp_dir().join("ghostmd_test_last_day");
        let _ = std::fs::remove_dir_all(&tmp);
        let day1 = tmp.join("diary/2024/03-march/14");
        let day2 = tmp.join("diary/2024/03-march/15");
        let today = tmp.join("diary/2024/03-march/16");
        std::fs::create_dir_all(&day1).unwrap();
        std::fs::create_dir_all(&day2).unwrap();
        std::fs::create_dir_all(&today).unwrap();
        std::fs::write(day1.join("notes.md"), "old").unwrap();
        std::fs::write(day2.join("notes.md"), "- [ ] pending").unwrap();

        // Excluding today (16), should find 15
        let result = last_diary_day_dir(&tmp, &today).unwrap();
        assert!(result.ends_with("15"));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn last_diary_day_dir_across_months() {
        let tmp = std::env::temp_dir().join("ghostmd_test_last_day_months");
        let _ = std::fs::remove_dir_all(&tmp);
        let day1 = tmp.join("diary/2024/02-february/28");
        let today = tmp.join("diary/2024/03-march/01");
        std::fs::create_dir_all(&day1).unwrap();
        std::fs::create_dir_all(&today).unwrap();
        std::fs::write(day1.join("notes.md"), "- [ ] item").unwrap();

        // Excluding today (03/01), should find 02/28
        let result = last_diary_day_dir(&tmp, &today).unwrap();
        assert!(result.ends_with("02-february/28"));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn last_diary_day_dir_no_diary() {
        let tmp = std::env::temp_dir().join("ghostmd_test_no_diary");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let exclude = tmp.join("diary/2024/03-march/15");
        assert!(last_diary_day_dir(&tmp, &exclude).is_none());
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn last_diary_day_dir_only_today_returns_none() {
        let tmp = std::env::temp_dir().join("ghostmd_test_only_today");
        let _ = std::fs::remove_dir_all(&tmp);
        let today = tmp.join("diary/2024/03-march/15");
        std::fs::create_dir_all(&today).unwrap();
        std::fs::write(today.join("notes.md"), "- [ ] item").unwrap();

        assert!(last_diary_day_dir(&tmp, &today).is_none());

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn pending_with_headers_full_hierarchy() {
        let content = "# Work\n## Backend\n- [ ] fix bug\n- [x] deploy\n## Frontend\n- [ ] update CSS\n# Personal\n- [x] done\n";
        let result = extract_pending_with_headers(content);
        assert_eq!(result, vec![
            "# Work",
            "## Backend",
            "- [ ] fix bug",
            "",
            "## Frontend",
            "- [ ] update CSS",
        ]);
    }

    #[test]
    fn pending_with_headers_no_headings() {
        let content = "- [ ] buy milk\n- [x] done\n- [ ] call dentist\n";
        let result = extract_pending_with_headers(content);
        assert_eq!(result, vec!["- [ ] buy milk", "- [ ] call dentist"]);
    }

    #[test]
    fn pending_with_headers_skips_empty_sections() {
        let content = "# Has items\n- [ ] yes\n# No items\n- [x] all done\n# Also has items\n- [ ] pending\n";
        let result = extract_pending_with_headers(content);
        assert_eq!(result, vec![
            "# Has items",
            "- [ ] yes",
            "",
            "# Also has items",
            "- [ ] pending",
        ]);
    }

    #[test]
    fn pending_with_headers_deep_nesting() {
        let content = "# A\n## B\n### C\n- [ ] deep item\n";
        let result = extract_pending_with_headers(content);
        assert_eq!(result, vec!["# A", "## B", "### C", "- [ ] deep item"]);
    }

    #[test]
    fn pending_with_headers_sibling_resets() {
        // When ## changes under same #, only ## should re-emit (# already emitted)
        let content = "# Top\n## First\n- [ ] a\n## Second\n- [ ] b\n";
        let result = extract_pending_with_headers(content);
        assert_eq!(result, vec![
            "# Top",
            "## First",
            "- [ ] a",
            "",
            "## Second",
            "- [ ] b",
        ]);
    }

    #[test]
    fn pending_with_headers_no_pending() {
        let content = "# Work\n- [x] done\n## Sub\nsome text\n";
        let result = extract_pending_with_headers(content);
        assert!(result.is_empty());
    }

    #[test]
    fn pending_items_in_dir_with_headers() {
        let tmp = std::env::temp_dir().join("ghostmd_test_pending_headers");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("notes.md"), "# Work\n- [ ] buy milk\n- [x] done\n").unwrap();
        std::fs::write(tmp.join("todo.md"), "# Errands\n- [ ] call dentist\n").unwrap();

        let items = pending_items_in_dir(&tmp);
        assert_eq!(items, vec![
            "# Work",
            "- [ ] buy milk",
            "",
            "# Errands",
            "- [ ] call dentist",
        ]);

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn pending_items_in_dir_empty_dir() {
        let tmp = std::env::temp_dir().join("ghostmd_test_pending_empty");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(pending_items_in_dir(&tmp).is_empty());
        std::fs::remove_dir_all(&tmp).ok();
    }
}
