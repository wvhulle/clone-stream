use clone_stream::ForkStream;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use futures::{StreamExt, stream};

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

criterion_group!(comparative_benchmarks, clone_overhead_comparison);
criterion_main!(comparative_benchmarks);
