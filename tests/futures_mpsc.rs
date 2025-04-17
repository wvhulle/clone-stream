use std::sync::{Arc, Mutex};

use clone_stream::CloneStream;

mod barrier_future;

use barrier_future::{BarrierFuture, PendingFutureObserver, SharedBarriedFuture};
use futures::{
    SinkExt, StreamExt,
    executor::{ThreadPool, block_on},
    future::join_all,
    join,
    task::SpawnExt,
};
use log::trace;
#[test]
fn mass_send() {
    let (mut sender, rx) = futures::channel::mpsc::unbounded::<usize>();

    let template_clone: CloneStream<_> = rx.into();
    let pool = ThreadPool::builder().create().unwrap();
    let n_clones = 500;

    let n_items = 500;

    let expected_received = (0..n_items).collect::<Vec<_>>();

    let mut join_handles = Vec::new();

    let wakers = Arc::new(Mutex::new(SharedBarriedFuture::new(n_clones)));

    let observer = PendingFutureObserver::new(wakers.clone());
    for i in 0..n_clones {
        let receive_fut =
            BarrierFuture::new(template_clone.clone().collect::<Vec<_>>(), wakers.clone());
        let expected = expected_received.clone();
        let handle = pool
            .spawn_with_handle(async move {
                trace!("Task {} started listening", i + 1);

                assert_eq!(
                    receive_fut.await,
                    expected,
                    "Fork {} received unexpected items",
                    i + 1
                );
                trace!("Task {} finished listening", i + 1);
            })
            .unwrap();
        join_handles.push(handle);
    }

    let send = pool
        .spawn_with_handle(async move {
            // info!("Waiting for forks to start waiting.");
            trace!("Waiting for forks to start waiting.");
            observer.await;

            trace!("All forks are waiting.");
            for i in 0..n_items {
                sender.send(i).await.unwrap();
            }
        })
        .unwrap();

    block_on(async move {
        join!(send, join_all(join_handles));
    });
}
