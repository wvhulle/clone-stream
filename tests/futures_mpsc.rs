use clone_stream::CloneStream;
use futures::{
    SinkExt, StreamExt,
    executor::{ThreadPool, block_on},
    future::join_all,
    join,
    task::SpawnExt,
};
#[test]
fn mass_send() {
    let (mut sender, rx) = futures::channel::mpsc::unbounded::<usize>();

    let template_clone: CloneStream<_> = rx.into();
    let pool = ThreadPool::builder().create().unwrap();
    let n_clones = 500;

    let n_items = 5000;

    let expected_received = (0..n_items).collect::<Vec<_>>();

    let mut join_handles = Vec::new();

    for i in 0..n_clones {
        let receive_fut = template_clone.clone().collect::<Vec<_>>();
        let expected = expected_received.clone();
        let handle = pool
            .spawn_with_handle(async move {
                assert_eq!(
                    receive_fut.await,
                    expected,
                    "Fork {} received unexpected items",
                    i + 1
                );
            })
            .unwrap();
        join_handles.push(handle);
    }

    let send = pool
        .spawn_with_handle(async move {
            for i in 0..n_items {
                sender.send(i).await.unwrap();
            }
        })
        .unwrap();

    block_on(async move {
        join!(send, join_all(join_handles));
    });
}
