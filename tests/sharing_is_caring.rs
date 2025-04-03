mod mock;

use std::{task::Poll, time::Duration};

use futures::{SinkExt, future::try_join_all};
use mock::{ConcurrentSetup, TestableStream, TimeRange};
use tokio::time::sleep_until;

const N_FORKS: u32 = 100;

#[tokio::test]
async fn next_becoms_pending_when_it_happens_before_send() {
    let setup = ConcurrentSetup::<()>::new();

    let primary_task = setup
        .forked_stream
        .assert_background(Poll::Pending, setup.time_range);

    sleep_until(setup.time_range.middle()).await;

    primary_task.await.expect("Background task panicked.");
}

#[tokio::test]
async fn start_end_same_time() {
    let mut setup = ConcurrentSetup::new();

    let wait_for_all = try_join_all((0..N_FORKS).map(|_| {
        setup
            .forked_stream
            .assert_background(Poll::Ready(Some(0)), setup.time_range)
    }));
    sleep_until(setup.time_range.middle()).await;

    setup.input_sink.send(0).await.unwrap();

    wait_for_all.await.expect("Background task panicked.");
}

#[tokio::test]
async fn start_end_different_times() {
    let mut setup = ConcurrentSetup::new();

    let phases = setup
        .time_range
        .split_into_consecutive_range_with_gaps(3, Duration::from_millis(1));

    let start_up_phase = phases[0];
    let send_phase = phases[1];
    let end_phase = phases[2];

    let start_moments = start_up_phase.split_into_consecutive_instants(N_FORKS);
    let end_moments = end_phase.split_into_consecutive_instants(N_FORKS);

    let wait_for_all = try_join_all((0..N_FORKS - 1).map(|i| {
        setup.forked_stream.assert_background(
            Poll::Ready(Some(0)),
            TimeRange {
                start: start_moments[i as usize],
                end: end_moments[i as usize],
            },
        )
    }));
    sleep_until(send_phase.middle()).await;

    setup.input_sink.send(0).await.unwrap();

    wait_for_all.await.expect("Background task panicked.");
}
