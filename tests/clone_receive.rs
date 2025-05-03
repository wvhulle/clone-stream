use core::time::Duration;

use clone_stream::ForkStream;
use futures::{StreamExt, future::try_join_all};
use tokio::time::Instant;
use util::until;
mod util;

#[tokio::test]

async fn clone_pair_receives() {
    util::log();
    let (sender, rx) = tokio::sync::mpsc::unbounded_channel::<char>();

    let input_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

    let mut adam = input_stream.fork();

    let mut bob = adam.clone();

    let start = Instant::now() + Duration::from_millis(10);

    let send = tokio::spawn(async move {
        until(start, 3).await;

        sender.send('a').unwrap();
    });

    let adam_receives = tokio::spawn(async move {
        until(start, 2).await;

        assert_eq!(
            adam.next().await,
            Some('a'),
            "Stream clone should have received 'a'."
        );
    });

    let bob_receives = tokio::spawn(async move {
        until(start, 2).await;

        assert_eq!(bob.next().await, Some('a'), "Bob should have received 'a'.");
    });

    try_join_all([send, adam_receives, bob_receives])
        .await
        .unwrap();
}
