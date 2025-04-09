pub use forked_stream::TimeRange;
use forked_stream::{
    TestSetup, fork_warmup, min_spacing_seq_polled_forks, time_per_fork_to_receive_cold,
};
use futures::SinkExt;
use tokio::time::sleep_until;

const LARGE_N_FORKS: usize = 100;

#[tokio::test]
async fn start_poll_sequentially() {
    let sub_poll_time_ranges = TimeRange::consecutive(
        fork_warmup(LARGE_N_FORKS),
        LARGE_N_FORKS,
        min_spacing_seq_polled_forks(LARGE_N_FORKS),
    );

    let last = sub_poll_time_ranges.last().unwrap();

    let mut setup: TestSetup = TestSetup::new(LARGE_N_FORKS);

    let mut sender = setup.sender.clone();

    let metrics = setup
        .poll_forks_background(|i| sub_poll_time_ranges[i], async move {
            sleep_until(last.middle()).await;

            sender.send(0).await.unwrap();
        })
        .await;

    assert!(metrics.success());
}

#[tokio::test]
async fn start_poll_abort_simultaneously() {
    let test_time_range = TimeRange::after_for(
        fork_warmup(LARGE_N_FORKS),
        time_per_fork_to_receive_cold(LARGE_N_FORKS),
    );

    let mut setup: TestSetup = TestSetup::new(LARGE_N_FORKS);

    let mut sender = setup.sender.clone();

    let metrics = setup
        .poll_forks_background(|_| test_time_range, async move {
            sleep_until(test_time_range.middle()).await;

            sender.send(0).await.unwrap();
        })
        .await;

    assert!(metrics.success());
}
