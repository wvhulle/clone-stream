use core::time::Duration;
use std::sync::Arc;

use clone_stream::{CloneStream, clean_log::log};
use futures::{FutureExt, SinkExt, StreamExt, future::try_join_all, join, stream};
use log::{info, warn};
use tokio::time::{Instant, sleep_until};

const N_STREAM_CLONES: usize = 2;

const N_ITEMS_SENT: usize = 2;

#[tokio::test]
async fn mass_send() {
    log();
    let (sender, receiver) = futures::channel::mpsc::unbounded::<usize>();

    let template_clone: CloneStream<_> = receiver.into();

    let expect_numbers = (0..N_ITEMS_SENT).collect::<Vec<_>>();

    let ready_to_send = Arc::new(tokio::sync::Barrier::new(N_STREAM_CLONES + 1));
    let wait_for_receive_all = try_join_all((0..N_STREAM_CLONES).map(|i| {
        let mut clone = template_clone.clone();
        let expected = expect_numbers.clone();
        let ready_to_send = ready_to_send.clone();
        info!("Spawning clone {}", i + 1);
        tokio::spawn(async move {
            info!("Spawned clone {}", i + 1);
            let first = clone.next().now_or_never();
            match &first {
                Some(item) => info!("Clone {} polled first item and received {:?}", i + 1, item),
                None => info!("Clone {} polled first item and got Pending", i + 1),
            }
            ready_to_send.wait().await;
            info!("Clone {} passed barrier", i + 1);
            let mut all_items = clone.collect::<Vec<_>>().await;
            info!("Clone {} collected all items: {:?}", i + 1, all_items);
            if let Some(item) = first {
                all_items.insert(0, item.unwrap());
            }
            assert_eq!(
                all_items,
                expected,
                "Clone {} received unexpected items",
                i + 1
            );
        })
    }));

    tokio::spawn(async move {
        ready_to_send.wait().await;
        warn!("Send task passed barrier, sending items");
        stream::iter(0..N_ITEMS_SENT)
            .map(Ok)
            .forward(sender)
            .await
            .unwrap();
    });

    wait_for_receive_all.await.unwrap();
}

#[tokio::test]
async fn mass_spaced_send() {
    let (mut sender, receiver) = futures::channel::mpsc::unbounded::<usize>();

    let template_clone: CloneStream<_> = receiver.into();

    let expect_numbers = (0..N_ITEMS_SENT).collect::<Vec<_>>();

    let start = Instant::now();

    let get_ready = start + std::time::Duration::from_millis(10) * N_STREAM_CLONES as u32;

    let wait_for_receive_all = try_join_all((0..N_STREAM_CLONES).map(|i| {
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

    let (send, receivers_task_results) = join!(send, wait_for_receive_all);

    send.expect("Send task panicked");
    receivers_task_results.expect("Receive task panicked");
}
