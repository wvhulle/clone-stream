mod util;
use clone_stream::ForkStream;
use core::time::Duration;
use futures::{StreamExt, join};
use log::{debug, info, warn};
use std::sync::Arc;
use tokio::{spawn, sync::Barrier, time::sleep};

/// Test that with limited queue capacity, slow clones miss messages
#[tokio::test]
async fn slow_clone_not_miss_cache() {
    // log();
    let spacing = Duration::from_millis(100);

    let interval = tokio::time::interval(spacing);
    let stream = tokio_stream::wrappers::IntervalStream::new(interval)
        .enumerate()
        .map(|(i, _)| i);

    let mut clone_0 = stream.fork_with_limits(1, 2);

    let mut clone_1 = clone_0.clone();

    let barrier = Arc::new(Barrier::new(2));
    let barrier1 = barrier.clone();

    // Clone 1 is fast - gets items immediately
    let clone_0_task = spawn(async move {
        barrier.wait().await;
        sleep(spacing.mul_f32(0.2)).await;
        debug!("Clone 0 starting");
        let first = clone_0.next().await.unwrap();
        info!("Clone 0 got first item: {first}");

        let next = clone_0.next().await.unwrap();
        info!("Clone 0 got next item: {next}");
        // assert_eq!(next - first, 1, "Clone 0      should get consecutive items");
        (first, next)
    });

    // Clone 2 is slow - blocks after first item
    let clone_1_task = spawn(async move {
        barrier1.wait().await;
        sleep(spacing.mul_f32(0.5)).await; // Ensure clone 0 starts first
        debug!("Clone 1 starting");
        let first = clone_1.next().await.unwrap();
        info!("Clone 1 got first item: {first}, now blocking...");

        // Block for longer than the stream interval to miss at least one item
        sleep(spacing.mul_f32(1.0)).await;
        info!("Clone 1 woke up, polling next item...");

        let next = clone_1.next().await.unwrap();
        info!("Clone 1 got next item: {next}");

        // With limited queue capacity, slow clone should miss items
        let difference = next - first;
        debug!("Clone 1 difference: {difference}");
        // assert!(difference > 1, "Clone 1 should have missed at least one message (got difference {})", difference);
        (first, next)
    });

    let (good_result, bad_result) = join!(clone_0_task, clone_1_task);

    let (good_first, good_next) = good_result.expect("clone_0 panicked");
    let (bad_first, bad_next) = bad_result.expect("clone_1 panicked");

    info!("Results - Clone 0: {good_first} → {good_next}, Clone 1: {bad_first} → {bad_next}");

    assert!(
        good_next - good_first == 1,
        "clone_0 should get consecutive items since it does not have a blocking call in between (got {}).",
        good_next - good_first
    );
    assert!(
        bad_next - bad_first == 1,
        "clone_1 should have not have missed the second element since it should have been cached in the queue, instead it missed {} elements.",
        bad_next - bad_first
    );
}

/// Test that with limited queue capacity, slow clones miss messages
#[tokio::test]
async fn slow_clone_miss_cache() {
    const NUM_SAMPLES: usize = 10;
    let mut fast_clone_misses = Vec::new();
    let mut slow_clone_misses = Vec::new();

    for sample in 0..NUM_SAMPLES {
        info!("Running sample {}/{NUM_SAMPLES}", sample + 1);

        let spacing = Duration::from_millis(50); // Faster intervals for quicker test
        let interval = tokio::time::interval(spacing);
        let stream = tokio_stream::wrappers::IntervalStream::new(interval)
            .enumerate()
            .map(|(i, _)| i);

        let mut clone_0 = stream.fork_with_limits(1, 3); // Smaller capacity for more contention

        let mut clone_1 = clone_0.clone();
        let mut clone_2 = clone_0.clone();

        let barrier = Arc::new(Barrier::new(3));
        let barrier1 = barrier.clone();
        let barrier2 = barrier.clone();

        // Fast clone 0
        let clone_0_task = spawn(async move {
            barrier.wait().await;
            let first = clone_0.next().await.unwrap();
            let second = clone_0.next().await.unwrap();
            (first, second)
        });

        // Fast clone 1
        let clone_1_task = spawn(async move {
            barrier1.wait().await;
            let first = clone_1.next().await.unwrap();
            let second = clone_1.next().await.unwrap();
            (first, second)
        });

        // Slow clone 2 - blocks for a significant time
        let clone_2_task = spawn(async move {
            barrier2.wait().await;
            sleep(spacing.mul_f32(0.5)).await; // Start slightly later

            let first = clone_2.next().await.unwrap();
            // Block for much longer than the stream interval
            sleep(spacing.mul_f32(6.0)).await; // 300ms delay vs 50ms intervals
            let second = clone_2.next().await.unwrap();

            (first, second)
        });

        let (result_0, result_1, result_2) = join!(clone_0_task, clone_1_task, clone_2_task);

        let (first_0, second_0) = result_0.expect("clone_0 panicked");
        let (first_1, second_1) = result_1.expect("clone_1 panicked");
        let (first_2, second_2) = result_2.expect("clone_2 panicked");

        let missed_0 = (second_0 - first_0) - 1;
        let missed_1 = (second_1 - first_1) - 1;
        let missed_2 = (second_2 - first_2) - 1;

        // Track the best performing fast clone for this sample
        let fast_clone_missed = std::cmp::min(missed_0, missed_1);
        fast_clone_misses.push(fast_clone_missed);
        slow_clone_misses.push(missed_2);

        debug!(
            "Sample {sample}: Fast clone missed {fast_clone_missed}, Slow clone missed {missed_2}"
        );
    }

    // Calculate averages
    let avg_fast_misses = fast_clone_misses.iter().sum::<usize>() as f64 / NUM_SAMPLES as f64;
    let avg_slow_misses = slow_clone_misses.iter().sum::<usize>() as f64 / NUM_SAMPLES as f64;

    warn!(
        "Average misses over {NUM_SAMPLES} samples: Fast clones: {avg_fast_misses:.2}, Slow clone: {avg_slow_misses:.2}"
    );

    // Statistical assertions: slow clones should miss more items on average
    assert!(
        avg_slow_misses > avg_fast_misses,
        "Slow clones should miss more items on average than fast clones. Fast: {avg_fast_misses:.2}, Slow: {avg_slow_misses:.2}"
    );

    // The difference should be statistically significant (at least 0.5 items on average)
    assert!(
        avg_slow_misses - avg_fast_misses >= 0.5,
        "Slow clones should miss significantly more items than fast clones. Difference: {:.2}",
        avg_slow_misses - avg_fast_misses
    );

    info!(
        "✓ Statistical test passed: Slow clones miss {:.1}x more items than fast clones",
        avg_slow_misses / avg_fast_misses.max(0.1)
    );
}
