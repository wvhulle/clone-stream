mod util;
use clone_stream::ForkStream;
use core::time::Duration;
use futures::{StreamExt, join};
use log::{debug, info, warn};
use std::sync::Arc;
use tokio::{spawn, sync::Barrier, time::sleep};
use util::log;

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

    // Clone 1 is fast - gets items immediately
    let clone_0_task = spawn(async move {
        barrier.wait().await;
        sleep(spacing.mul_f32(0.2)).await;
        debug!("Clone 0 starting");
        let first = clone_0.next().await.unwrap();
        debug!("Clone 0 got first item: {first}");

        let next = clone_0.next().await.unwrap();
        debug!("Clone 0 got next item: {next}");
        // assert_eq!(next - first, 1, "Clone 0      should get consecutive items");
        (first, next)
    });

    // Clone 2 is slow - blocks after first item
    let clone_1_task = spawn(async move {
        barrier1.wait().await;
        sleep(spacing.mul_f32(0.5)).await; // Ensure clone 0 starts first
        debug!("Clone 1 starting");
        let first = clone_1.next().await.unwrap();
        debug!("Clone 1 got first item: {first}, now blocking...");

        // Block for longer than the stream interval to miss at least one item
        sleep(spacing.mul_f32(1.0)).await;
        debug!("Clone 1 woke up, polling next item...");

        let next = clone_1.next().await.unwrap();
        debug!("Clone 1 got next item: {next}");

        // With limited queue capacity, slow clone should miss items
        let difference = next - first;
        debug!("Clone 1 difference: {difference}");
        // assert!(difference > 1, "Clone 1 should have missed at least one message (got difference {})", difference);
        (first, next)
    });

    let (good_result, bad_result) = join!(clone_0_task, clone_1_task);

    let (good_first, good_next) = good_result.expect("clone_0 panicked");
    let (bad_first, bad_next) = bad_result.expect("clone_1 panicked");

    debug!("Results - Clone 0: {good_first} → {good_next}, Clone 1: {bad_first} → {bad_next}");

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
    log();
    let spacing = Duration::from_millis(100);

    let interval = tokio::time::interval(spacing);
    let stream = tokio_stream::wrappers::IntervalStream::new(interval)
        .enumerate()
        .map(|(i, _)| i);

    let mut clone_0 = stream.fork_with_limits(1, 3);

    let mut clone_1 = clone_0.clone();

    let mut clone_2 = clone_0.clone();

    let barrier = Arc::new(Barrier::new(3));
    let barrier1 = barrier.clone();
    let barrier2 = barrier.clone();
    // Clone 0 is fast - gets items immediately
    let clone_0_task = spawn(async move {
        barrier.wait().await;
        // sleep(spacing.mul_f32(0.2)).await;
        info!("Clone 0 starting");
        let first = clone_0.next().await.unwrap();
        info!("Clone 0 got first item: {first}");

        let next = clone_0.next().await.unwrap();
        info!("Clone 0 got next item: {next}");
        (first, next)
    });

    // Clone 1
    let clone_1_task = spawn(async move {
        barrier1.wait().await;
        // sleep(spacing.mul_f32(0.1)).await; // Ensure clone 0 starts first
        info!("Clone 1 starting");
        let first = clone_1.next().await.unwrap();
        info!("Clone 1 got first item: {first}, now blocking...");

        let next = clone_1.next().await.unwrap();
        info!("Clone 1 got next item: {next}");
        (first, next)
    });

    // Clone 2 is slow - blocks after first item
    let clone_2_task = spawn(async move {
        barrier2.wait().await;
        // sleep(spacing.mul_f32(0.1)).await;

        info!("Clone 2 starting");
        let first = clone_2.next().await.unwrap();
        info!("Clone 2 got first item: {first}, now blocking...");

        // Block for longer than the stream interval to miss at least one item
        sleep(spacing.mul_f32(2.0)).await;
        debug!("Clone 2 woke up, polling next item...");

        let next = clone_2.next().await.unwrap();
        info!("Clone 2 got next item: {next}");

        // With limited queue capacity, slow clone should miss items
        (first, next)
    });

    debug!("All tasks spawned, waiting for results...");
    let (result_0, result_1, result_2) = join!(clone_0_task, clone_1_task, clone_2_task);

    info!("All tasks completed, processing results...");
    let (first_0, second_0) = result_0.expect("clone_0 panicked");
    let (first_1, second_1) = result_1.expect("clone_1 panicked");
    let (first_2, second_2) = result_2.expect("clone_2 panicked");

    warn!(
        "Results - Clone 0: {first_0} → {second_0}, Clone 1: {first_1} → {second_1}, Clone 2: {first_2} → {second_2}"
    );
    let elements_missed_by_0 = (second_0 - first_0) - 1;
    assert_eq!(
        elements_missed_by_0, 0,
        "clone_0 should get consecutive items since it does not have a blocking call in between."
    );
    let elements_missed_by_1 = (second_1 - first_1) - 1;
    assert_eq!(
        elements_missed_by_1, 0,
        "clone_1 should not have missed the second element since it should have been cached in the queue."
    );
    let elements_missed_by_2 = (second_2 - first_2) - 1;
    assert!(
        elements_missed_by_2 >= 1,
        "clone_2 should have missed at least one element since it was blocked for a long time."
    );
}
