#![allow(clippy::needless_for_each, clippy::cast_precision_loss)]

mod common;

use common::{
    BenchmarkConfig, CLONE_COUNTS, PerformanceStats, Pipe, benchmark_configurations, test_items,
};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use futures::{StreamExt, future, stream};
use std::time::Duration;

/// Combined benchmark testing clone count x item count combinations
fn benchmark_item_throughput(c: &mut Criterion) {
    use std::cell::RefCell;

    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("Item throughput");

    // Configure benchmark group
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(5));
    group.warm_up_time(Duration::from_secs(1));

    let stats = RefCell::new(PerformanceStats::default());

    benchmark_configurations().for_each(|BenchmarkConfig { clones, items }| {
        let test_id = format!("{clones}clones_{items}items");

        group.bench_with_input(
            BenchmarkId::new("combination", &test_id),
            &(clones, items),
            |bencher, &(clones, items)| {
                let mut total_time = Duration::ZERO;
                let mut iterations = 0;

                bencher.iter(|| {
                    let start = std::time::Instant::now();

                    let result = rt.block_on(async move {
                        test_items(items)
                            .pipe(stream::iter)
                            .pipe(clone_stream::ForkStream::fork)
                            .pipe(|forked| {
                                (0..clones)
                                    .map(|_| forked.clone())
                                    .map(|clone| {
                                        tokio::spawn(async move {
                                            clone.collect::<Vec<_>>().await.len()
                                        })
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .pipe(future::join_all)
                            .await
                            .into_iter()
                            .map(Result::unwrap)
                            .sum::<usize>()
                            .pipe(black_box)
                    });

                    total_time += start.elapsed();
                    iterations += 1;
                    result
                });

                // Store results for summary
                if iterations > 0 {
                    let avg_time_ns = total_time.as_nanos() as f64 / f64::from(iterations);
                    let current_stats = stats.borrow().clone();
                    let updated_stats = current_stats.with_result(clones, items, avg_time_ns);
                    *stats.borrow_mut() = updated_stats;
                }
            },
        );
    });

    group.finish();
    println!();

    // Print statistics summary
    stats.borrow().print_summary();
}

fn benchmark_clone_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Clone creation");
    group.sample_size(15);
    group.measurement_time(Duration::from_secs(3));
    group.warm_up_time(Duration::from_secs(1));

    CLONE_COUNTS.iter().for_each(|&clone_count| {
        group.bench_with_input(
            BenchmarkId::new("clones", clone_count),
            &clone_count,
            |bencher, &count| {
                bencher.iter(|| {
                    test_items(50)
                        .pipe(stream::iter)
                        .pipe(clone_stream::ForkStream::fork)
                        .pipe(|forked| (0..count).map(|_| forked.clone()).collect::<Vec<_>>())
                        .pipe(black_box)
                });
            },
        );
    });

    group.finish();
}

criterion_group!(
    fork_clone_benchmarks,
    benchmark_item_throughput,
    benchmark_clone_creation
);
criterion_main!(fork_clone_benchmarks);
