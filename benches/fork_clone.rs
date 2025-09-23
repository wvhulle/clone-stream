use clone_stream::ForkStream;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use futures::{StreamExt, future, stream};

const BENCHMARK_DATA_SIZE: usize = 50;
const CLONE_COUNTS: &[usize] = &[1, 2, 4, 8, 16];
const CONCURRENT_COUNTS: &[usize] = &[2, 4, 8];

/// Configuration for benchmark scenarios
struct BenchmarkConfig {
    data_size: usize,
    clone_counts: &'static [usize],
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            data_size: BENCHMARK_DATA_SIZE,
            clone_counts: CLONE_COUNTS,
        }
    }
}

/// Creates test data for benchmarks using functional approach
fn create_test_data(size: usize) -> impl Iterator<Item = usize> {
    0..size
}

/// Benchmarks clone creation performance scaling with number of clones
fn benchmark_clone_creation_scaling(c: &mut Criterion) {
    let config = BenchmarkConfig::default();
    
    let mut group = c.benchmark_group("clone_creation_scaling");
    group.sample_size(100);

    config.clone_counts.iter().for_each(|&clone_count| {
        group.bench_with_input(
            BenchmarkId::new("clones", clone_count),
            &clone_count,
            |bencher, &count| {
                bencher.iter(|| {
                    let data = create_test_data(config.data_size);
                    let stream = stream::iter(data);
                    let forked = stream.fork();

                    black_box(
                        (0..count)
                            .map(|_| forked.clone())
                            .collect::<Vec<_>>()
                    )
                })
            }
        );
    });

    group.finish();
}

/// Benchmarks concurrent consumption performance with multiple clones
fn benchmark_concurrent_consumption(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = BenchmarkConfig::default();
    
    let mut group = c.benchmark_group("concurrent_consumption");
    group.sample_size(20);

    CONCURRENT_COUNTS.iter().for_each(|&clone_count| {
        group.bench_with_input(
            BenchmarkId::new("clones", clone_count),
            &clone_count,
            |bencher, &count| {
                bencher.iter(|| {
                    rt.block_on(async {
                        let data = create_test_data(config.data_size);
                        let stream = stream::iter(data);
                        let forked = stream.fork();

                        let results = (0..count)
                            .map(|_| forked.clone())
                            .map(|clone| tokio::spawn(async move {
                                clone.collect::<Vec<_>>().await.len()
                            }))
                            .collect::<Vec<_>>();

                        black_box(
                            future::join_all(results)
                                .await
                                .into_iter()
                                .map(Result::unwrap)
                                .collect::<Vec<_>>()
                        )
                    })
                })
            }
        );
    });

    group.finish();
}

/// Benchmarks memory efficiency under different usage patterns
fn benchmark_memory_patterns(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_patterns");

    let scenarios = [
        ("sequential_consumption", false),
        ("parallel_consumption", true),
    ];

    scenarios.iter().for_each(|(name, parallel)| {
        group.bench_function(*name, |bencher| {
            bencher.iter(|| {
                rt.block_on(async {
                    let data = create_test_data(100);
                    let stream = stream::iter(data);
                    let forked = stream.fork();

                    let clones = (0..4)
                        .map(|_| forked.clone())
                        .collect::<Vec<_>>();

                    let results = if *parallel {
                        // Parallel consumption - tests concurrent queue management
                        future::join_all(
                            clones.into_iter().map(|clone| {
                                tokio::spawn(async move { clone.count().await })
                            })
                        ).await.into_iter().map(Result::unwrap).collect()
                    } else {
                        // Sequential consumption - tests queue cleanup
                        let mut results = Vec::new();
                        for clone in clones {
                            results.push(clone.count().await);
                        }
                        results
                    };

                    black_box(results)
                })
            })
        });
    });

    group.finish();
}

/// Benchmarks throughput with varying data sizes
fn benchmark_throughput_scaling(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("throughput_scaling");
    
    let data_sizes = [10, 100, 1000];

    data_sizes.iter().for_each(|&size| {
        group.bench_with_input(
            BenchmarkId::new("items", size),
            &size,
            |bencher, &item_count| {
                bencher.iter(|| {
                    rt.block_on(async {
                        let data = create_test_data(item_count);
                        let stream = stream::iter(data);
                        let forked = stream.fork();

                        let results = (0..4)
                            .map(|_| forked.clone())
                            .map(|clone| tokio::spawn(async move {
                                clone.fold(0usize, |acc, _| async move { acc + 1 }).await
                            }))
                            .collect::<Vec<_>>();

                        black_box(
                            future::join_all(results)
                                .await
                                .into_iter()
                                .map(Result::unwrap)
                                .sum::<usize>()
                        )
                    })
                })
            }
        );
    });

    group.finish();
}

criterion_group!(
    fork_clone_benchmarks,
    benchmark_clone_creation_scaling,
    benchmark_concurrent_consumption,
    benchmark_memory_patterns,
    benchmark_throughput_scaling
);
criterion_main!(fork_clone_benchmarks);