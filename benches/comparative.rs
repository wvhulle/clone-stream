use clone_stream::ForkStream;
use criterion::{Criterion, black_box, criterion_group, criterion_main, BenchmarkId};
use futures::{StreamExt, stream};

const COMPARATIVE_DATA_SIZE: usize = 1000;

/// Test scenarios for overhead comparison
#[derive(Clone, Copy)]
enum ComparisonScenario {
    Baseline,
    SingleClone,
    MultipleClones(usize),
}

impl ComparisonScenario {
    fn name(&self) -> String {
        match self {
            Self::Baseline => "baseline_stream".to_string(),
            Self::SingleClone => "single_clone".to_string(), 
            Self::MultipleClones(n) => format!("multiple_clones_{}", n),
        }
    }
}

/// Creates test data iterator for consistent benchmarking
fn create_benchmark_data() -> impl Iterator<Item = usize> + Clone {
    0..COMPARATIVE_DATA_SIZE
}

/// Functional approach to measure stream processing overhead
async fn measure_stream_processing(scenario: ComparisonScenario) -> usize {
    let data = create_benchmark_data();
    
    match scenario {
        ComparisonScenario::Baseline => {
            stream::iter(data)
                .fold(0, |acc, _| async move { acc + 1 })
                .await
        },
        ComparisonScenario::SingleClone => {
            stream::iter(data)
                .fork()
                .fold(0, |acc, _| async move { acc + 1 })
                .await
        },
        ComparisonScenario::MultipleClones(clone_count) => {
            let forked = stream::iter(data).fork();
            
            // Create clones and consume them all
            let clones = (0..clone_count)
                .map(|_| forked.clone())
                .collect::<Vec<_>>();
            
            // Sum results from all clones
            let mut total = 0;
            for clone in clones {
                total += clone.fold(0, |acc, _| async move { acc + 1 }).await;
            }
            total
        }
    }
}

/// Benchmarks overhead comparison between original streams and cloned streams
fn benchmark_overhead_comparison(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("overhead_comparison");

    let scenarios = [
        ComparisonScenario::Baseline,
        ComparisonScenario::SingleClone,
        ComparisonScenario::MultipleClones(2),
        ComparisonScenario::MultipleClones(4),
    ];

    scenarios.iter().for_each(|&scenario| {
        group.bench_function(&scenario.name(), |bencher| {
            bencher.iter(|| {
                rt.block_on(async {
                    black_box(measure_stream_processing(scenario).await)
                })
            })
        });
    });

    group.finish();
}

/// Benchmarks memory allocation patterns
fn benchmark_allocation_patterns(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("allocation_patterns");

    let test_cases = [
        ("immediate_consumption", true),
        ("delayed_consumption", false),
    ];

    test_cases.iter().for_each(|(name, immediate)| {
        group.bench_function(*name, |bencher| {
            bencher.iter(|| {
                rt.block_on(async {
                    let data = create_benchmark_data();
                    let forked = stream::iter(data).fork();
                    
                    let clones = (0..3)
                        .map(|_| forked.clone())
                        .collect::<Vec<_>>();

                    if *immediate {
                        // Immediate consumption - minimal memory usage
                        let results = clones.into_iter()
                            .map(|clone| async move { clone.count().await })
                            .collect::<Vec<_>>();
                        
                        black_box(
                            futures::future::join_all(results)
                                .await
                                .into_iter()
                                .sum::<usize>()
                        )
                    } else {
                        // Delayed consumption - tests queue buildup
                        let first_clone = clones[0].clone();
                        let remaining_clones = &clones[1..];
                        
                        // First clone consumes immediately
                        let first_result = first_clone.count().await;
                        
                        // Remaining clones consume after delay
                        let remaining_results = remaining_clones.iter()
                            .map(|clone| clone.clone().count())
                            .collect::<Vec<_>>();
                        
                        let remaining_sum = futures::future::join_all(remaining_results)
                            .await
                            .into_iter()
                            .sum::<usize>();
                        
                        black_box(first_result + remaining_sum)
                    }
                })
            })
        });
    });

    group.finish();
}

/// Benchmarks throughput scaling under different clone configurations
fn benchmark_throughput_comparison(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("throughput_comparison");

    let clone_counts = [1, 2, 4, 8];

    clone_counts.iter().for_each(|&count| {
        group.bench_with_input(
            BenchmarkId::new("concurrent_clones", count),
            &count,
            |bencher, &clone_count| {
                bencher.iter(|| {
                    rt.block_on(async {
                        let data = create_benchmark_data();
                        let forked = stream::iter(data).fork();

                        let results = (0..clone_count)
                            .map(|_| forked.clone())
                            .map(|clone| async move {
                                clone.fold(0usize, |acc, _| async move { acc + 1 }).await
                            })
                            .collect::<Vec<_>>();

                        black_box(
                            futures::future::join_all(results)
                                .await
                                .into_iter()
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
    comparative_benchmarks,
    benchmark_overhead_comparison,
    benchmark_allocation_patterns,
    benchmark_throughput_comparison
);
criterion_main!(comparative_benchmarks);