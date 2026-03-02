use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ghostmd_core::tree::FileTree;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn create_flat_structure(root: &Path, num_dirs: usize, files_per_dir: usize) {
    for d in 0..num_dirs {
        let dir = root.join(format!("dir_{d:04}"));
        fs::create_dir_all(&dir).unwrap();
        for f in 0..files_per_dir {
            fs::write(dir.join(format!("file_{f:04}.md")), "content").unwrap();
        }
    }
}

fn create_deep_structure(root: &Path, depth: usize) {
    let mut current = root.to_path_buf();
    for level in 0..depth {
        current = current.join(format!("level_{level}"));
        fs::create_dir_all(&current).unwrap();
        fs::write(current.join("file.md"), "content").unwrap();
    }
}

fn bench_scan_100_files(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    create_flat_structure(tmp.path(), 10, 10);

    c.bench_function("scan_100_files", |b| {
        b.iter(|| {
            let mut tree = FileTree::new(tmp.path().to_path_buf());
            tree.scan().unwrap();
            black_box(&tree);
        });
    });
}

fn bench_scan_1000_files(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    create_flat_structure(tmp.path(), 100, 10);

    c.bench_function("scan_1000_files", |b| {
        b.iter(|| {
            let mut tree = FileTree::new(tmp.path().to_path_buf());
            tree.scan().unwrap();
            black_box(&tree);
        });
    });
}

fn bench_scan_10000_files(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    create_flat_structure(tmp.path(), 1000, 10);

    c.bench_function("scan_10000_files", |b| {
        b.iter(|| {
            let mut tree = FileTree::new(tmp.path().to_path_buf());
            tree.scan().unwrap();
            black_box(&tree);
        });
    });
}

fn bench_flatten_1000(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    create_flat_structure(tmp.path(), 100, 10);

    let mut tree = FileTree::new(tmp.path().to_path_buf());
    tree.scan().unwrap();

    c.bench_function("flatten_1000_files", |b| {
        b.iter(|| {
            black_box(tree.flatten());
        });
    });
}

fn bench_find_node_deep(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    create_deep_structure(tmp.path(), 10);

    let mut tree = FileTree::new(tmp.path().to_path_buf());
    tree.scan().unwrap();

    // Build the path to the deepest file
    let mut deepest = tmp.path().to_path_buf();
    for level in 0..10 {
        deepest = deepest.join(format!("level_{level}"));
    }
    deepest = deepest.join("file.md");

    c.bench_function("find_node_10_levels_deep", |b| {
        b.iter(|| {
            black_box(tree.find_node(&deepest));
        });
    });
}

fn bench_toggle_all_dirs(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    create_flat_structure(tmp.path(), 100, 10); // 100 dirs, 1000 files total

    let mut tree = FileTree::new(tmp.path().to_path_buf());
    tree.scan().unwrap();

    // Collect all directory paths
    let dir_paths: Vec<std::path::PathBuf> = (0..100)
        .map(|d| tmp.path().join(format!("dir_{d:04}")))
        .collect();

    c.bench_function("toggle_all_dirs_1000_files", |b| {
        b.iter(|| {
            for dir in &dir_paths {
                black_box(tree.toggle_dir(dir));
            }
            // Toggle back to restore state
            for dir in &dir_paths {
                black_box(tree.toggle_dir(dir));
            }
        });
    });
}

fn bench_file_count(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    create_flat_structure(tmp.path(), 1000, 10); // 10000 files

    let mut tree = FileTree::new(tmp.path().to_path_buf());
    tree.scan().unwrap();

    c.bench_function("file_count_10000_files", |b| {
        b.iter(|| {
            black_box(tree.file_count());
        });
    });
}

criterion_group!(
    benches,
    bench_scan_100_files,
    bench_scan_1000_files,
    bench_scan_10000_files,
    bench_flatten_1000,
    bench_find_node_deep,
    bench_toggle_all_dirs,
    bench_file_count,
);
criterion_main!(benches);
