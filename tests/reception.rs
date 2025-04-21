use core::time::Duration;

use clone_stream::ForkStream;
use futures::{
    FutureExt, StreamExt,
    future::{select_all, try_join_all},
};
use log::{LevelFilter, info};
use tokio::{
    select,
    time::{Instant, sleep, sleep_until},
};

fn until(start: Instant, n: usize) -> impl Future<Output = ()> {
    sleep_until(start + Duration::from_millis(10) * n as u32)
}

#[tokio::test]
async fn test_sleep() {
    let _ = env_logger::builder()
        .format_file(true)
        .format_line_number(true)
        .format_module_path(false)
        .filter_level(LevelFilter::Trace)
        .try_init();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<usize>();

    let rx = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

    let mut fork = rx.fork();

    let mut clone = fork.clone();

    let start = Instant::now() + Duration::from_millis(10);

    let send = tokio::spawn(async move {
        until(start, 3).await;

        info!("Sending 1");
        tx.send(1).unwrap();
        info!("Sent 1");

        until(start, 5).await;

        info!("Sending 2");
        tx.send(2).unwrap();
        info!("Sent 2");
    });

    let fork_receive = tokio::spawn(async move {
        until(start, 2).await;

        info!("Fork stream should receive 1");
        assert_eq!(
            fork.next().await,
            Some(1),
            "Fork stream should have received 1"
        );

        until(start, 6).await;

        info!("Fork stream should now time out.");
        select! {
            next = fork.next() => {
                assert_eq!(next, None, "Fork stream should receive None");
            }
            () = until(start, 7) => {

            }
        }
    });

    let clone_receive = tokio::spawn(async move {
        until(start, 2).await;

        info!("Clone stream should receive 1");
        assert_eq!(
            clone.next().await,
            Some(1),
            "Clone stream should have received 1"
        );
        info!("Clone stream received 1");

        until(start, 4).await;

        info!("Clone stream should receive 2");
        select! {
            next = clone.next() => {
                assert_eq!(next, Some(2), "Clone stream should have received 2");
            },
            () = until(start, 7) => {
                panic!("Clone stream  timed out");
            }
        }
    });

    try_join_all([send, fork_receive, clone_receive])
        .await
        .unwrap();
}
