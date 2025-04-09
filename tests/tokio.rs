use std::task::Poll;

pub use forked_stream::TimeRange;
use forked_stream::{
    TOKIO_TASK_STARTUP, TestSetup, space_required_consecutive_fork_polls,
    time_fork_needs_to_wake_and_receive, time_start_tokio_task,
    time_to_receive_on_fork_after_input,
};
use futures::SinkExt;
use tokio::time::sleep_until;

const LARGE_N_FORKS: usize = 100;

#[tokio::test]
async fn none_sent() {
    let simple_test_time_range = TimeRange::from(TOKIO_TASK_STARTUP.mul_f32(2.0));
    let task_time_range = simple_test_time_range.inner(0.1);

    let mut setup: TestSetup = TestSetup::new(1);

    let task = setup.forks[0]
        .take()
        .unwrap()
        .assert_background(Poll::Pending, task_time_range);

    sleep_until(simple_test_time_range.middle()).await;

    let _ = task.await.expect("Background task panicked.");
}

#[tokio::test]
async fn one_sent_received() {
    let simple_test_time_range =
        TimeRange::after_for(time_start_tokio_task(1), time_start_tokio_task(1));

    let mut setup: TestSetup = TestSetup::new(1);

    let task = setup.forks[0]
        .take()
        .unwrap()
        .assert_background(Poll::Ready(Some(0)), simple_test_time_range);

    sleep_until(simple_test_time_range.middle()).await;

    setup.sender.send(0).await.unwrap();

    let _ = task.await.expect("Background task panicked.");
}

#[tokio::test]
async fn start_poll_sequentially() {
    let sub_poll_time_ranges = TimeRange::consecutive(
        time_fork_needs_to_wake_and_receive(LARGE_N_FORKS),
        LARGE_N_FORKS,
        space_required_consecutive_fork_polls(LARGE_N_FORKS),
    );

    let last = sub_poll_time_ranges.last().unwrap();

    let mut setup: TestSetup = TestSetup::new(LARGE_N_FORKS);

    let mut sender = setup.sender.clone();

    let metrics = setup
        .launch(|i| sub_poll_time_ranges[i], async move {
            sleep_until(last.middle()).await;

            sender.send(0).await.unwrap();
        })
        .await;

    assert!(metrics.success());
}

#[tokio::test]
async fn start_poll_abort_simultaneously() {
    let test_time_range = TimeRange::after_for(
        time_fork_needs_to_wake_and_receive(LARGE_N_FORKS),
        time_to_receive_on_fork_after_input(LARGE_N_FORKS),
    );

    let mut setup: TestSetup = TestSetup::new(LARGE_N_FORKS);

    let mut sender = setup.sender.clone();

    let metrics = setup
        .launch(|_| test_time_range, async move {
            sleep_until(test_time_range.middle()).await;

            sender.send(0).await.unwrap();
        })
        .await;

    assert!(metrics.success());
}
