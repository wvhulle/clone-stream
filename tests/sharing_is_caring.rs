mod mock;

use std::task::Poll;

use futures::{SinkExt, future::try_join_all};
use log::info;
use mock::{ConcurrentSetup, TestableStream, instants};
use tokio::time::sleep_until;

#[tokio::test]
async fn next_becoms_pending_when_it_happens_before_send() {
    let setup = ConcurrentSetup::<()>::new();

    let test_moments = instants(10);

    let primary_task = setup
        .forked_stream
        .assert_background(Poll::Pending, test_moments[1]);

    info!("Waiting a bit so that the background task can finish... ");

    sleep_until(test_moments[2]).await;

    info!("Waiting until background task is ready.");
    primary_task.await.expect("Background task panicked.");
}

#[tokio::test]
async fn next_becomes_ready_when_killed_after_send() {
    let mut setup = ConcurrentSetup::new();

    let test_moments = instants(10);

    // First background task is started
    let primary_task = setup
        .forked_stream
        .assert_background(Poll::Ready(Some(0)), test_moments[1]);

    sleep_until(test_moments[0]).await;

    info!("Sending an item to the input stream.");
    setup.input_sink.send(0).await.unwrap();

    primary_task.await.expect("Background task panicked.");
}

#[tokio::test]
async fn hundreds_next_polls_at_the_same_time() {
    let mut setup = ConcurrentSetup::new();

    let test_moments = instants(10);

    let wait_for_all = try_join_all((1..100).map(|_| {
        setup
            .forked_stream
            .assert_background(Poll::Ready(Some(0)), test_moments[1])
    }));
    sleep_until(test_moments[0]).await;

    info!("Sending an item to the input stream.");
    setup.input_sink.send(0).await.unwrap();

    wait_for_all.await.expect("Background task panicked.");
}

#[tokio::test]
async fn poll_three_forks_consecutively() {
    let mut setup = ConcurrentSetup::new();

    let test_moments = instants(10);

    // First background task is started
    let primary_task = setup
        .forked_stream
        .assert_background(Poll::Ready(Some(0)), test_moments[1]);

    // First background task is started
    let secundary_task = setup
        .forked_stream
        .assert_background(Poll::Ready(Some(0)), test_moments[2]);

    let tertiary_task = setup
        .forked_stream
        .assert_background(Poll::Ready(Some(0)), test_moments[3]);

    sleep_until(test_moments[0]).await;

    info!("Sending an item to the input stream.");
    setup.input_sink.send(0).await.unwrap();

    primary_task
        .await
        .expect("Primary background task panicked.");
    secundary_task
        .await
        .expect("Secundary background task panicked.");

    tertiary_task
        .await
        .expect("Tertiary background task panicked.");
}
