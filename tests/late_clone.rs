use core::time::Duration;

use clone_stream::ForkStream;
use futures::{StreamExt, future::try_join_all};
use tokio::time::Instant;
mod util;

use util::until;

#[tokio::test]

async fn late_clone() {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<char>();

    let input_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(receiver);

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
    });

    let bob_receives = tokio::spawn(async move {
        until(start, 2).await;

        assert_eq!(bob.next().await, Some('a'), "Bob should have received 'a'.");

        let mut third_clone = bob.clone();
        until(start, 4).await;

        assert_eq!(
            third_clone.next().await,
            Some('b'),
            "Bob should have received 'b'."
        );
    });

    try_join_all([send, adam_receives, bob_receives])
        .await
        .unwrap();
}
