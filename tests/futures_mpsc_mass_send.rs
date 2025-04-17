use std::{thread::sleep, time::Duration};

use forked_stream::ForkStream;
use futures::{
    FutureExt, SinkExt, StreamExt,
    executor::{ThreadPool, block_on},
    join,
    stream::FuturesUnordered,
    task::SpawnExt,
};

#[test]
fn two_forks() {
    let (mut sender, rx) = futures::channel::mpsc::unbounded();

    let mut fork_0 = rx.fork();
    let mut fork_1 = fork_0.clone();

    let n = 100;

    let mut expected = (0..n).map(Some).collect::<Vec<_>>();
    expected.push(None);

    let expected_fork_0 = expected.clone();

    let pool = ThreadPool::new().unwrap();

    assert!(fork_0.next().now_or_never().is_none());
    assert!(fork_1.next().now_or_never().is_none());

    let send = pool
        .spawn_with_handle(async move {
            println!("Sender started sending");
            for i in 0..n {
                sender.send(Some(i)).await.unwrap();
            }
            sender.send(None).await.unwrap();
            println!("Sender finished sending");
        })
        .unwrap();

    let receive_fork_0 = pool
        .spawn_with_handle(async move {
            println!("Fork 0 started listening");
            let mut seen = Vec::new();
            fork_0
                .for_each(|item| {
                    seen.push(item);
                    futures::future::ready(())
                })
                .await;
            println!("Fork 0 finished listening");
            assert_eq!(seen, expected_fork_0);
        })
        .unwrap();

    let expected_fork_1 = expected.clone();

    let receive_fork_1 = pool
        .spawn_with_handle(async move {
            println!("Fork 1 started listening");
            let mut seen = Vec::new();
            fork_1
                .for_each(|item| {
                    seen.push(item);
                    futures::future::ready(())
                })
                .await;
            println!("Fork 1 finished listening");
            assert_eq!(seen, expected_fork_1.clone());
        })
        .unwrap();

    block_on(async move {
        join!(send, receive_fork_0, receive_fork_1);
    });
}

#[test]
fn many_forks() {
    let n_forks = 100;
    let n_items = 10;

    let (mut sender, receiver) = futures::channel::mpsc::unbounded();

    let template_fork = receiver.fork();

    let mut forks = (0..n_forks)
        .map(|_| template_fork.clone())
        .collect::<Vec<_>>();

    let mut expected = (0..n_items).map(Some).collect::<Vec<_>>();
    expected.push(None);

    let pool = ThreadPool::new().unwrap();

    for unactive_fork in &mut forks {
        assert!(unactive_fork.next().now_or_never().is_none());
    }

    let send = pool
        .spawn_with_handle(async move {
            for i in 0..n_items {
                sleep(Duration::from_micros(200));
                sender.send(Some(i)).await.unwrap();
            }
            sender.send(None).await.unwrap();
        })
        .unwrap();

    let ((), list_of_vec_of_seen_values) = block_on(async move {
        let collect = (forks.into_iter().map(|fork| {
            pool.spawn_with_handle(async move { fork.collect().await })
                .unwrap()
        }))
        .collect::<FuturesUnordered<_>>()
        .collect::<Vec<Vec<_>>>();

        join!(send, collect)
    });

    for seen in list_of_vec_of_seen_values {
        assert_eq!(seen, expected);
    }
}
