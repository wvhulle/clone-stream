use core::time::Duration;
mod util;
use clone_stream::CloneStream;
use futures::{StreamExt, future::join_all, join, stream};
use log::info;
use tokio::time::{Instant, sleep_until};
use util::log;

const N_STREAM_CLONES: usize = 3;

const N_ITEMS_SENT: usize = 2;

#[tokio::test]
async fn mass_send() {
    // log();
    info!("Starting mass_send benchmark");
    let (sender, receiver) = futures::channel::mpsc::unbounded::<usize>();

    let template_clone: CloneStream<_> = receiver.into();
    info!("Created template clone from receiver");

    let expect_numbers = (0..N_ITEMS_SENT).collect::<Vec<_>>();
    info!("Expected numbers: {expect_numbers:?}");

    let start = Instant::now();

    let every_clone_starts_listen =
        start + std::time::Duration::from_millis(10) * N_STREAM_CLONES as u32;

    let start_sending =
        every_clone_starts_listen + Duration::from_millis(10) * N_STREAM_CLONES as u32;

    info!(
        "Timing: start={start:?}, clones_listen={every_clone_starts_listen:?}, start_sending={start_sending:?}"
    );

    let wait_for_receive_all = join_all((0..N_STREAM_CLONES).map(|i| {
        let i = i + 1;
        let clone = template_clone.clone();
        async move {
            info!("Creating clone {i}");
            tokio::spawn(async move {
                info!("Clone {i}: waiting to start listening...");
                sleep_until(every_clone_starts_listen).await;
                info!("Clone {i}: starting to collect items");
                let result = clone.collect::<Vec<_>>().await;
                info!("Clone {i}: collected {} items", result.len());
                result
            })
            .await
        }
    }));

    let send = tokio::spawn(async move {
        info!("Sender: waiting to start sending...");
        sleep_until(start_sending).await;
        info!("Sender: starting to send {} items", N_ITEMS_SENT);
        let result = stream::iter(0..N_ITEMS_SENT).map(Ok).forward(sender).await;
        info!("Sender: finished sending, result={result:?}");
        result.unwrap();
    });

    info!("Starting concurrent execution of sender and receivers");
    let (_, outputs) = join!(send, wait_for_receive_all);
    info!("All tasks completed, checking outputs");

    for (i, output) in outputs.into_iter().enumerate() {
        let result = output.unwrap();
        info!(
            "Clone {} received {} items: {result:?}",
            i + 1,
            result.len()
        );
        assert_eq!(
            result,
            expect_numbers,
            "Clone {} received unexpected items",
            i + 1
        );
    }
    info!("All assertions passed!");
}
