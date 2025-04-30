use core::time::Duration;

use clone_stream::ForkStream;
use futures::future::{join_all, try_join_all};
use log::{info, trace};
use tokio::{select, time::Instant};
use util::until;

mod util;

#[tokio::test]

async fn basic_clone() {
    util::log();
    info!("Starting test");
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<usize>();

    info!("Creating stream");
    let rx = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

    let mut first_clone = rx.fork();

    info!("Creating clone");
    let mut second_clone = first_clone.clone();

    let start = Instant::now() + Duration::from_millis(10);

    info!("Starting send task");
    let send = tokio::spawn(async move {
        info!("Waiting a bit to send");
        until(start, 3).await;

        trace!("Sending 1");
        tx.send(1).unwrap();
        trace!("Sent 1");
    });

    let first_clone_receive = tokio::spawn(async move {
        info!("A few milliseconds before listening for the first item on clone 0.");
        until(start, 2).await;

        trace!("Fork stream should receive 1");
        assert_eq!(
            first_clone.next().await,
            Some(1),
            "Fork stream should have received 1"
        );
    });

    let clone_receive = tokio::spawn(async move {
        until(start, 2).await;

        trace!("Clone stream should receive 1");
        assert_eq!(
            second_clone.next().await,
            Some(1),
            "Clone stream should have received 1"
        );
    });

    try_join_all([send, first_clone_receive, clone_receive])
        .await
        .unwrap();
}

#[tokio::test]

async fn clone_late() {
    util::log();
    info!("Starting test");
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<usize>();

    info!("Creating stream");
    let rx = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

    let mut first_clone = rx.fork();

    info!("Creating clone");
    let mut second_clone = first_clone.clone();

    let start = Instant::now() + Duration::from_millis(10);

    info!("Starting send task");
    let send = tokio::spawn(async move {
        info!("Waiting a bit to send");
        until(start, 3).await;

        trace!("Sending 1");
        tx.send(1).unwrap();
        trace!("Sent 1");

        until(start, 5).await;

        trace!("Sending 2");
        tx.send(2).unwrap();
        trace!("Sent 2");
    });

    let first_clone_receive = tokio::spawn(async move {
        info!("A few milliseconds before listening for the first item on clone 0.");
        until(start, 2).await;

        trace!("Fork stream should receive 1");
        assert_eq!(
            first_clone.next().await,
            Some(1),
            "Fork stream should have received 1"
        );
    });

    let clone_receive = tokio::spawn(async move {
        until(start, 2).await;

        trace!("Clone stream should receive 1");
        assert_eq!(
            second_clone.next().await,
            Some(1),
            "Clone stream should have received 1"
        );

        let mut third_clone = second_clone.clone();
        until(start, 4).await;

        trace!("Third clone stream should receive 2");
        assert_eq!(
            third_clone.next().await,
            Some(2),
            "Third clone stream should have received 2"
        );
    });

    try_join_all([send, first_clone_receive, clone_receive])
        .await
        .unwrap();
}

#[tokio::test]
async fn queue_length() {
    util::log();
    info!("Starting test");
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<usize>();

    info!("Creating stream");
    let rx = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

    let mut first_clone = rx.fork();
    let start = Instant::now() + Duration::from_millis(10);

    join_all([
        tokio::spawn(async move {
            until(start, 5).await;
            tx.send(1).unwrap();
        }),
        tokio::spawn(async move {
            until(start, 2).await;
            select! {
                _ = first_clone.next() => {
                    panic!("Fork stream should have received 1");
                }
                () = until(start, 4) => {
                    trace!("Canceling await of first clone.");
                }
            };

            until(start, 6).await;

            assert_eq!(
                first_clone.n_queued_items(),
                0,
                "Fork stream should have 0 queued item"
            );

            drop(first_clone);
        }),
    ])
    .await
    .iter()
    .for_each(|result| {
        result.as_ref().unwrap();
    });
}

#[tokio::test]
async fn cancel() {
    util::log();
    info!("Starting test");
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<usize>();

    info!("Creating stream");
    let rx = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

    let mut first_clone = rx.fork();
    let mut second_clone = first_clone.clone();
    let start = Instant::now() + Duration::from_millis(10);

    join_all([
        tokio::spawn(async move {
            until(start, 10).await;
            trace!("Sending 1");
            tx.send(1).unwrap();
            trace!("Sent 1");
        }),
        tokio::spawn(async move {
            until(start, 2).await;

            trace!(
                "Clone {} should stop too early to receive 1.",
                first_clone.id
            );
            select! {
                _ = first_clone.next() => {
                    panic!("Fork stream should have received 1");
                }
                () = until(start, 4) => {
                    trace!("Cancelled next() await of clone {}.", first_clone.id);
                }
            };
        }),
        tokio::spawn(async move {
            trace!("Waiting a bit before receiving from the second clone");

            until(start, 4).await;

            trace!("Clone {} starts to wait to receive 1.", second_clone.id);
            select! {
                next = second_clone.next() => {
                    trace!("Clone {} received item: {next:?}", second_clone.id);
                    assert_eq!(next, Some(1), "Clone stream should have received 1");
                }
                () = until(start, 15) => {
                    panic!("Did not receive value in time.");
                }
            }
        }),
    ])
    .await
    .iter()
    .for_each(|result| {
        result.as_ref().unwrap();
    });
}
