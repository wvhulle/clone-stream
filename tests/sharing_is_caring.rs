mod mock;

use std::{task::Poll, time::Duration};

use futures::{SinkExt, future::try_join_all};
use log::info;
use mock::{ConcurrentSetup, TestableStream, instants_between};
use tokio::time::{Instant, sleep_until};

const N_FORKS: u32 = 100;

#[tokio::test]
async fn next_becoms_pending_when_it_happens_before_send() {
    let setup = ConcurrentSetup::<()>::new();

    let primary_task = setup.forked_stream.assert_background(
        Poll::Pending,
        Instant::now(),
        Instant::now() + Duration::from_millis(10),
    );

    sleep_until(Instant::now() + Duration::from_millis(5)).await;

    primary_task.await.expect("Background task panicked.");
}

#[tokio::test]
async fn hundreds_next_polls_at_the_same_time() {
    let mut setup = ConcurrentSetup::new();

    let start_test = Instant::now();
    let end_test = start_test + Duration::from_millis(100);

    let wait_for_all = try_join_all((0..N_FORKS).map(|_| {
        setup
            .forked_stream
            .assert_background(Poll::Ready(Some(0)), start_test, end_test)
    }));
    sleep_until(start_test + Duration::from_millis(50)).await;

    info!("Sending an item to the input stream.");
    setup.input_sink.send(0).await.unwrap();

    wait_for_all.await.expect("Background task panicked.");
}

#[tokio::test]
async fn hundreds_next_polls_randomly() {
    let mut setup = ConcurrentSetup::new();

    let start_test = Instant::now();
    let start_send = start_test + Duration::from_millis(10);
    let end_send = start_send + Duration::from_millis(10);
    let end_test = end_send + Duration::from_millis(100);

    let start_moments = instants_between(start_test, start_send, N_FORKS);
    let end_moments = instants_between(end_send, end_test, N_FORKS);

    let wait_for_all = try_join_all((0..N_FORKS - 1).map(|i| {
        setup.forked_stream.assert_background(
            Poll::Ready(Some(0)),
            start_moments[i as usize],
            end_moments[(i + 1) as usize],
        )
    }));
    sleep_until(start_send).await;

    setup.input_sink.send(0).await.unwrap();

    wait_for_all.await.expect("Background task panicked.");
}
