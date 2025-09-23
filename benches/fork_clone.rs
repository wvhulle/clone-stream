#![allow(clippy::needless_for_each, clippy::cast_precision_loss)]

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use futures::{StreamExt, future, stream};
use std::collections::HashMap;
use std::time::Duration;

// Reduced for faster benchmarks
const CLONE_COUNTS: &[usize] = &[1, 2, 4, 8];
const ITEM_COUNTS: &[usize] = &[10, 50, 100];

/// Performance statistics collector
#[derive(Default, Clone)]
struct PerformanceStats {
    results: HashMap<(usize, usize), f64>, // (clones, items) -> avg_time_ns
}

impl PerformanceStats {
    /// Create new stats with added result
    fn with_result(mut self, clones: usize, items: usize, time_ns: f64) -> Self {
        self.results.insert((clones, items), time_ns);
        self
    }
    fn generate_summary(&self) -> Vec<String> {
        let header = vec![
            "\n=== Performance Summary (clones x items) ===".to_string(),
            "Format: clones x items: avg_time | throughput".to_string(),
        ];

        let item_sections = ITEM_COUNTS
            .iter()
            .flat_map(|&items| {
                let section_header = format!("\n{items} items:");
                let clone_results = CLONE_COUNTS
                    .iter()
                    .filter_map(|&clones| {
                        self.results.get(&(clones, items)).map(|&time_ns| {
                            let total_ops = clones * items;
                            let ops_per_sec = (total_ops as f64 * 1_000_000_000.0) / time_ns;
                            format!(
                                "  {clones}x{items}: {:.2}μs | {:.1}K ops/sec",
                                time_ns / 1000.0,
                                ops_per_sec / 1000.0
                            )
                        })
                    })
                    .collect::<Vec<_>>();

                std::iter::once(section_header).chain(clone_results)
            })
            .collect::<Vec<_>>();

        let best_throughput = self
            .find_best_throughput()
            .map(|(clones, items)| format!("\nBest throughput: {clones}x{items} clonesxitems"))
            .into_iter()
            .collect::<Vec<_>>();

        header
            .into_iter()
            .chain(item_sections)
            .chain(best_throughput)
            .collect()
    }

    /// Find best throughput combination
    fn find_best_throughput(&self) -> Option<(usize, usize)> {
        self.results
            .iter()
            .max_by(|a, b| {
                let throughput_a = (a.0.0 * a.0.1) as f64 / a.1;
                let throughput_b = (b.0.0 * b.0.1) as f64 / b.1;
                throughput_a.partial_cmp(&throughput_b).unwrap()
            })
            .map(|((clones, items), _)| (*clones, *items))
    }

    fn print_summary(&self) {
        self.generate_summary()
            .into_iter()
            .for_each(|line| println!("{line}"));
    }
}

const fn create_test_data(size: usize) -> std::ops::Range<usize> {
    0..size
}

/// Benchmark configuration as immutable data
type BenchmarkConfig = (usize, usize); // (clones, items)

/// Generate all benchmark configurations
fn benchmark_configurations() -> impl Iterator<Item = BenchmarkConfig> {
    CLONE_COUNTS
        .iter()
        .flat_map(|&clones| ITEM_COUNTS.iter().map(move |&items| (clones, items)))
}

/// Functional pipeline trait for better composition
trait Pipe: Sized {
    fn pipe<F, T>(self, f: F) -> T
    where
        F: FnOnce(Self) -> T,
    {
        f(self)
    }
}

impl<T> Pipe for T {}

/// Combined benchmark testing clone count x item count combinations
fn benchmark_clone_scaling(c: &mut Criterion) {
    use std::cell::RefCell;

    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("Clone Scaling");

    // Configure benchmark group
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(5));
    group.warm_up_time(Duration::from_secs(1));

    println!("Testing clone scaling across different combinations...");

    // Local stats collection
    let stats = RefCell::new(PerformanceStats::default());

    benchmark_configurations().for_each(|(clones, items)| {
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
                        create_test_data(items)
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

/// Quick fanout test for clone creation performance
fn benchmark_fanout_quick(c: &mut Criterion) {
    let mut group = c.benchmark_group("Fanout Quick");
    group.sample_size(15);
    group.measurement_time(Duration::from_secs(3));
    group.warm_up_time(Duration::from_secs(1));

    println!("Quick fanout test:");

    CLONE_COUNTS.iter().for_each(|&clone_count| {
        group.bench_with_input(
            BenchmarkId::new("clones", clone_count),
            &clone_count,
            |bencher, &count| {
                bencher.iter(|| {
                    create_test_data(50)
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
    benchmark_clone_scaling,
    benchmark_fanout_quick
);
criterion_main!(fork_clone_benchmarks);
