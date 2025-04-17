use forked_stream::ForkStream;
use futures::{
    FutureExt, SinkExt, StreamExt,
    executor::{ThreadPool, block_on},
    future::join,
    join, select,
    task::SpawnExt,
};

#[test]
fn two_listen_one_pulling() {
    let (mut sender, rx) = futures::channel::mpsc::unbounded();

    let mut fork_0 = rx.fork();
    let mut fork_1 = fork_0.clone();

    assert!(fork_0.next().now_or_never().is_none());
    assert!(fork_1.next().now_or_never().is_none());

    let pool = ThreadPool::new().unwrap();

    let send = pool
        .spawn_with_handle(async move {
            sender.send('a').await.unwrap();

            sender.send('b').await.unwrap();
        })
        .unwrap();

    let receive = pool
        .spawn_with_handle(async move {
            assert_eq!(fork_0.next().await, Some('a'));
            assert_eq!(fork_0.next().await, Some('b'));
            assert_eq!(fork_0.next().await, None);
        })
        .unwrap();

    block_on(async move {
        join!(send, receive);
    });

    assert_eq!(fork_1.next().now_or_never(), Some(Some('a')));
    assert_eq!(fork_1.next().now_or_never(), Some(Some('b')));
    assert_eq!(fork_1.next().now_or_never(), Some(None));
}

#[test]
fn two_listen_two_pulling() {
    let (mut sender, rx) = futures::channel::mpsc::unbounded();

    let mut fork_0 = rx.fork();
    let mut fork_1 = fork_0.clone();

    let (tx, rx) = std::sync::mpsc::channel();

    let pool = ThreadPool::builder()
        .after_start(move |n| {
            let _ = tx.send(n);
        })
        .create()
        .unwrap();

    assert!(fork_0.next().now_or_never().is_none());
    assert!(fork_1.next().now_or_never().is_none());

    let send = pool
        .spawn_with_handle(async move {
            let mut first_started = false;
            let mut second_started = false;
            loop {
                let n = rx.recv().unwrap();
                if n == 1 {
                    first_started = true;
                } else if n == 2 {
                    second_started = true;
                }

                if first_started && second_started {
                    break;
                }
            }

            sender.send('a').await.unwrap();

            sender.send('b').await.unwrap();
        })
        .unwrap();

    let receive_0 = pool
        .spawn_with_handle(async move {
            assert_eq!(fork_0.next().await, Some('a'));
            assert_eq!(fork_0.next().await, Some('b'));
            assert_eq!(fork_0.next().await, None);
        })
        .unwrap();

    let receive_1 = pool
        .spawn_with_handle(async move {
            assert_eq!(fork_1.next().await, Some('a'));
            assert_eq!(fork_1.next().await, Some('b'));
            assert_eq!(fork_1.next().await, None);
        })
        .unwrap();

    block_on(async move {
        join!(send, receive_0, receive_1);
    });
}
