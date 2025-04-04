mod mock;

use std::task::Poll;

use futures::{SinkExt, future::try_join_all};
use log::trace;
use mock::{ConcurrentSetup, TestableStream, TimeRange};
use tokio::time::sleep_until;

const N_FORKS: u32 = 100;

#[tokio::test]
async fn undelivered() {
    let setup = ConcurrentSetup::<()>::new(None, 1);

    let primary_task = setup
        .forked_stream
        .assert_background(Poll::Pending, setup.time_range);

    sleep_until(setup.time_range.middle()).await;

    primary_task.await.expect("Background task panicked.");
}

#[tokio::test]
async fn overlapping() {
    let mut setup = ConcurrentSetup::new(None, N_FORKS as usize);

    let phases = setup.time_range.split(3, 0.1);

    let start_up_phase = phases[0];
    let send_phase = phases[1];
    let end_phase = phases[2];

    let start_moments = start_up_phase.moments(N_FORKS);
    let end_moments = end_phase.moments(N_FORKS);

    let wait_for_all = try_join_all((0..N_FORKS).map(|i| {
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

// cargo test --test clone parallel
#[tokio::test]
async fn parallel() {
    let mut setup = ConcurrentSetup::new(None, N_FORKS as usize);
    trace!("Starting test");
    let wait_for_all = try_join_all((0..N_FORKS).map(|_| {
        setup
            .forked_stream
            .assert_background(Poll::Ready(Some(0)), setup.time_range)
    }));
    trace!("Waiting for all tasks to be ready");
    sleep_until(setup.time_range.middle()).await;

    trace!("Sending item");
    setup.input_sink.send(0).await.unwrap();

    trace!("Waiting for all tasks to finish");
    wait_for_all.await.expect("Background task panicked.");
}
