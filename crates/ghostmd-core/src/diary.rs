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
/// Walks `diary/YYYY/{month or MM-month}/DD/` and sorts by parsed date to handle
/// both old (`march/`) and new (`04-april/`) directory formats.
pub fn last_diary_day_dir(root: &Path, exclude: &Path) -> Option<PathBuf> {
    let diary = root.join("diary");
    if !diary.is_dir() {
        return None;
    }

    let mut day_dirs: Vec<PathBuf> = Vec::new();
    collect_day_dirs(&diary, 0, &mut day_dirs);

    // Sort by parsed date (not lexicographic) to handle mixed month formats
    day_dirs.sort_by_key(|p| parse_day_dir_date(p));
    while let Some(dir) = day_dirs.pop() {
        if dir != exclude {
            return Some(dir);
        }
    }
    None
}

/// Parse a day directory path into a NaiveDate for sorting.
/// Expects path ending in `YYYY/{month-dir}/DD` where month-dir is either
/// `march` (old format) or `03-march` (new format).
fn parse_day_dir_date(path: &Path) -> Option<NaiveDate> {
    let day: u32 = path.file_name()?.to_str()?.parse().ok()?;
    let month_dir = path.parent()?.file_name()?.to_str()?;
    let year: i32 = path.parent()?.parent()?.file_name()?.to_str()?.parse().ok()?;
    let month = parse_month(month_dir)?;
    NaiveDate::from_ymd_opt(year, month, day)
}

/// Parse a month directory name into a month number (1-12).
/// Handles both `march` and `03-march` formats.
fn parse_month(s: &str) -> Option<u32> {
    // Try "MM-month" format first (e.g. "03-march")
    if let Some((num, _)) = s.split_once('-') {
        if let Ok(m) = num.parse::<u32>() {
            if (1..=12).contains(&m) {
                return Some(m);
            }
        }
    }
    // Fall back to bare month name (e.g. "march")
    match s.to_lowercase().as_str() {
        "january" => Some(1), "february" => Some(2), "march" => Some(3),
        "april" => Some(4), "may" => Some(5), "june" => Some(6),
        "july" => Some(7), "august" => Some(8), "september" => Some(9),
        "october" => Some(10), "november" => Some(11), "december" => Some(12),
        _ => None,
    }
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

/// Collect all pending checkbox items (`- [ ] ...`) from all `.md` files in a directory.
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
                items.extend(extract_pending_items(&content));
            }
        }
    }
    items
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
    fn last_diary_day_dir_mixed_old_and_new_format() {
        // Old format: "march/DD", new format: "04-april/DD"
        let tmp = std::env::temp_dir().join("ghostmd_test_mixed_format");
        let _ = std::fs::remove_dir_all(&tmp);
        let old_day = tmp.join("diary/2026/march/29");
        let new_day = tmp.join("diary/2026/04-april/02");
        let today = tmp.join("diary/2026/04-april/04");
        std::fs::create_dir_all(&old_day).unwrap();
        std::fs::create_dir_all(&new_day).unwrap();
        std::fs::create_dir_all(&today).unwrap();
        std::fs::write(old_day.join("notes.md"), "- [ ] old item").unwrap();
        std::fs::write(new_day.join("notes.md"), "- [ ] new item").unwrap();

        // Should find 04-april/02 (April 2) not march/29 (March 29)
        let result = last_diary_day_dir(&tmp, &today).unwrap();
        assert!(result.ends_with("04-april/02"), "got: {}", result.display());

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
    fn pending_items_in_dir_collects_from_all_files() {
        let tmp = std::env::temp_dir().join("ghostmd_test_pending_dir");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("notes.md"), "- [ ] buy milk\n- [x] done\n").unwrap();
        std::fs::write(tmp.join("todo.md"), "- [ ] call dentist\n").unwrap();

        let items = pending_items_in_dir(&tmp);
        assert_eq!(items, vec!["- [ ] buy milk", "- [ ] call dentist"]);

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
