mod util;
use core::time::Duration;
use std::sync::Arc;

use clone_stream::ForkStream;
use futures::{StreamExt, join};
use tokio::{spawn, sync::Barrier, time::sleep};

/// Test that with limited queue capacity, slow clones miss messages
#[tokio::test]
async fn slow_clone_not_miss_cache() {
    let spacing = Duration::from_millis(100);

    let interval = tokio::time::interval(spacing);
    let stream = tokio_stream::wrappers::IntervalStream::new(interval)
        .enumerate()
        .map(|(i, _)| i);

    let mut clone_0 = stream.fork_with_limits(1, 2);

    let mut clone_1 = clone_0.clone();

    let barrier = Arc::new(Barrier::new(2));
    let barrier1 = barrier.clone();

    let clone_0_task = spawn(async move {
        barrier.wait().await;
        sleep(spacing.mul_f32(0.2)).await;
        let first = clone_0.next().await.unwrap();
        let next = clone_0.next().await.unwrap();
        (first, next)
    });

    let clone_1_task = spawn(async move {
        barrier1.wait().await;
        sleep(spacing.mul_f32(0.5)).await; // Ensure clone 0 starts first
        let first = clone_1.next().await.unwrap();

        // Block for longer than the stream interval
        sleep(spacing.mul_f32(1.0)).await;

        let next = clone_1.next().await.unwrap();
        (first, next)
    });

    let (good_result, bad_result) = join!(clone_0_task, clone_1_task);

    let (good_first, good_next) = good_result.expect("clone_0 panicked");
    let (bad_first, bad_next) = bad_result.expect("clone_1 panicked");


    assert!(
        good_next - good_first == 1,
        "clone_0 should get consecutive items since it does not have a blocking call in between \
         (got {}).",
        good_next - good_first
    );
    assert!(
        bad_next - bad_first == 1,
        "clone_1 should have not have missed the second element since it should have been cached \
         in the queue, instead it missed {} elements.",
        bad_next - bad_first
    );
}

/// Test that bounded queues cause item drops under contention
#[tokio::test]
async fn bounded_queue_causes_drops() {
    const NUM_SAMPLES: usize = 3;
    let mut total_misses = Vec::new();
    let mut drop_count = 0;

    for _sample in 0..NUM_SAMPLES {
        let spacing = Duration::from_millis(1);
        let misses = test_queue_scenario(spacing, 1, 3).await;
        total_misses.push(misses);

        if misses > 0 {
            drop_count += 1;
        }
    }

    #[allow(clippy::cast_precision_loss)]
    let avg_misses = total_misses.iter().sum::<usize>() as f64 / NUM_SAMPLES as f64;


    // The key test: bounded queues should cause some drops under contention
    assert!(
        avg_misses > 0.0,
        "Bounded queues should cause item drops under contention. Got: {avg_misses:.2}"
    );

    // Most samples should experience drops
    assert!(
        drop_count >= (NUM_SAMPLES / 2),
        "Most samples should experience drops with bounded queues. {drop_count}/{NUM_SAMPLES} experienced drops"
    );

}

async fn test_queue_scenario(
    spacing: Duration,
    queue_capacity: usize,
    clone_count: usize,
) -> usize {
    let interval = tokio::time::interval(spacing);
    let stream = tokio_stream::wrappers::IntervalStream::new(interval)
        .enumerate()
        .map(|(i, _)| i);

    let mut clone_0 = stream.fork_with_limits(queue_capacity, clone_count);
    let mut clone_1 = clone_0.clone();
    let mut clone_2 = clone_0.clone();

    let barrier = Arc::new(Barrier::new(3));
    let barrier1 = barrier.clone();
    let barrier2 = barrier.clone();

    let clone_0_task = spawn(async move {
        barrier.wait().await;
        let first = clone_0.next().await.unwrap();
        // Short delay
        sleep(spacing.mul_f32(50.0)).await;
        let second = clone_0.next().await.unwrap();
        (first, second)
    });

    let clone_1_task = spawn(async move {
        barrier1.wait().await;
        let first = clone_1.next().await.unwrap();
        // Medium delay
        sleep(spacing.mul_f32(100.0)).await;
        let second = clone_1.next().await.unwrap();
        (first, second)
    });

    let clone_2_task = spawn(async move {
        barrier2.wait().await;
        let first = clone_2.next().await.unwrap();
        // Long delay to create queue pressure
        sleep(spacing.mul_f32(200.0)).await;
        let second = clone_2.next().await.unwrap();
        (first, second)
    });

    let (result_0, result_1, result_2) = join!(clone_0_task, clone_1_task, clone_2_task);

    let (first_0, second_0) = result_0.expect("clone_0 panicked");
    let (first_1, second_1) = result_1.expect("clone_1 panicked");
    let (first_2, second_2) = result_2.expect("clone_2 panicked");

    let missed_0 = (second_0 - first_0).saturating_sub(1);
    let missed_1 = (second_1 - first_1).saturating_sub(1);
    let missed_2 = (second_2 - first_2).saturating_sub(1);

    missed_0 + missed_1 + missed_2
}
