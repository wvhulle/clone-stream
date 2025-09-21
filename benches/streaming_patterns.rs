use clone_stream::ForkStream;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use futures::{StreamExt, future::join_all};
use tokio::{
    runtime::Runtime,
    sync::mpsc,
    time::{Duration, interval, sleep},
};

/// Benchmarks sustained streaming with rate limiting
fn sustained_streaming(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("sustained_streaming");
    group.sample_size(10);

    for items_per_batch in &[50, 100, 200] {
        group.bench_with_input(
            BenchmarkId::new("items", items_per_batch),
            items_per_batch,
            |b, &items_per_batch| {
                b.iter(|| {
                    rt.block_on(async {
                        let (sender, receiver) = mpsc::unbounded_channel::<usize>();
                        let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
                        let forked = stream.fork();

                        // Create consumers
                        let clone_count = 3;
                        let clones: Vec<_> = (0..clone_count).map(|_| forked.clone()).collect();

                        // Producer task
                        let producer = tokio::spawn(async move {
                            let mut interval = interval(Duration::from_millis(1));
                            for i in 0..items_per_batch {
                                interval.tick().await;
                                if sender.send(i).is_err() {
                                    break;
                                }
                            }
                        });

                        // Consumer tasks
                        let consumers: Vec<_> = clones
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

                        // Wait for completion
                        let _ = producer.await;
                        let _results = black_box(join_all(consumers).await);
                    });
                });
            },
        );
    }
    group.finish();
}

/// Benchmarks bursty traffic patterns
fn bursty_traffic(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("bursty_traffic");
    group.sample_size(10);

    group.bench_function("burst_handling", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (sender, receiver) = mpsc::unbounded_channel::<String>();
                let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
                let forked = stream.fork();

                // Create mixed-speed consumers
                let consumers: Vec<_> = (0..4)
                    .map(|idx| {
                        let mut clone = forked.clone();
                        let is_slow = idx % 2 == 1;
                        tokio::spawn(async move {
                            let mut count = 0;
                            while let Some(_item) = clone.next().await {
                                count += 1;
                                if is_slow && count % 5 == 0 {
                                    sleep(Duration::from_millis(1)).await;
                                }
                            }
                            count
                        })
                    })
                    .collect();

                // Send bursts
                let producer = tokio::spawn(async move {
                    for burst in 0..5 {
                        // Send burst
                        for item in 0..20 {
                            let data = format!("burst-{burst}-item-{item}");
                            if sender.send(data).is_err() {
                                break;
                            }
                        }
                        // Pause between bursts
                        sleep(Duration::from_millis(2)).await;
                    }
                });

                let _ = producer.await;
                let _results = black_box(join_all(consumers).await);
            });
        });
    });

    group.finish();
}

/// Benchmarks late clone joining behavior
fn late_clone_joining(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("late_joining", |b| {
        b.iter(|| {
            rt.block_on(async {
                let (sender, receiver) = mpsc::unbounded_channel::<u64>();
                let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);
                let forked = stream.fork();

                // Early consumers
                let early_consumers: Vec<_> = (0..2)
                    .map(|_| {
                        let mut clone = forked.clone();
                        tokio::spawn(async move {
                            let mut count = 0;
                            while clone.next().await.is_some() {
                                count += 1;
                            }
                            count
                        })
                    })
                    .collect();

                // Producer
                let producer = tokio::spawn(async move {
                    for i in 0..100 {
                        if sender.send(i).is_err() {
                            break;
                        }
                        if i == 30 {
                            // Small delay to simulate late joining
                            sleep(Duration::from_millis(1)).await;
                        }
                    }
                });

                // Late consumers (after some delay)
                sleep(Duration::from_millis(1)).await;
                let late_consumers: Vec<_> = (0..2)
                    .map(|_| {
                        let mut clone = forked.clone();
                        tokio::spawn(async move {
                            let mut count = 0;
                            while clone.next().await.is_some() {
                                count += 1;
                            }
                            count
                        })
                    })
                    .collect();

                let _ = producer.await;
                let mut all_consumers = early_consumers;
                all_consumers.extend(late_consumers);
                let _results = black_box(join_all(all_consumers).await);
            });
        });
    });
}

criterion_group!(
    streaming_pattern_benchmarks,
    sustained_streaming,
    bursty_traffic,
    late_clone_joining
);
criterion_main!(streaming_pattern_benchmarks);
