use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ghostmd_core::buffer::UndoBuffer;
use ropey::Rope;

fn bench_insert_char(c: &mut Criterion) {
    c.bench_function("insert_char_100kb", |b| {
        b.iter_batched(
            || UndoBuffer::from_str(&"a".repeat(100_000)),
            |mut buf| {
                buf.insert(black_box(50_000), "x");
                buf
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_insert_word(c: &mut Criterion) {
    c.bench_function("insert_word_100kb", |b| {
        b.iter_batched(
            || UndoBuffer::from_str(&"a".repeat(100_000)),
            |mut buf| {
                buf.insert(black_box(50_000), "helloworld");
                buf
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_clone_rope(c: &mut Criterion) {
    let rope = Rope::from("a".repeat(100_000).as_str());
    c.bench_function("clone_rope_100kb", |b| {
        b.iter(|| {
            black_box(rope.clone());
        });
    });
}

fn bench_1000_edits_then_undo_all(c: &mut Criterion) {
    c.bench_function("1000_edits_then_undo_all", |b| {
        b.iter_batched(
            || {
                let mut buf = UndoBuffer::from_str("start");
                for i in 0..1000 {
                    buf.insert(i % 5, "x");
                }
                buf
            },
            |mut buf| {
                while buf.undo() {}
                buf
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_snapshot_after_edits(c: &mut Criterion) {
    c.bench_function("snapshot_after_1000_edits", |b| {
        b.iter_batched(
            || {
                let mut buf = UndoBuffer::from_str("start");
                for i in 0..1000 {
                    buf.insert(i % 5, "x");
                }
                buf
            },
            |buf| {
                black_box(buf.text());
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_insert_unicode(c: &mut Criterion) {
    c.bench_function("insert_unicode_100kb", |b| {
        b.iter_batched(
            || UndoBuffer::from_str(&"a".repeat(100_000)),
            |mut buf| {
                buf.insert(black_box(50_000), "こんにちは");
                buf
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_large_delete(c: &mut Criterion) {
    c.bench_function("delete_50kb_from_100kb", |b| {
        b.iter_batched(
            || UndoBuffer::from_str(&"a".repeat(100_000)),
            |mut buf| {
                buf.delete(black_box(25_000..75_000));
                buf
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_undo_redo_cycle(c: &mut Criterion) {
    c.bench_function("undo_redo_cycle_100_edits", |b| {
        b.iter_batched(
            || {
                let mut buf = UndoBuffer::from_str("start");
                for i in 0..100 {
                    buf.insert(i % 5, "x");
                }
                buf
            },
            |mut buf| {
                // Undo all edits
                while buf.undo() {}
                // Redo all edits
                while buf.redo() {}
                buf
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_insert_char,
    bench_insert_word,
    bench_clone_rope,
    bench_1000_edits_then_undo_all,
    bench_snapshot_after_edits,
    bench_insert_unicode,
    bench_large_delete,
    bench_undo_redo_cycle,
);
criterion_main!(benches);
