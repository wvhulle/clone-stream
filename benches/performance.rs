//! # Clone Stream Performance Benchmarks
//!
//! ## Running
//! ```bash
//! cargo bench
//! ```
//!
//! ## Output
//! - Statistical analysis with confidence intervals
//! - HTML reports in `target/criterion/`
//! - Performance regression detection

use clone_stream::ForkStream;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use futures::{StreamExt, future::join_all, stream};
use tokio::runtime::Runtime;

fn single_clone_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("single_clone_1000_items", |b| {
        b.to_async(&rt).iter(|| async {
            let stream = stream::iter(0..1000);
            let items: Vec<_> = black_box(stream.fork().collect().await);
            black_box(items)
        });
    });
}

fn multiple_clones_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("multiple_clones");

    for clone_count in &[10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("clones_100_items", clone_count),
            clone_count,
            |b, &clone_count| {
                b.to_async(&rt).iter(|| async move {
                    let stream = stream::iter(0..100);
                    let template = stream.fork();

                    let tasks: Vec<_> = (0..clone_count)
                        .map(|_| {
                            let clone = template.clone();
                            tokio::spawn(async move { clone.collect::<Vec<_>>().await })
                        })
                        .collect();

                    let results = join_all(tasks).await;
                    let lengths: Vec<_> = results.into_iter().map(|r| r.unwrap().len()).collect();
                    black_box(lengths)
                });
            },
        );
    }
    group.finish();
}

fn late_clone_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("late_clone_creation", |b| {
        b.to_async(&rt).iter(|| async {
            let (tx, rx) = futures::channel::mpsc::unbounded::<i32>();
            let mut original = rx.fork();

            // Send first batch
            for i in 0..50 {
                tx.unbounded_send(i).unwrap();
            }

            // Read some from original
            let mut early_items = Vec::new();
            for _ in 0..25 {
                if let Some(item) = original.next().await {
                    early_items.push(item);
                }
            }

            // Create late clone
            let late_clone = original.clone();

            // Send second batch
            for i in 50..100 {
                tx.unbounded_send(i).unwrap();
            }
            drop(tx);

            // Collect remaining items
            let remaining: Vec<_> = original.collect().await;
            let late_items: Vec<_> = late_clone.collect().await;

            black_box((early_items, remaining, late_items))
        });
    });
}

fn stream_size_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("stream_sizes");

    for size in &[100, 1000, 10000] {
        group.bench_with_input(BenchmarkId::new("single_clone", size), size, |b, &size| {
            b.to_async(&rt).iter(|| async move {
                let stream = stream::iter(0..size);
                let items: Vec<_> = black_box(stream.fork().collect().await);
                black_box(items)
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    single_clone_benchmark,
    multiple_clones_benchmark,
    late_clone_benchmark,
    stream_size_benchmark
);
criterion_main!(benches);
