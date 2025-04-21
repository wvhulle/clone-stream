use core::time::Duration;

use clone_stream::CloneStream;
use futures::{StreamExt, future::join_all, join, stream};
use log::trace;
use tokio::time::{Instant, sleep_until};

const N_STREAM_CLONES: usize = 100;

const N_ITEMS_SENT: usize = 100;

#[test_log::test(tokio::test)]
async fn mass_send() {
    let (sender, receiver) = futures::channel::mpsc::unbounded::<usize>();

    let template_clone: CloneStream<_> = receiver.into();

    let expect_numbers = (0..N_ITEMS_SENT).collect::<Vec<_>>();

    let start = Instant::now();

    let get_ready = start + std::time::Duration::from_millis(10) * N_STREAM_CLONES as u32;

    trace!("Waiting a few milliseconds before forking");
    let wait_for_receive_all = join_all((0..N_STREAM_CLONES).map(|i| {
        let receive_all = template_clone.clone().collect::<Vec<_>>();
        let expected = expect_numbers.clone();
        tokio::spawn(async move {
            trace!(
                "Waiting a few milliseconds before receiving from fork {}",
                i + 1
            );
            sleep_until(get_ready).await;
            trace!("Fork {} receiving", i + 1);
            assert_eq!(
                receive_all.await,
                expected,
                "Fork {} received unexpected items",
                i + 1
            );
        })
    }));

    trace!("Waiting a few milliseconds before sending");

    let send = tokio::spawn(async move {
        trace!("Waiting a few milliseconds before sending");
        sleep_until(get_ready + Duration::from_millis(10)).await;
        trace!("Sending items");
        stream::iter(0..N_ITEMS_SENT)
            .map(Ok)
            .forward(sender)
            .await
            .unwrap();
    });

    trace!("Waiting for all tasks to finish");

    let _ = join!(send, wait_for_receive_all);
}
