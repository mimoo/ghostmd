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

/// Find the most recent diary note file by walking the diary directory.
/// Relies on the `YYYY/MM-month/DD/HHmmss-slug.md` structure for lexicographic ordering.
pub fn last_diary_note(root: &Path) -> Option<PathBuf> {
    let diary = root.join("diary");
    if !diary.is_dir() {
        return None;
    }

    let mut notes: Vec<PathBuf> = Vec::new();
    collect_diary_notes(&diary, &mut notes);

    // Lexicographic sort works because paths are YYYY/MM/DD/HHmmss
    notes.sort();
    notes.pop()
}

fn collect_diary_notes(dir: &Path, notes: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_diary_notes(&path, notes);
            } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                notes.push(path);
            }
        }
    }
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
    fn last_diary_note_finds_most_recent() {
        let tmp = std::env::temp_dir().join("ghostmd_test_last_diary");
        let _ = std::fs::remove_dir_all(&tmp);
        let diary = tmp.join("diary/2024/03-march/15");
        std::fs::create_dir_all(&diary).unwrap();
        std::fs::write(diary.join("100000-first.md"), "first").unwrap();
        std::fs::write(diary.join("120000-second.md"), "second").unwrap();

        let result = last_diary_note(&tmp).unwrap();
        assert!(result.ends_with("120000-second.md"));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn last_diary_note_across_days() {
        let tmp = std::env::temp_dir().join("ghostmd_test_last_diary_days");
        let _ = std::fs::remove_dir_all(&tmp);
        let day1 = tmp.join("diary/2024/03-march/14");
        let day2 = tmp.join("diary/2024/03-march/15");
        std::fs::create_dir_all(&day1).unwrap();
        std::fs::create_dir_all(&day2).unwrap();
        std::fs::write(day1.join("230000-old.md"), "old").unwrap();
        std::fs::write(day2.join("080000-new.md"), "new").unwrap();

        let result = last_diary_note(&tmp).unwrap();
        assert!(result.ends_with("080000-new.md"));

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn last_diary_note_no_diary_dir() {
        let tmp = std::env::temp_dir().join("ghostmd_test_no_diary");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(last_diary_note(&tmp).is_none());
        std::fs::remove_dir_all(&tmp).ok();
    }
}
