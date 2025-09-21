use clone_stream::ForkStream;
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use futures::{StreamExt, stream, future::join_all};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

/// Benchmarks performance scaling with different numbers of clones
fn clone_scaling_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("clone_scaling_performance");
    group.sample_size(10);
    
    for clone_count in &[2, 4, 8, 16, 32] {
        group.bench_with_input(
            BenchmarkId::new("clones", clone_count),
            clone_count,
            |b, &clone_count| {
                b.iter(|| {
                    rt.block_on(async {
                        let (sender, receiver) = mpsc::unbounded_channel::<usize>();
                        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
                        let forked = stream.fork();
                        
                        // Create clones
                        let clones: Vec<_> = (0..clone_count)
                            .map(|_| forked.clone())
                            .collect();
                        
                        // Spawn consumers
                        let tasks: Vec<_> = clones
                            .into_iter()
                            .map(|mut clone| {
                                tokio::spawn(async move {
                                    let mut count = 0;
                                    while clone.next().await.is_some() {
                                        count += 1;
                                    }
                                    count
                                })
                            })
                            .collect();
                        
                        // Send data
                        for i in 0..100 {
                            sender.send(i).unwrap();
                        }
                        drop(sender);
                        
                        // Wait for completion
                        let _results = black_box(join_all(tasks).await);
                    });
                })
            },
        );
    }
    group.finish();
}

/// Benchmarks throughput with different data sizes
fn throughput_scaling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("throughput_scaling");
    group.sample_size(10);
    
    for data_size in &[100, 500, 1000, 2000] {
        group.bench_with_input(
            BenchmarkId::new("items", data_size),
            data_size,
            |b, &data_size| {
                b.iter(|| {
                    rt.block_on(async {
                        let (sender, receiver) = mpsc::unbounded_channel::<usize>();
                        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
                        let forked = stream.fork();
                        
                        // Create fixed number of clones
                        let clone_count = 4;
                        let clones: Vec<_> = (0..clone_count)
                            .map(|_| forked.clone())
                            .collect();
                        
                        // Spawn consumers
                        let tasks: Vec<_> = clones
                            .into_iter()
                            .map(|mut clone| {
                                tokio::spawn(async move {
                                    let mut count = 0;
                                    while clone.next().await.is_some() {
                                        count += 1;
                                    }
                                    count
                                })
                            })
                            .collect();
                        
                        // Send data
                        for i in 0..data_size {
                            sender.send(i).unwrap();
                        }
                        drop(sender);
                        
                        // Wait for completion
                        let _results = black_box(join_all(tasks).await);
                    });
                })
            },
        );
    }
    group.finish();
}

/// Benchmarks performance with different types of data
fn data_type_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("data_type_performance");
    group.sample_size(10);
    
    // Small integers
    group.bench_function("small_integers", |b| {
        b.iter(|| {
            rt.block_on(async {
                let data: Vec<u32> = (0..200).collect();
                let stream = stream::iter(data);
                let forked = stream.fork();
                
                let clones: Vec<_> = (0..4).map(|_| forked.clone()).collect();
                let tasks: Vec<_> = clones
                    .into_iter()
                    .map(|clone| {
                        tokio::spawn(async move {
                            black_box(clone.collect::<Vec<_>>().await)
                        })
                    })
                    .collect();
                
                let _results = black_box(join_all(tasks).await);
            });
        })
    });
    
    // String data
    group.bench_function("string_data", |b| {
        b.iter(|| {
            rt.block_on(async {
                let data: Vec<String> = (0..200)
                    .map(|i| format!("item_{}", i))
                    .collect();
                let stream = stream::iter(data);
                let forked = stream.fork();
                
                let clones: Vec<_> = (0..4).map(|_| forked.clone()).collect();
                let tasks: Vec<_> = clones
                    .into_iter()
                    .map(|clone| {
                        tokio::spawn(async move {
                            black_box(clone.collect::<Vec<_>>().await)
                        })
                    })
                    .collect();
                
                let _results = black_box(join_all(tasks).await);
            });
        })
    });
    
    // Large structures
    group.bench_function("large_structures", |b| {
        b.iter(|| {
            rt.block_on(async {
                #[derive(Clone)]
                struct LargeData {
                    id: u64,
                    name: String,
                    data: Vec<u8>,
                }
                
                let data: Vec<LargeData> = (0..100)
                    .map(|i| LargeData {
                        id: i,
                        name: format!("item_{}", i),
                        data: vec![i as u8; 50],
                    })
                    .collect();
                let stream = stream::iter(data);
                let forked = stream.fork();
                
                let clones: Vec<_> = (0..4).map(|_| forked.clone()).collect();
                let tasks: Vec<_> = clones
                    .into_iter()
                    .map(|clone| {
                        tokio::spawn(async move {
                            black_box(clone.collect::<Vec<_>>().await)
                        })
                    })
                    .collect();
                
                let _results = black_box(join_all(tasks).await);
            });
        })
    });
    
    group.finish();
}

criterion_group!(
    performance_scaling_benchmarks,
    clone_scaling_performance,
    throughput_scaling,
    data_type_performance
);
criterion_main!(performance_scaling_benchmarks);
