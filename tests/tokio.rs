use std::task::Poll;

pub use forked_stream::TimeRange;
use forked_stream::{
    ForkAsyncMockSetup, TOKIO_TASK_STARTUP, fork_warmup, resume_forks, spacing_required, warmup,
};
use futures::SinkExt;
use log::info;
use tokio::time::sleep_until;

const LARGE_N_FORKS: usize = 100;

#[tokio::test]
async fn none_sent() {
    let simple_test_time_range = TimeRange::from(TOKIO_TASK_STARTUP.mul_f32(2.0));
    let task_time_range = simple_test_time_range.inner(0.1);

    let mut setup = ForkAsyncMockSetup::<1>::new(1);

    let task = setup.forks[0]
        .take()
        .unwrap()
        .assert_background(Poll::Pending, task_time_range);

    sleep_until(simple_test_time_range.middle()).await;

    let _ = task.await.expect("Background task panicked.");
}

#[tokio::test]
async fn one_sent_received() {
    let simple_test_time_range = TimeRange::after_for(warmup(1), warmup(1));

    let mut setup = ForkAsyncMockSetup::<1>::new(1);

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
    println!("Starting test");
    let sub_poll_time_ranges = TimeRange::sequential_overlapping_sub_ranges_from(
        fork_warmup(LARGE_N_FORKS),
        LARGE_N_FORKS,
        spacing_required(LARGE_N_FORKS),
    );

    println!("Sub poll time ranges: {}", sub_poll_time_ranges.len());

    let last = sub_poll_time_ranges.last().unwrap();

    let mut setup = ForkAsyncMockSetup::<1>::new(LARGE_N_FORKS);

    let mut sender = setup.sender.clone();

    let metrics = setup
        .launch(|i| sub_poll_time_ranges[i], async move {
            info!("Waiting to send until the middle of the send phase");
            sleep_until(last.middle()).await;

            info!("Sending item");
            sender.send(0).await.unwrap();
        })
        .await;

    assert!(metrics.is_successful());
}

#[tokio::test]
async fn start_poll_abort_simultaneously() {
    let test_time_range =
        TimeRange::after_for(fork_warmup(LARGE_N_FORKS), resume_forks(LARGE_N_FORKS));

    let mut setup = ForkAsyncMockSetup::<1>::new(LARGE_N_FORKS);

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
