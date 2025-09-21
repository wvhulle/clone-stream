use core::pin::Pin;
use core::task::Poll;

use clone_stream::CloneStream;
use futures::stream::Stream;
use futures::{StreamExt, future::join_all, future::poll_fn, stream};
use tokio::sync::oneshot;

const N_STREAM_CLONES: usize = 64;
const N_ITEMS_SENT: usize = 10_0;

#[tokio::main] // keep it simple and single-threaded
async fn main() {
    // Gate base production until all clones have polled once
    let (start_tx, start_rx) = oneshot::channel::<()>();
    let base = futures::stream::pending::<usize>()
        .take_until(async {
            let _ = start_rx.await;
        })
        .chain(stream::iter(0..N_ITEMS_SENT));
    let template: CloneStream<_> = base.into();

    let expected = (0..N_ITEMS_SENT).collect::<Vec<_>>();
    let clones: Vec<_> = (0..N_STREAM_CLONES).map(|_| template.clone()).collect();

    // Spawn collectors with a one-time pre-poll to register interest
    let mut ready_receivers = Vec::with_capacity(N_STREAM_CLONES);
    let tasks: Vec<_> = clones
        .into_iter()
        .enumerate()
        .map(|(idx, mut clone)| {
            let (ready_tx, ready_rx) = oneshot::channel::<()>();
            ready_receivers.push(ready_rx);
            tokio::spawn(async move {
                // One initial poll to move clone into a pending state with a waker
                poll_fn(|cx| {
                    let _ = Pin::new(&mut clone).poll_next(cx);
                    Poll::Ready(())
                })
                .await;
                let _ = ready_tx.send(());

                // Then fully collect
                let got = clone.collect::<Vec<_>>().await;
                (idx, got)
            })
        })
        .collect();

    // Ensure all clones have been polled once
    for rx in ready_receivers {
        let _ = rx.await;
    }

    // Start producing items
    let _ = start_tx.send(());

    // Gather results
    let results = join_all(tasks).await;
    for r in results {
        let (i, got) = r.expect("collector task panicked");
        if got != expected {
            eprintln!(
                "clone {i} mismatch: got len {} vs expected {}",
                got.len(),
                expected.len()
            );
            // Print a small prefix/suffix for quick inspection
            eprintln!(
                "first 10 got: {:?}",
                &got.iter().take(10).collect::<Vec<_>>()
            );
            eprintln!(
                "last 10 got: {:?}",
                &got.iter().rev().take(10).collect::<Vec<_>>()
            );
            std::process::exit(1);
        }
    }

    println!("manual bench OK: all {N_STREAM_CLONES} clones received {N_ITEMS_SENT} items");
}
