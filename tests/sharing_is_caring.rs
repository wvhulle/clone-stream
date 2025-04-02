use std::{task::Poll, time::Duration};

use forked_stream::new_concurrent_setup;
use futures::SinkExt;
use log::info;
use tokio::time::sleep;

#[tokio::test]
async fn background_task() {
    let mut setup = new_concurrent_setup();

    let background = setup.expect_background(Some(0), Duration::from_millis(100));

    sleep(Duration::from_millis(10)).await;

    info!("Sending an item to the input stream.");
    setup.input_sink.send(0).await.unwrap();
    info!("Sent an item to the input stream.");

    sleep(Duration::from_millis(10)).await;

    let completed = setup.current_next();
    assert_eq!(completed, Poll::Ready(Some(0)));

    background.await.expect("Background task panicked.");
}
