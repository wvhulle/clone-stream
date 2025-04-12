use forked_stream::ForkStream;
use futures::{
    FutureExt, SinkExt, StreamExt,
    executor::{ThreadPool, block_on},
    join,
    task::SpawnExt,
};

#[test]
fn one_listens_receives_two() {
    let (mut sender, rx) = futures::channel::mpsc::unbounded();

    let mut fork_0 = rx.fork();
    let mut fork_1 = fork_0.clone();

    assert!(fork_0.next().now_or_never().is_none());

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

    assert_eq!(fork_1.next().now_or_never(), Some(None));
    assert_eq!(fork_1.next().now_or_never(), Some(None));
}
