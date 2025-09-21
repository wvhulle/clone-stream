use core::time::Duration;

use clone_stream::CloneStream;
use futures::{SinkExt, StreamExt, future::join_all, join, stream};
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

    let wait_for_receive_all = join_all((0..N_STREAM_CLONES).map(|i| {
        let receive_all = template_clone.clone().collect::<Vec<_>>();
        let expected = expect_numbers.clone();
        tokio::spawn(async move {
            sleep_until(get_ready).await;
            assert_eq!(
                receive_all.await,
                expected,
                "Clone {} received unexpected items",
                i + 1
            );
        })
    }));

    let send = tokio::spawn(async move {
        sleep_until(get_ready + Duration::from_millis(10)).await;
        stream::iter(0..N_ITEMS_SENT)
            .map(Ok)
            .forward(sender)
            .await
            .unwrap();
    });

    let _ = join!(send, wait_for_receive_all);
}

#[test_log::test(tokio::test)]
async fn mass_spaced_send() {
    let (mut sender, receiver) = futures::channel::mpsc::unbounded::<usize>();

    let template_clone: CloneStream<_> = receiver.into();

    let expect_numbers = (0..N_ITEMS_SENT).collect::<Vec<_>>();

    let start = Instant::now();

    let get_ready = start + std::time::Duration::from_millis(10) * N_STREAM_CLONES as u32;

    let wait_for_receive_all = join_all((0..N_STREAM_CLONES).map(|i| {
        let receive_all = template_clone.clone().collect::<Vec<_>>();
        let expected = expect_numbers.clone();
        tokio::spawn(async move {
            sleep_until(get_ready).await;
            assert_eq!(
                receive_all.await,
                expected,
                "Fork {} received unexpected items",
                i + 1
            );
        })
    }));

    let send = tokio::spawn(async move {
        sleep_until(get_ready + Duration::from_millis(10)).await;
        for i in 0..N_ITEMS_SENT {
            sleep_until(start + Duration::from_millis(10 * i as u64)).await;
            sender.send(i).await.unwrap();
        }
    });

    let _ = join!(send, wait_for_receive_all);
}
