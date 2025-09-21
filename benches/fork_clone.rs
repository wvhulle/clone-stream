use clone_stream::ForkStream;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use futures::{StreamExt, future::join_all, stream};
use tokio::runtime::Runtime;

/// Benchmarks basic fork and clone operations
fn basic_fork_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("basic_fork_creation", |b| {
        b.iter(|| {
            rt.block_on(async {
                let data: Vec<usize> = (0..100).collect();
                let stream = stream::iter(data);
                let _forked = black_box(stream.fork());
            });
        });
    });
}

/// Benchmarks clone creation from a forked stream
fn clone_creation_scaling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("clone_creation_scaling");

    for clone_count in &[1, 2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("clones", clone_count),
            clone_count,
            |b, &clone_count| {
                b.iter(|| {
                    rt.block_on(async {
                        let data: Vec<usize> = (0..50).collect();
                        let stream = stream::iter(data);
                        let forked = stream.fork();

                        let _clones: Vec<_> =
                            black_box((0..clone_count).map(|_| forked.clone()).collect());
                    });
                });
            },
        );
    }
    group.finish();
}

/// Benchmarks concurrent consumption with multiple clones
fn concurrent_consumption(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_consumption");

    for clone_count in &[2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("clones", clone_count),
            clone_count,
            |b, &clone_count| {
                b.iter(|| {
                    rt.block_on(async {
                        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<usize>();
                        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
                        let forked = stream.fork();

                        let clones: Vec<_> = (0..clone_count).map(|_| forked.clone()).collect();

                        let tasks: Vec<_> = clones
                            .into_iter()
                            .map(|mut clone| {
                                tokio::spawn(async move {
                                    let mut count = 0;
                                    while (clone.next().await).is_some() {
                                        count += 1;
                                    }
                                    count
                                })
                            })
                            .collect();

                        for i in 0..100 {
                            sender.send(i).unwrap();
                        }
                        drop(sender);

                        let _results = join_all(tasks).await;
                    });
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    fork_clone_benchmarks,
    basic_fork_creation,
    clone_creation_scaling,
    concurrent_consumption
);
criterion_main!(fork_clone_benchmarks);
