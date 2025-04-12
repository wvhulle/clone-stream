use forked_stream::{ForkStream, enable_debug_log};
use futures::{
    FutureExt, SinkExt, StreamExt,
    executor::{ThreadPool, block_on},
    join,
    task::SpawnExt,
};

#[test]
fn p() {
    enable_debug_log();
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
                println!("Sender sending {}", i);
                sender.send(Some(i)).await;
            }
            sender.send(None).await;
            println!("Sender finished sending");
        })
        .unwrap();

    let receive_fork_0 = pool
        .spawn_with_handle(async move {
            println!("Fork 0 started listening");
            let mut seen = Vec::new();
            fork_0
                .for_each(|item| {
                    println!("Fork 0 received {:?}", item);
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
                    println!("Fork 1 received {:?}", item);
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
