mod mock;

use core::task;
use std::{task::Poll, time::Duration};

use futures::{SinkExt, future::try_join_all};
use log::{info, trace};
use mock::ForkAsyncMockSetup;
pub use mock::{StreamWithWakers, TimeRange, log_init};
use tokio::time::sleep_until;

const LARGE_N_FORKS: usize = 200;
const TOKIO_TASK_STARTUP: Duration = Duration::from_micros(1000);

fn complete_spawns(n: usize) -> Duration {
    let n = f32::from(u16::try_from(n).unwrap());
    TOKIO_TASK_STARTUP.mul_f32(n * 0.5 / n.sqrt())
}

fn complete_wake_ups(n: usize) -> Duration {
    let n = f32::from(u16::try_from(n).unwrap());
    TOKIO_TASK_STARTUP.mul_f32(n * 0.23)
}

fn spacing_necessary(n: usize) -> Duration {
    let n = f32::from(u16::try_from(n).unwrap());
    TOKIO_TASK_STARTUP.mul_f32(n * 1.0 / n.sqrt())
}

#[tokio::test]
async fn none_sent() {
    let simple_test_time_range = TimeRange::from(TOKIO_TASK_STARTUP.mul_f32(2.0));
    let task_time_range = simple_test_time_range.inner(0.1);

    let mut setup = ForkAsyncMockSetup::<1, 1>::new();

    let task = setup.forks[0]
        .take()
        .unwrap()
        .assert_background(Poll::Pending, task_time_range);

    sleep_until(simple_test_time_range.middle()).await;

    task.await.expect("Background task panicked.");
}

#[tokio::test]
async fn one_sent_received() {
    const N_FORKS: usize = 1;
    let simple_test_time_range =
        TimeRange::from(TOKIO_TASK_STARTUP * 3 * N_FORKS.try_into().unwrap());

    let task_life_time = simple_test_time_range.inner(0.1);

    let mut setup = ForkAsyncMockSetup::<N_FORKS, 1>::new();

    let task = setup.forks[0]
        .take()
        .unwrap()
        .assert_background(Poll::Ready(Some(0)), task_life_time);

    sleep_until(task_life_time.middle()).await;

    setup.sender.send(0).await.unwrap();

    task.await.expect("Background task panicked.");
}

#[tokio::test]
async fn start_poll_sequentially() {
    let sub_poll_time_ranges = TimeRange::sequential_overlapping_sub_ranges_from(
        complete_spawns(LARGE_N_FORKS),
        LARGE_N_FORKS as u32,
        spacing_necessary(LARGE_N_FORKS),
    );

    let last = sub_poll_time_ranges.last().unwrap();

    let mut setup = ForkAsyncMockSetup::<LARGE_N_FORKS, 1>::new();

    let mut sender = setup.sender.clone();

    let metrics = setup
        .launch(|i| sub_poll_time_ranges[i], async move {
            info!("Waiting to send until the middle of the send phase");
            sleep_until(last.middle()).await;

            info!("Sending item");
            sender.send(0).await.unwrap();
        })
        .await;

    println!("Metrics: {metrics:?}");

    assert!(metrics.is_successful());
}

#[tokio::test]
async fn start_poll_abort_simultaneously() {
    let test_time_range = TimeRange::after_for(
        complete_spawns(LARGE_N_FORKS),
        complete_wake_ups(LARGE_N_FORKS),
    );

    let mut setup = ForkAsyncMockSetup::<LARGE_N_FORKS, 1>::new();

    let mut sender = setup.sender.clone();

    let metrics = setup
        .launch(|_| test_time_range, async move {
            info!("Sending item");

            sleep_until(test_time_range.middle()).await;

            sender.send(0).await.unwrap();
        })
        .await;

    println!("Metrics: {metrics:?}");

    assert!(metrics.is_successful());
}
