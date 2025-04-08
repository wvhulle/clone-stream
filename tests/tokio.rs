mod mock;

use std::{task::Poll, time::Duration};

use futures::{SinkExt, future::try_join_all};
use log::{info, trace};
use mock::ForkAsyncMockSetup;
pub use mock::{StreamWithWakers, TimeRange, log_init};
use tokio::time::sleep_until;

const TOKIO_TASK_STARTUP: Duration = Duration::from_micros(1000);

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
    const N_FORKS: usize = 100;
    let sequential_test_time_range =
        TimeRange::from(TOKIO_TASK_STARTUP * 2 * N_FORKS.try_into().unwrap());

    let task_time_range = sequential_test_time_range.inner(0.1);

    let mut setup = ForkAsyncMockSetup::<N_FORKS, 1>::new();

    let sub_poll_time_ranges =
        sequential_test_time_range.sequential_overlapping_sub_ranges(N_FORKS);

    let wait_for_all = try_join_all(setup.forks.iter_mut().enumerate().map(|(i, fork)| {
        let poll_time_range = sub_poll_time_ranges[i];
        fork.take()
            .unwrap()
            .assert_background(Poll::Ready(Some(0)), poll_time_range)
    }));

    info!("Waiting to send until the middle of the send phase");
    sleep_until(task_time_range.middle()).await;

    info!("Sending item");
    setup.sender.send(0).await.unwrap();

    info!("Sent item");
    wait_for_all.await.expect("Background task panicked.");
}

#[tokio::test]
async fn start_poll_abort_simultaneously() {
    const N_FORKS: usize = 100;
    let test_time_range = TimeRange::from(TOKIO_TASK_STARTUP * 2 * N_FORKS.try_into().unwrap());

    let task_time_range = test_time_range.inner(0.1);

    let mut setup = ForkAsyncMockSetup::<N_FORKS, 1>::new();

    info!("Starting test");
    let wait_for_all = try_join_all(setup.forks.iter_mut().map(|fork| {
        fork.take()
            .unwrap()
            .assert_background(Poll::Ready(Some(0)), task_time_range)
    }));
    info!("Waiting for all tasks to be ready");
    sleep_until(task_time_range.middle()).await;

    info!("Sending item");
    setup.sender.send(0).await.unwrap();

    trace!("Waiting for all tasks to finish");
    wait_for_all.await.expect("Background task panicked.");
}
