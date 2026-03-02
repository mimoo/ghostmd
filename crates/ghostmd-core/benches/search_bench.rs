use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ghostmd_core::search::{ContentSearch, FuzzySearch};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn create_files(root: &Path, count: usize, with_content: bool) {
    for i in 0..count {
        let dir = root.join(format!("dir_{:04}", i / 100));
        fs::create_dir_all(&dir).unwrap();
        let filename = match i % 5 {
            0 => format!("meeting_notes_{i:05}.md"),
            1 => format!("todo_list_{i:05}.md"),
            2 => format!("project_plan_{i:05}.md"),
            3 => format!("daily_journal_{i:05}.md"),
            _ => format!("reference_{i:05}.md"),
        };
        let content = if with_content {
            format!(
                "# Document {i}\n\nSome text here.\n{}\nMore text follows.\n",
                if i % 3 == 0 { "TODO: finish this section" } else { "This section is complete." }
            )
        } else {
            String::new()
        };
        fs::write(dir.join(filename), content).unwrap();
    }
}

fn bench_fuzzy_100(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    create_files(tmp.path(), 100, false);

    let mut search = FuzzySearch::new(tmp.path().to_path_buf());
    search.refresh_cache().unwrap();

    c.bench_function("fuzzy_search_100_files", |b| {
        b.iter(|| {
            black_box(search.search_files("meeting"));
        });
    });
}

fn bench_fuzzy_1000(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    create_files(tmp.path(), 1000, false);

    let mut search = FuzzySearch::new(tmp.path().to_path_buf());
    search.refresh_cache().unwrap();

    c.bench_function("fuzzy_search_1000_files", |b| {
        b.iter(|| {
            black_box(search.search_files("meeting"));
        });
    });
}

fn bench_fuzzy_10000(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    create_files(tmp.path(), 10000, false);

    let mut search = FuzzySearch::new(tmp.path().to_path_buf());
    search.refresh_cache().unwrap();

    c.bench_function("fuzzy_search_10000_files", |b| {
        b.iter(|| {
            black_box(search.search_files("meeting"));
        });
    });
}

fn bench_content_search_100(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    create_files(tmp.path(), 100, true);

    let searcher = ContentSearch::new(tmp.path().to_path_buf());

    c.bench_function("content_search_100_files", |b| {
        b.iter(|| {
            black_box(searcher.search("TODO").unwrap());
        });
    });
}

fn bench_content_search_1000(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    create_files(tmp.path(), 1000, true);

    let searcher = ContentSearch::new(tmp.path().to_path_buf());

    c.bench_function("content_search_1000_files", |b| {
        b.iter(|| {
            black_box(searcher.search("TODO").unwrap());
        });
    });
}

fn bench_refresh_cache_1000(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    create_files(tmp.path(), 1000, false);

    c.bench_function("refresh_cache_1000_files", |b| {
        b.iter(|| {
            let mut search = FuzzySearch::new(tmp.path().to_path_buf());
            search.refresh_cache().unwrap();
            black_box(&search);
        });
    });
}

fn bench_fuzzy_single_char(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    create_files(tmp.path(), 10000, false);

    let mut search = FuzzySearch::new(tmp.path().to_path_buf());
    search.refresh_cache().unwrap();

    c.bench_function("fuzzy_single_char_m_10000_files", |b| {
        b.iter(|| {
            black_box(search.search_files("m"));
        });
    });
}

fn bench_content_search_large_files(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    // Create 100 files of ~100KB each
    let dir = tmp.path().join("large");
    fs::create_dir_all(&dir).unwrap();
    for i in 0..100 {
        let mut content = String::with_capacity(100_000);
        for line in 0..2000 {
            if line == 1000 && i % 3 == 0 {
                content.push_str("TODO: fix this important issue\n");
            } else {
                content.push_str("Lorem ipsum dolor sit amet, consectetur adipiscing elit.\n");
            }
        }
        fs::write(dir.join(format!("large_file_{i:04}.md")), &content).unwrap();
    }

    let searcher = ContentSearch::new(tmp.path().to_path_buf());

    c.bench_function("content_search_100_files_100kb", |b| {
        b.iter(|| {
            black_box(searcher.search("TODO").unwrap());
        });
    });
}

criterion_group!(
    benches,
    bench_fuzzy_100,
    bench_fuzzy_1000,
    bench_fuzzy_10000,
    bench_content_search_100,
    bench_content_search_1000,
    bench_refresh_cache_1000,
    bench_fuzzy_single_char,
    bench_content_search_large_files,
);
criterion_main!(benches);
