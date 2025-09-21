use clone_stream::ForkStream;
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use futures::{StreamExt, stream, future::join_all};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

/// Benchmarks memory usage with large queues
fn large_queue_handling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("large_queue_handling");
    group.sample_size(10);
    
    for queue_size in &[100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::new("queue_items", queue_size),
            queue_size,
            |b, &queue_size| {
                b.iter(|| {
                    rt.block_on(async {
                        let (sender, receiver) = mpsc::unbounded_channel::<usize>();
                        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
                        let forked = stream.fork();
                        
                        // Create slow and fast consumers
                        let slow_clone = forked.clone();
                        let fast_clone = forked.clone();
                        
                        // Fast consumer
                        let fast_task = tokio::spawn(async move {
                            let mut fast_clone = fast_clone;
                            let mut count = 0;
                            while fast_clone.next().await.is_some() {
                                count += 1;
                            }
                            count
                        });
                        
                        // Slow consumer (creates queue buildup)
                        let slow_task = tokio::spawn(async move {
                            let mut slow_clone = slow_clone;
                            let mut count = 0;
                            while let Some(_) = slow_clone.next().await {
                                count += 1;
                                if count % 10 == 0 {
                                    sleep(Duration::from_millis(1)).await;
                                }
                            }
                            count
                        });
                        
                        // Producer
                        let producer = tokio::spawn(async move {
                            for i in 0..queue_size {
                                sender.send(i).unwrap();
                            }
                        });
                        
                        let _ = producer.await;
                        let _results = black_box(join_all([fast_task, slow_task]).await);
                    });
                })
            },
        );
    }
    group.finish();
}

/// Benchmarks memory cleanup after clone drops
fn clone_cleanup(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("clone_drop_cleanup", |b| {
        b.iter(|| {
            rt.block_on(async {
                let data: Vec<String> = (0..500)
                    .map(|i| format!("data_item_{}", i))
                    .collect();
                let stream = stream::iter(data);
                let forked = stream.fork();
                
                // Create many clones
                let clones: Vec<_> = (0..10)
                    .map(|_| forked.clone())
                    .collect();
                
                // Consume with some clones, drop others
                let mut tasks = Vec::new();
                
                for (idx, clone) in clones.into_iter().enumerate() {
                    if idx < 5 {
                        // Some clones consume data
                        let task = tokio::spawn(async move {
                            black_box(clone.collect::<Vec<_>>().await)
                        });
                        tasks.push(task);
                    }
                    // Other clones are just dropped (implicit cleanup)
                }
                
                let _results = black_box(join_all(tasks).await);
            });
        })
    });
}

/// Benchmarks memory efficiency with different clone patterns
fn memory_efficiency_patterns(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory_efficiency_patterns");
    group.sample_size(10);
    
    // Sequential clone creation and consumption
    group.bench_function("sequential_clones", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (sender, receiver) = mpsc::unbounded_channel::<usize>();
                let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
                let forked = stream.fork();
                
                // Producer
                let producer = tokio::spawn(async move {
                    for i in 0..200 {
                        sender.send(i).unwrap();
                        if i % 20 == 0 {
                            sleep(Duration::from_millis(1)).await;
                        }
                    }
                });
                
                // Sequential consumers (create, consume, drop)
                let mut results = Vec::new();
                for _ in 0..5 {
                    let mut clone = forked.clone();
                    let task = tokio::spawn(async move {
                        let mut count = 0;
                        for _ in 0..10 {
                            if clone.next().await.is_some() {
                                count += 1;
                            }
                        }
                        count
                    });
                    results.push(task.await.unwrap());
                }
                
                let _ = producer.await;
                black_box(results);
            });
        })
    });
    
    // Concurrent clone creation and consumption
    group.bench_function("concurrent_clones", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (sender, receiver) = mpsc::unbounded_channel::<usize>();
                let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
                let forked = stream.fork();
                
                // Create all clones at once
                let clones: Vec<_> = (0..5)
                    .map(|_| forked.clone())
                    .collect();
                
                // Producer
                let producer = tokio::spawn(async move {
                    for i in 0..200 {
                        sender.send(i).unwrap();
                    }
                });
                
                // Concurrent consumers
                let tasks: Vec<_> = clones
                    .into_iter()
                    .map(|mut clone| {
                        tokio::spawn(async move {
                            let mut count = 0;
                            while clone.next().await.is_some() {
                                count += 1;
                                if count >= 40 {
                                    break;
                                }
                            }
                            count
                        })
                    })
                    .collect();
                
                let _ = producer.await;
                let _results = black_box(join_all(tasks).await);
            });
        })
    });
    
    group.finish();
}

criterion_group!(
    memory_management_benchmarks,
    large_queue_handling,
    clone_cleanup,
    memory_efficiency_patterns
);
criterion_main!(memory_management_benchmarks);
