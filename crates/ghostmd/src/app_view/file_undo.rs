use std::path::{Path, PathBuf};

/// Maximum number of undo entries to keep in the stack.
const MAX_UNDO_STACK: usize = 50;

/// A file operation that can be undone/redone.
pub(crate) enum FileOp {
    /// A file or directory was deleted (moved to trash).
    /// Stores the original path and in-memory backup of contents.
    Delete {
        path: PathBuf,
        backup: Vec<(PathBuf, Vec<u8>)>,
        is_dir: bool,
    },
    /// A batch of files were deleted together.
    DeleteBatch { ops: Vec<FileOp> },
    /// A file or directory was renamed in place.
    Rename {
        old_path: PathBuf,
        new_path: PathBuf,
    },
    /// A file or directory was moved to a different directory.
    Move {
        old_path: PathBuf,
        new_path: PathBuf,
    },
    /// A new file or directory was created.
    Create { path: PathBuf, is_dir: bool },
}

/// Back up a single file or directory's contents into memory.
pub(crate) fn backup_file(path: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    if path.is_dir() {
        backup_dir(path)
    } else {
        match std::fs::read(path) {
            Ok(bytes) => vec![(path.to_path_buf(), bytes)],
            Err(_) => vec![],
        }
    }
}

/// Recursively back up a directory's contents into memory.
fn backup_dir(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                result.extend(backup_dir(&p));
            } else if let Ok(bytes) = std::fs::read(&p) {
                result.push((p, bytes));
            }
        }
    }
    // Include the directory itself (empty marker) so we recreate it on restore
    result.push((dir.to_path_buf(), Vec::new()));
    result
}

/// Restore backed-up files to disk. Recreates parent directories as needed.
fn restore_backup(backup: &[(PathBuf, Vec<u8>)]) {
    // First pass: create directories
    for (path, data) in backup {
        if data.is_empty() && path.extension().is_none() {
            std::fs::create_dir_all(path).ok();
        } else if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
    }
    // Second pass: write files
    for (path, data) in backup {
        if !data.is_empty() || path.extension().is_some() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            std::fs::write(path, data).ok();
        }
    }
}

/// Push an operation onto the undo stack, clearing the redo stack.
pub(crate) fn push_undo(undo_stack: &mut Vec<FileOp>, redo_stack: &mut Vec<FileOp>, op: FileOp) {
    redo_stack.clear();
    undo_stack.push(op);
    if undo_stack.len() > MAX_UNDO_STACK {
        undo_stack.remove(0);
    }
}

/// Reverse a file operation (undo). Returns true on success.
/// The caller should move the op from undo to redo stack.
pub(crate) fn reverse_op(op: &mut FileOp) -> bool {
    match op {
        FileOp::Delete { path: _, backup, .. } => {
            restore_backup(backup);
            true
        }
        FileOp::DeleteBatch { ops } => {
            let mut ok = true;
            for sub_op in ops.iter_mut() {
                if !reverse_op(sub_op) {
                    ok = false;
                }
            }
            ok
        }
        FileOp::Rename { old_path, new_path } | FileOp::Move { old_path, new_path } => {
            // Undo: move new_path back to old_path
            std::fs::rename(&*new_path, &*old_path).is_ok()
        }
        FileOp::Create { path, is_dir } => {
            // Undo create: back up contents, then delete
            let new_backup = backup_file(path);
            let removed = if *is_dir {
                std::fs::remove_dir_all(&*path).is_ok()
            } else {
                std::fs::remove_file(&*path).is_ok()
            };
            if removed {
                // Morph into Delete so redo can restore
                *op = FileOp::Delete {
                    path: path.clone(),
                    backup: new_backup,
                    is_dir: *is_dir,
                };
                true
            } else {
                false
            }
        }
    }
}

/// Re-apply a file operation (redo). Returns true on success.
/// The caller should move the op from redo to undo stack.
pub(crate) fn reapply_op(op: &mut FileOp) -> bool {
    match op {
        FileOp::Delete { path, backup, is_dir } => {
            // Redo delete: re-backup current state, then delete
            let new_backup = backup_file(path);
            let removed = if *is_dir {
                std::fs::remove_dir_all(&*path).is_ok()
            } else {
                std::fs::remove_file(&*path).is_ok()
            };
            if removed {
                *backup = new_backup;
                true
            } else {
                false
            }
        }
        FileOp::DeleteBatch { ops } => {
            let mut ok = true;
            for sub_op in ops.iter_mut() {
                if !reapply_op(sub_op) {
                    ok = false;
                }
            }
            ok
        }
        FileOp::Rename { old_path, new_path } | FileOp::Move { old_path, new_path } => {
            // Redo: move old_path to new_path
            std::fs::rename(&*old_path, &*new_path).is_ok()
        }
        FileOp::Create { path, is_dir } => {
            // This was morphed from a Delete by reverse_op; restore backup
            // Actually this shouldn't happen in normal flow since reverse_op
            // converts Create→Delete. But handle it gracefully.
            if *is_dir {
                std::fs::create_dir_all(&*path).is_ok()
            } else {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                std::fs::write(&*path, "").is_ok()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_backup_and_restore_file() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("test.md");
        fs::write(&file, "hello world").unwrap();

        let backup = backup_file(&file);
        assert_eq!(backup.len(), 1);
        assert_eq!(backup[0].1, b"hello world");

        fs::remove_file(&file).unwrap();
        assert!(!file.exists());

        restore_backup(&backup);
        assert!(file.exists());
        assert_eq!(fs::read_to_string(&file).unwrap(), "hello world");
    }

    #[test]
    fn test_backup_and_restore_directory() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("notes");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.md"), "aaa").unwrap();
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("sub").join("b.md"), "bbb").unwrap();

        let backup = backup_file(&dir);
        assert!(backup.len() >= 3);

        fs::remove_dir_all(&dir).unwrap();
        assert!(!dir.exists());

        restore_backup(&backup);
        assert!(dir.exists());
        assert_eq!(fs::read_to_string(dir.join("a.md")).unwrap(), "aaa");
        assert_eq!(fs::read_to_string(dir.join("sub").join("b.md")).unwrap(), "bbb");
    }

    #[test]
    fn test_undo_redo_create() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("new.md");
        fs::write(&file, "content").unwrap();

        let mut op = FileOp::Create {
            path: file.clone(),
            is_dir: false,
        };

        // Undo create = delete the file (morphs op to Delete)
        assert!(reverse_op(&mut op));
        assert!(!file.exists());

        // Redo = restore from backup (the morphed Delete op)
        assert!(reverse_op(&mut op));
        assert!(file.exists());
        assert_eq!(fs::read_to_string(&file).unwrap(), "content");
    }

    #[test]
    fn test_undo_redo_rename() {
        let tmp = TempDir::new().unwrap();
        let old = tmp.path().join("old.md");
        let new = tmp.path().join("new.md");
        fs::write(&old, "content").unwrap();
        fs::rename(&old, &new).unwrap();

        let mut op = FileOp::Rename {
            old_path: old.clone(),
            new_path: new.clone(),
        };

        // Undo rename: new -> old
        assert!(reverse_op(&mut op));
        assert!(old.exists());
        assert!(!new.exists());

        // Redo rename: old -> new
        assert!(reapply_op(&mut op));
        assert!(!old.exists());
        assert!(new.exists());
    }

    #[test]
    fn test_undo_redo_move() {
        let tmp = TempDir::new().unwrap();
        let dir_a = tmp.path().join("a");
        let dir_b = tmp.path().join("b");
        fs::create_dir_all(&dir_a).unwrap();
        fs::create_dir_all(&dir_b).unwrap();
        let old = dir_a.join("note.md");
        let new = dir_b.join("note.md");
        fs::write(&old, "content").unwrap();
        fs::rename(&old, &new).unwrap();

        let mut op = FileOp::Move {
            old_path: old.clone(),
            new_path: new.clone(),
        };

        // Undo move: new -> old
        assert!(reverse_op(&mut op));
        assert!(old.exists());
        assert!(!new.exists());

        // Redo move: old -> new
        assert!(reapply_op(&mut op));
        assert!(!old.exists());
        assert!(new.exists());
    }

    #[test]
    fn test_push_undo_clears_redo_and_caps() {
        let mut undo = Vec::new();
        let mut redo = vec![FileOp::Create {
            path: PathBuf::from("/tmp/x"),
            is_dir: false,
        }];

        push_undo(
            &mut undo,
            &mut redo,
            FileOp::Create {
                path: PathBuf::from("/tmp/y"),
                is_dir: false,
            },
        );

        assert!(redo.is_empty());
        assert_eq!(undo.len(), 1);
    }

    #[test]
    fn test_undo_stack_capped_at_max() {
        let mut undo = Vec::new();
        let mut redo = Vec::new();

        for i in 0..60 {
            push_undo(
                &mut undo,
                &mut redo,
                FileOp::Create {
                    path: PathBuf::from(format!("/tmp/file{}", i)),
                    is_dir: false,
                },
            );
        }

        assert_eq!(undo.len(), MAX_UNDO_STACK);
    }

    #[test]
    fn test_undo_delete_restores_file() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("note.md");
        fs::write(&file, "my notes").unwrap();

        let backup = backup_file(&file);
        fs::remove_file(&file).unwrap();

        let mut op = FileOp::Delete {
            path: file.clone(),
            backup,
            is_dir: false,
        };

        // Undo delete = restore from backup
        assert!(reverse_op(&mut op));
        assert!(file.exists());
        assert_eq!(fs::read_to_string(&file).unwrap(), "my notes");

        // Redo delete = delete again
        assert!(reapply_op(&mut op));
        assert!(!file.exists());

        // Undo again = restore
        assert!(reverse_op(&mut op));
        assert!(file.exists());
        assert_eq!(fs::read_to_string(&file).unwrap(), "my notes");
    }

    #[test]
    fn test_undo_delete_batch() {
        let tmp = TempDir::new().unwrap();
        let f1 = tmp.path().join("a.md");
        let f2 = tmp.path().join("b.md");
        fs::write(&f1, "aaa").unwrap();
        fs::write(&f2, "bbb").unwrap();

        let b1 = backup_file(&f1);
        let b2 = backup_file(&f2);
        fs::remove_file(&f1).unwrap();
        fs::remove_file(&f2).unwrap();

        let mut op = FileOp::DeleteBatch {
            ops: vec![
                FileOp::Delete { path: f1.clone(), backup: b1, is_dir: false },
                FileOp::Delete { path: f2.clone(), backup: b2, is_dir: false },
            ],
        };

        assert!(reverse_op(&mut op));
        assert!(f1.exists());
        assert!(f2.exists());
        assert_eq!(fs::read_to_string(&f1).unwrap(), "aaa");
        assert_eq!(fs::read_to_string(&f2).unwrap(), "bbb");
    }
}
