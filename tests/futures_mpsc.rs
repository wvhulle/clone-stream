use clone_stream::CloneStream;
use futures::{
    StreamExt,
    executor::{ThreadPool, block_on},
    future::join_all,
    join, stream,
    task::SpawnExt,
};

const N_STREAM_CLONES: usize = 500;

const N_ITEMS_SENT: usize = 5000;

#[test]
fn mass_send() {
    let (sender, receiver) = futures::channel::mpsc::unbounded::<usize>();

    let template_clone: CloneStream<_> = receiver.into();
    let pool = ThreadPool::builder().create().unwrap();

    let expect_numbers = (0..N_ITEMS_SENT).collect::<Vec<_>>();

    let wait_for_receive_all = join_all((0..N_STREAM_CLONES).map(|i| {
        let receive_all = template_clone.clone().collect::<Vec<_>>();
        let expected = expect_numbers.clone();
        pool.spawn_with_handle(async move {
            assert_eq!(
                receive_all.await,
                expected,
                "Fork {} received unexpected items",
                i + 1
            );
        })
        .unwrap()
    }));

    let send = pool
        .spawn_with_handle(async move {
            (stream::iter(0..N_ITEMS_SENT).map(Ok))
                .forward(sender)
                .await
                .unwrap();
        })
        .unwrap();

    block_on(async move {
        join!(send, wait_for_receive_all);
    });
}
