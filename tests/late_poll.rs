use core::time::Duration;

use clone_stream::ForkStream;
use futures::{
    StreamExt,
    future::{join_all, try_join_all},
};
use log::{info, trace};
use tokio::{select, time::Instant};
mod util;

use util::until;

#[tokio::test]
async fn poll_before_send() {
    util::log();
    let (sender, rx) = tokio::sync::mpsc::unbounded_channel::<char>();

    let rx = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

    let mut adam = rx.fork();
    let mut bob = adam.clone();
    let start = Instant::now() + Duration::from_millis(10);

    join_all([
        tokio::spawn(async move {
            until(start, 10).await;
            sender.send('a').unwrap();
        }),
        tokio::spawn(async move {
            until(start, 2).await;

            select! {
                _ = adam.next() => {
                    panic!("Adam unexpectedly received 'a'.");
                }
                () = until(start, 4) => {
                    trace!("Cancelled next() await of adam.");
                }
            };
        }),
        tokio::spawn(async move {
            until(start, 4).await;

            select! {
                next = bob.next() => {
                    assert_eq!(next, Some('a'), "Bob should have received 'a'.");
                }
                () = until(start, 15) => {
                    panic!("Bob did not receive 'a' in time.");
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

#[tokio::test]
async fn poll_after_send() {
    let (sender, rx) = tokio::sync::mpsc::unbounded_channel::<char>();

    let input_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

    let mut adam = input_stream.fork();
    let mut bob = adam.clone();

    let start = Instant::now() + Duration::from_millis(10);

    let send = tokio::spawn(async move {
        until(start, 3).await;

        sender.send('a').unwrap();

        until(start, 5).await;

        sender.send('b').unwrap();
    });

    let adam_receives = tokio::spawn(async move {
        until(start, 2).await;

        assert_eq!(
            adam.next().await,
            Some('a'),
            "Adam should have received 'a'."
        );

        until(start, 6).await;

        select! {
            next = adam.next() => {
                assert_eq!(next, None, "Adam should receive None");
            }
            () = until(start, 7) => {

            }
        }
    });

    let bob_receives = tokio::spawn(async move {
        until(start, 2).await;

        assert_eq!(
            bob.next().await,
            Some('a'),
            "Clone stream should have received 1"
        );

        until(start, 4).await;

        select! {
            next = bob.next() => {
                assert_eq!(next, Some('b'), "Bob should have received 'b'.");
            },
            () = until(start, 7) => {
                panic!("Bob timed out");
            }
        }
    });

    try_join_all([send, adam_receives, bob_receives])
        .await
        .unwrap();
}
