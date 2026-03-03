use chrono::{Local, NaiveDate};
use std::path::{Path, PathBuf};

/// Returns the default ghostmd root directory: `~/Documents/ghostmd`.
pub fn ghostmd_root() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME environment variable not set");
    PathBuf::from(home).join("Documents").join("ghostmd")
}

/// Returns the diary directory for a specific date, e.g. `<root>/diary/2024/03/15/`.
pub fn diary_dir(root: &Path, date: NaiveDate) -> PathBuf {
    root.join("diary")
        .join(date.format("%Y").to_string())
        .join(date.format("%B").to_string().to_lowercase())
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn diary_dir_for_specific_date() {
        let root = Path::new("/notes");
        let date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        let dir = diary_dir(root, date);
        assert_eq!(dir, PathBuf::from("/notes/diary/2024/march/15"));
    }

    #[test]
    fn diary_dir_for_january() {
        let root = Path::new("/notes");
        let date = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap();
        let dir = diary_dir(root, date);
        assert_eq!(dir, PathBuf::from("/notes/diary/2024/january/05"));
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
    fn ghostmd_root_is_under_documents() {
        let root = ghostmd_root();
        let root_str = root.to_string_lossy();
        assert!(root_str.contains("Documents"));
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
        assert_eq!(dir, PathBuf::from("/notes/diary/2024/february/29"));
    }

    #[test]
    fn diary_dir_dec_31() {
        let root = Path::new("/notes");
        let date = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let dir = diary_dir(root, date);
        assert_eq!(dir, PathBuf::from("/notes/diary/2024/december/31"));
    }

    #[test]
    fn diary_dir_new_years_day() {
        let root = Path::new("/notes");
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let dir = diary_dir(root, date);
        assert_eq!(dir, PathBuf::from("/notes/diary/2025/january/01"));
    }
}
