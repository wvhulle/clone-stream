use core::time::Duration;

use clone_stream::ForkStream;
use futures::{StreamExt, future::join_all};
use tokio::{select, time::Instant};
use util::until;
mod util;

#[tokio::test]
async fn cancelled_next_queue_empty() {
    util::log();
    let (sender, rx) = tokio::sync::mpsc::unbounded_channel::<usize>();

    let input_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

    let mut clone_stream = input_stream.fork();
    let start = Instant::now() + Duration::from_millis(10);

    join_all([
        tokio::spawn(async move {
            until(start, 5).await;
            sender.send(1).unwrap();
        }),
        tokio::spawn(async move {
            until(start, 2).await;
            select! {
                _ = clone_stream.next() => {
                    panic!("Stream clone should have received 1");
                }
                () = until(start, 4) => {
                }
            };

            until(start, 6).await;

            assert_eq!(
                clone_stream.n_queued_items(),
                0,
                "Stream clone should have 0 queued item"
            );

            drop(clone_stream);
        }),
    ])
    .await
    .iter()
    .for_each(|result| {
        result.as_ref().unwrap();
    });
}
