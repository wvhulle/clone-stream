use std::sync::Arc;

use clone_stream::CloneStream;
use futures::{FutureExt, SinkExt, StreamExt, future::try_join_all};
use tokio::sync::Barrier;

const N_CLONES: usize = 5;
const N_ITEMS: usize = 10;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let (mut sender, receiver) = futures::channel::mpsc::unbounded::<usize>();
    let template: CloneStream<_> = receiver.into();

    let expected: Vec<_> = (0..N_ITEMS).collect();

    // Barrier ensures all clones poll once before sender starts
    let barrier = Arc::new(Barrier::new(N_CLONES + 1));

    let receivers = try_join_all((0..N_CLONES).map(|i| {
        let mut clone = template.clone();
        let expected = expected.clone();
        let barrier = barrier.clone();

        tokio::spawn(async move {
            // Poll once (will be Pending since no items yet) - registers this clone as
            // waiting
            let first = clone.next().now_or_never();
            println!("Clone {i}: first poll = {first:?}");

            // Signal ready and wait for all clones + sender
            barrier.wait().await;

            // Collect remaining items
            let mut items = clone.collect::<Vec<_>>().await;

            // If first poll got an item, insert it
            if let Some(Some(item)) = first {
                items.insert(0, item);
            }

            println!("Clone {i}: received {items:?}");
            assert_eq!(items, expected, "Clone {i} received unexpected items");
            items
        })
    }));

    // Sender waits at barrier then sends items - NOT awaited
    tokio::spawn(async move {
        barrier.wait().await;
        println!("Sender: sending {N_ITEMS} items...");
        for i in 0..N_ITEMS {
            sender.send(i).await.unwrap();
        }
    });

    // Only wait for receivers
    receivers.await.unwrap();

    println!("All {N_CLONES} clones received all {N_ITEMS} items.");
}
