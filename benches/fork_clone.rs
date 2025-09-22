use clone_stream::ForkStream;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use futures::{StreamExt, stream};

/// Benchmarks clone creation from a forked stream
fn clone_creation_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("clone_creation_scaling");

    for clone_count in &[1, 2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("clones", clone_count),
            clone_count,
            |b, &clone_count| {
                b.iter(|| {
                    let data: Vec<usize> = (0..50).collect();
                    let stream = stream::iter(data);
                    let forked = stream.fork();

                    let _clones: Vec<_> =
                        black_box((0..clone_count).map(|_| forked.clone()).collect());
                });
            },
        );
    }
    group.finish();
}

/// Benchmarks concurrent consumption with multiple clones
fn concurrent_consumption(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_consumption");

    for clone_count in &[2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("clones", clone_count),
            clone_count,
            |b, &clone_count| {
                b.iter(|| {
                    rt.block_on(async {
                        // Use a simple iterator stream to avoid async overhead
                        let data: Vec<usize> = (0..100).collect();
                        let stream = stream::iter(data);
                        let forked = stream.fork();

                        let clones: Vec<_> = (0..clone_count).map(|_| forked.clone()).collect();

                        // Concurrently consume all clones - this demonstrates the core clone-stream functionality
                        let tasks: Vec<_> = clones
                            .into_iter()
                            .map(|clone| {
                                tokio::spawn(async move {
                                    let collected: Vec<_> = clone.collect().await;
                                    collected.len() // Return count of items received
                                })
                            })
                            .collect();

                        // Wait for all concurrent consumers to complete
                        let results: Vec<_> = futures::future::join_all(tasks)
                            .await
                            .into_iter()
                            .map(|result| result.unwrap())
                            .collect();

                        black_box(results)
                    })
                });
            },
        );
    }
    group.finish();
}

/// Benchmarks clone stream overhead vs original stream
fn clone_overhead_comparison(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("clone_overhead");
    let data: Vec<usize> = (0..1000).collect();

    group.bench_function("original_stream", |b| {
        b.iter(|| {
            rt.block_on(async {
                let stream = stream::iter(data.clone());
                let count = stream.fold(0, |acc, _| async move { acc + 1 }).await;
                black_box(count)
            })
        });
    });

    group.bench_function("single_clone", |b| {
        b.iter(|| {
            rt.block_on(async {
                let stream = stream::iter(data.clone());
                let forked = stream.fork();
                let clone = forked.clone();
                let count = clone.fold(0, |acc, _| async move { acc + 1 }).await;
                black_box(count)
            })
        });
    });

    group.finish();
}
criterion_group!(
    fork_clone_benchmarks,
    clone_creation_scaling,
    concurrent_consumption,
    clone_overhead_comparison
);
criterion_main!(fork_clone_benchmarks);
