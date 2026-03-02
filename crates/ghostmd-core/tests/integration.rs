use std::fs;

use chrono::NaiveDate;
use tempfile::TempDir;

use ghostmd_core::buffer::UndoBuffer;
use ghostmd_core::diary;
use ghostmd_core::note::Note;
use ghostmd_core::search::{ContentSearch, FuzzySearch};
use ghostmd_core::tree::FileTree;

// ---------------------------------------------------------------------------
// 1. Full note lifecycle: diary path -> Note -> save -> FileTree -> search
// ---------------------------------------------------------------------------

#[test]
fn test_full_note_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Use the diary module to generate a path for today.
    let path = diary::new_diary_path(root, "Integration Test Note");

    // Create a Note at that path and ensure parent directories exist.
    let note = Note::new(path.clone());
    note.ensure_dir().unwrap();

    // Save some content.
    let content = "# Integration Test Note\n\nThis is body text for the lifecycle test.";
    note.save(content).unwrap();

    // Scan with FileTree and verify the note appears.
    let mut tree = FileTree::new(root.to_path_buf());
    tree.scan().unwrap();
    assert!(
        tree.file_count() >= 1,
        "FileTree should contain at least one file after saving a note"
    );
    assert!(
        tree.find_node(&path).is_some(),
        "FileTree should be able to find the note we just saved"
    );

    // FuzzySearch for it by (partial) title.
    let mut fuzzy = FuzzySearch::new(root.to_path_buf());
    fuzzy.refresh_cache().unwrap();
    let results = fuzzy.search_files("integration-test-note");
    assert!(
        !results.is_empty(),
        "FuzzySearch should find the note by its slugified title"
    );
    assert!(
        results
            .iter()
            .any(|r| r.path.to_string_lossy().contains("integration-test-note")),
        "At least one result path should contain the slug"
    );

    // ContentSearch for text within the note.
    let content_search = ContentSearch::new(root.to_path_buf());
    let matches = content_search.search("lifecycle test").unwrap();
    assert!(
        !matches.is_empty(),
        "ContentSearch should find 'lifecycle test' inside the note"
    );
    assert!(
        matches.iter().any(|m| m.path == path),
        "ContentSearch result should reference the note's path"
    );
}

// ---------------------------------------------------------------------------
// 2. Buffer save/load roundtrip
// ---------------------------------------------------------------------------

#[test]
fn test_buffer_save_load_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("roundtrip.md");

    // Create a buffer with initial text and make several edits.
    let mut buf = UndoBuffer::from_str("Hello World");
    buf.insert(5, ","); // "Hello, World"
    buf.delete(11..12); // remove trailing 'd' -> "Hello, Worl"
    buf.insert(11, "d!"); // "Hello, World!"

    let final_text = buf.text();

    // Save via Note.
    let note = Note::new(path.clone());
    note.save(&final_text).unwrap();

    // Load back and verify content matches.
    let (_, loaded_content) = Note::load(&path).unwrap();
    assert_eq!(
        loaded_content, final_text,
        "Loaded content should match the buffer state that was saved"
    );
}

// ---------------------------------------------------------------------------
// 3. Undo is independent of save (in-memory vs on-disk)
// ---------------------------------------------------------------------------

#[test]
fn test_undo_independent_of_save() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("undo-test.md");

    let mut buf = UndoBuffer::from_str("base");
    buf.insert(4, " edit1"); // "base edit1"

    // Save current buffer state to disk.
    let note = Note::new(path.clone());
    note.save(&buf.text()).unwrap();
    let saved_text = buf.text();

    // More edits after save.
    buf.insert(10, " edit2"); // "base edit1 edit2"

    // Undo past the save point.
    buf.undo(); // undo " edit2" -> "base edit1"
    buf.undo(); // undo " edit1" -> "base"

    let buffer_state = buf.text();

    // Buffer state should differ from what was saved on disk.
    assert_ne!(
        buffer_state, saved_text,
        "After undoing past the save point, buffer state should differ from the saved file"
    );

    // The file on disk should still hold the saved content.
    let (_, on_disk) = Note::load(&path).unwrap();
    assert_eq!(
        on_disk, saved_text,
        "The file on disk should still contain the originally saved content"
    );
}

// ---------------------------------------------------------------------------
// 4. Diary organization in tree
// ---------------------------------------------------------------------------

#[test]
fn test_diary_organization_in_tree() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Create diary paths for 3 different dates.
    let dates = [
        NaiveDate::from_ymd_opt(2024, 1, 10).unwrap(),
        NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
        NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
    ];

    for (i, date) in dates.iter().enumerate() {
        let dir = diary::diary_dir(root, *date);
        fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join(format!("note-{}.md", i));
        let note = Note::new(file_path);
        note.save(&format!("# Diary entry {}", i)).unwrap();
    }

    // Scan the tree and verify structure.
    let mut tree = FileTree::new(root.to_path_buf());
    tree.scan().unwrap();

    assert_eq!(
        tree.file_count(),
        3,
        "FileTree should contain exactly 3 diary notes"
    );

    // The diary directory itself should be in the tree.
    let diary_root = root.join("diary");
    assert!(
        tree.find_node(&diary_root).is_some(),
        "FileTree should contain the diary directory"
    );

    // Verify each file exists in the tree.
    for (i, date) in dates.iter().enumerate() {
        let dir = diary::diary_dir(root, *date);
        let file_path = dir.join(format!("note-{}.md", i));
        assert!(
            tree.find_node(&file_path).is_some(),
            "FileTree should contain diary note at {:?}",
            file_path
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Large note roundtrip
// ---------------------------------------------------------------------------

#[test]
fn test_large_note_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("large-note.md");

    // Create a ~1 MB string.
    let chunk = "abcdefghij"; // 10 bytes
    let large_content: String = chunk.repeat(100_000); // 1,000,000 bytes = ~1 MB
    assert!(large_content.len() >= 1_000_000);

    // Save and load.
    let note = Note::new(path.clone());
    note.save(&large_content).unwrap();

    let (_, loaded) = Note::load(&path).unwrap();
    assert_eq!(
        loaded, large_content,
        "Large note content should survive a save/load roundtrip"
    );

    // FuzzySearch should find the file by name.
    let mut fuzzy = FuzzySearch::new(tmp.path().to_path_buf());
    fuzzy.refresh_cache().unwrap();
    let results = fuzzy.search_files("large-note");
    assert!(
        !results.is_empty(),
        "FuzzySearch should find the large note file"
    );
    assert!(
        results
            .iter()
            .any(|r| r.path.to_string_lossy().contains("large-note")),
        "At least one FuzzySearch result should reference 'large-note'"
    );
}

// ---------------------------------------------------------------------------
// 6. Many notes search
// ---------------------------------------------------------------------------

#[test]
fn test_many_notes_search() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Create 50 notes with varied titles. Half contain a special marker.
    let marker = "XYZZY_MARKER";
    for i in 0..50 {
        let filename = format!("note-{:03}.md", i);
        let body = if i % 2 == 0 {
            format!("# Note {}\n\nThis note contains the {} string.", i, marker)
        } else {
            format!("# Note {}\n\nThis note has ordinary content.", i)
        };
        fs::write(root.join(&filename), &body).unwrap();
    }

    // FuzzySearch: refresh and verify cache count.
    let mut fuzzy = FuzzySearch::new(root.to_path_buf());
    fuzzy.refresh_cache().unwrap();
    assert_eq!(
        fuzzy.cached_count(),
        50,
        "FuzzySearch cache should contain all 50 notes"
    );

    // Search for a specific note by number.
    let results = fuzzy.search_files("note-042");
    assert!(
        !results.is_empty(),
        "FuzzySearch should find note-042"
    );
    assert!(
        results
            .iter()
            .any(|r| r.path.to_string_lossy().contains("note-042")),
        "At least one result should be note-042"
    );

    // ContentSearch: search for the marker that appears in ~half the files.
    let content_search = ContentSearch::new(root.to_path_buf());
    let matches = content_search.search(marker).unwrap();

    // Even-numbered notes (0, 2, 4, ..., 48) = 25 notes contain the marker.
    assert_eq!(
        matches.len(),
        25,
        "ContentSearch should find the marker in exactly 25 of 50 notes"
    );
}

// ---------------------------------------------------------------------------
// 7. Unicode roundtrip
// ---------------------------------------------------------------------------

#[test]
fn test_unicode_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Create a note with a Unicode title via diary slugify.
    let title = "Meeting Notes: Cafe Rencontre";
    let slug = diary::slugify(title);
    let path = root.join(format!("{}.md", slug));

    let unicode_content = concat!(
        "# Unicode Test\n\n",
        "Chinese: \u{4f60}\u{597d}\u{4e16}\u{754c}\n",
        "Japanese: \u{3053}\u{3093}\u{306b}\u{3061}\u{306f}\n",
        "Korean: \u{c548}\u{b155}\u{d558}\u{c138}\u{c694}\n",
        "Emoji: \u{1f600}\u{1f680}\u{2764}\u{fe0f}\u{1f4dd}\n",
        "Accented: caf\u{e9} na\u{ef}ve r\u{e9}sum\u{e9} \u{fc}ber\n",
        "Math: \u{2200}x \u{2208} \u{211d}, x\u{b2} \u{2265} 0\n",
    );

    let note = Note::new(path.clone());
    note.save(unicode_content).unwrap();

    let (loaded_note, loaded_content) = Note::load(&path).unwrap();
    assert_eq!(
        loaded_content, unicode_content,
        "Unicode content should survive a save/load roundtrip"
    );
    assert_eq!(
        loaded_note.path, path,
        "Loaded note path should match the original"
    );
}

// ---------------------------------------------------------------------------
// 8. Tree reflects filesystem changes
// ---------------------------------------------------------------------------

#[test]
fn test_tree_reflects_filesystem_changes() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    // Start with two files.
    fs::write(root.join("alpha.md"), "alpha").unwrap();
    fs::write(root.join("beta.md"), "beta").unwrap();

    let mut tree = FileTree::new(root.to_path_buf());
    tree.scan().unwrap();
    let initial_count = tree.file_count();
    assert_eq!(initial_count, 2, "Initial tree should have 2 files");

    // Create new files directly on the filesystem.
    fs::write(root.join("gamma.md"), "gamma").unwrap();
    fs::create_dir_all(root.join("subdir")).unwrap();
    fs::write(root.join("subdir/delta.md"), "delta").unwrap();

    // Re-scan and verify new files appear.
    tree.scan().unwrap();
    let new_count = tree.file_count();
    assert_eq!(new_count, 4, "After adding 2 files, tree should have 4 files");
    assert!(
        new_count > initial_count,
        "File count should have increased after adding files"
    );

    // Verify specific new files are found.
    assert!(
        tree.find_node(&root.join("gamma.md")).is_some(),
        "gamma.md should be in the tree after re-scan"
    );
    assert!(
        tree.find_node(&root.join("subdir/delta.md")).is_some(),
        "subdir/delta.md should be in the tree after re-scan"
    );
}
