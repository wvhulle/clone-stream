use core::time::Duration;

use clone_stream::ForkStream;
use futures::{StreamExt, future::try_join_all};
use log::{info, trace};
use tokio::{
    select,
    time::{Instant, sleep_until},
};

fn until(start: Instant, n: usize) -> impl Future<Output = ()> {
    sleep_until(start + Duration::from_millis(10) * n as u32)
}

#[tokio::test]

async fn skip() {
    let _ = env_logger::builder()
        .format_file(true)
        .format_line_number(true)
        .filter_level(log::LevelFilter::Trace)
        .format_module_path(false)
        .try_init();
    info!("Starting test");
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<usize>();

    info!("Creating stream");
    let rx = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);

    let mut fork = rx.fork();

    info!("Creating clone");
    let mut clone = fork.clone();

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

    let fork_receive = tokio::spawn(async move {
        info!("A few milliseconds before listening for the first item on clone 0.");
        until(start, 2).await;

        trace!("Fork stream should receive 1");
        assert_eq!(
            fork.next().await,
            Some(1),
            "Fork stream should have received 1"
        );

        until(start, 6).await;

        trace!("Fork stream should now time out.");
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

        trace!("Clone stream should receive 1");
        assert_eq!(
            clone.next().await,
            Some(1),
            "Clone stream should have received 1"
        );
        trace!("Clone stream received 1");

        until(start, 4).await;

        trace!("Clone stream should receive 2");
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
