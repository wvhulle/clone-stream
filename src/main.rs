use core::{
    future::poll_fn,
    sync::atomic::{AtomicUsize, Ordering},
    task::Poll,
};
use std::sync::Arc;

use forked_stream::ForkStream;
use futures::{
    FutureExt, SinkExt, StreamExt,
    executor::{ThreadPool, block_on},
    join, select,
    task::SpawnExt,
};
use log::{info, trace};

static forks_started: AtomicUsize = AtomicUsize::new(0);
static forks_waiting: AtomicUsize = AtomicUsize::new(0);

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .format_line_number(true)
        .format_file(true)
        .init();
    info!("Starting test");
    let (mut sender, rx) = futures::channel::mpsc::unbounded();

    let wait_until_n_waiting = move |n| {
        poll_fn(move |cx| {
            if forks_waiting.load(Ordering::SeqCst) >= n {
                Poll::Ready(())
            } else {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
    };

    let mut fork_0 = rx.fork();
    let mut fork_1 = fork_0.clone();

    let pool = ThreadPool::builder()
        .after_start(move |_| {
            forks_started.fetch_add(1, Ordering::SeqCst);
        })
        .create()
        .unwrap();

    let send = pool
        .spawn_with_handle(async move {
            info!("Waiting for forks to start waiting.");
            wait_until_n_waiting(2).await;

            info!("Sender started sending");
            sender.send('a').await.unwrap();
            trace!("Sender sent 'a'");
            sender.send('b').await.unwrap();
        })
        .unwrap();

    let receive_0 = pool
        .spawn_with_handle(async move {
            join!(
                async move {
                    info!("Waiting until one other fork is waiting.");

                    wait_until_n_waiting(1).await;
                    info!("At least one fork is waiting. Ready to receive on 0");
                },
                async move {
                    forks_waiting.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(fork_0.next().await, Some('a'));
                    assert_eq!(fork_0.next().await, Some('b'));
                    trace!("Fork 0 received 'b'");
                    assert_eq!(fork_0.next().await, None);
                    trace!("Fork 0 finished waiting");
                }
            )
        })
        .unwrap();

    let receive_1 = pool
        .spawn_with_handle(async move {
            join!(
                async move {
                    info!("Waiting until one other fork is waiting.");
                    wait_until_n_waiting(1).await;
                    info!("At least one fork is waiting. Ready to receive on 1");
                },
                async move {
                    forks_waiting.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(fork_1.next().await, Some('a'));
                    trace!("Fork 1 received 'a'");
                    assert_eq!(fork_1.next().await, Some('b'));
                    trace!("Fork 1 received 'b'");
                    assert_eq!(fork_1.next().await, None);
                    trace!("Fork 1 finished waiting");
                }
            )
        })
        .unwrap();

    block_on(async move {
        join!(send, receive_0, receive_1);
    });
}
