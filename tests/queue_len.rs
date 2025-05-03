use core::time::Duration;

use clone_stream::ForkStream;
use futures::{StreamExt, future::join_all};
use log::{info, trace};
use tokio::{select, time::Instant};
use util::until;
mod util;

#[tokio::test]
async fn queue_length() {
    util::log();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<usize>();

    let input_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

    let mut stream_clone = input_stream.fork();
    let start = Instant::now() + Duration::from_millis(10);

    join_all([
        tokio::spawn(async move {
            until(start, 5).await;
            tx.send(1).unwrap();
        }),
        tokio::spawn(async move {
            until(start, 2).await;
            select! {
                _ = stream_clone.next() => {
                    panic!("Stream clone should have received 1");
                }
                () = until(start, 4) => {
                    trace!("Canceling await of first clone.");
                }
            };

            until(start, 6).await;

            assert_eq!(
                stream_clone.n_queued_items(),
                0,
                "Stream clone should have 0 queued item"
            );

            drop(stream_clone);
        }),
    ])
    .await
    .iter()
    .for_each(|result| {
        result.as_ref().unwrap();
    });
}
