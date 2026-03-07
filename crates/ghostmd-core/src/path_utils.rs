use std::path::{Path, PathBuf};

/// Generate a unique path by appending `-2`, `-3`, ... if `base` already exists.
/// For directories, appends the suffix to the directory name.
/// For files, appends the suffix before the extension.
/// Returns `base` unchanged if it doesn't exist.
pub fn unique_path(base: &Path) -> PathBuf {
    if !base.exists() {
        return base.to_path_buf();
    }
    let parent = base.parent().unwrap_or(Path::new("."));
    let stem = base.file_stem().unwrap_or_default().to_string_lossy();
    let ext = base.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let is_dir = base.is_dir();

    for n in 2..100 {
        let candidate = if is_dir {
            parent.join(format!("{}-{}", stem, n))
        } else {
            parent.join(format!("{}-{}{}", stem, n, ext))
        };
        if !candidate.exists() {
            return candidate;
        }
    }
    base.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn unique_path_no_collision() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("notes.md");
        assert_eq!(unique_path(&path), path);
    }

    #[test]
    fn unique_path_file_collision() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("notes.md");
        std::fs::write(&path, "").unwrap();
        let result = unique_path(&path);
        assert_eq!(result, tmp.path().join("notes-2.md"));
    }

    #[test]
    fn unique_path_multiple_collisions() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("notes.md"), "").unwrap();
        std::fs::write(tmp.path().join("notes-2.md"), "").unwrap();
        let result = unique_path(&tmp.path().join("notes.md"));
        assert_eq!(result, tmp.path().join("notes-3.md"));
    }

    #[test]
    fn unique_path_directory_collision() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("folder");
        std::fs::create_dir(&dir).unwrap();
        let result = unique_path(&dir);
        assert_eq!(result, tmp.path().join("folder-2"));
    }
}
