use forked_stream::{ForkStream, enable_debug_log};
use futures::{
    FutureExt, SinkExt, StreamExt,
    executor::{ThreadPool, block_on},
    join,
    task::SpawnExt,
};

#[test]
fn m() {
    enable_debug_log();
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
fn n() {
    enable_debug_log();
    let (mut sender, rx) = futures::channel::mpsc::unbounded();

    let mut fork_0 = rx.fork();
    let mut fork_1 = fork_0.clone();

    assert!(fork_0.next().now_or_never().is_none());
    assert!(fork_1.next().now_or_never().is_none());

    let pool = ThreadPool::new().unwrap();

    let send = pool
        .spawn_with_handle(async move {
            println!("[Sender] Sending a");
            sender.send('a').await.unwrap();

            println!("[Sender] Sent a");
            println!("[Sender] Sending b");
            sender.send('b').await.unwrap();
            println!("[Sender] Sent b");
        })
        .unwrap();

    let receive = pool
        .spawn_with_handle(async move {
            println!("[Receiver] Receiving a");
            assert_eq!(fork_0.next().await, Some('a'));
            println!("[Receiver] Receiving b");
            assert_eq!(fork_0.next().await, Some('b'));
            println!("[Receiver] Receiving None");
            assert_eq!(fork_0.next().now_or_never(), Some(None));
        })
        .unwrap();

    println!("Waiting for both sender and receiver");
    block_on(async move {
        join!(send, receive);
    });

    assert_eq!(fork_1.next().now_or_never(), Some(Some('a')));
    assert_eq!(fork_1.next().now_or_never(), Some(Some('b')));
    assert_eq!(fork_1.next().now_or_never(), Some(None));
}

#[test]
fn o() {
    enable_debug_log();
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
