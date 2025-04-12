use forked_stream::ForkStream;
use futures::{
    FutureExt, SinkExt, StreamExt,
    executor::{ThreadPool, block_on},
    join,
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
            assert_eq!(fork_0.next().now_or_never(), Some(None));
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

    let pool = ThreadPool::new().unwrap();

    assert!(fork_0.next().now_or_never().is_none());
    assert!(fork_1.next().now_or_never().is_none());

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

            assert_eq!(fork_1.next().await, Some('a'));
            assert_eq!(fork_1.next().await, Some('b'));
            assert_eq!(fork_1.next().await, None);
        })
        .unwrap();

    block_on(async move {
        join!(send, receive);
    });
}
