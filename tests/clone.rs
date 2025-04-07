mod mock;

use std::task::Poll;

use futures::{SinkExt, executor::block_on};
use mock::ForkAsyncMockSetup;
const N_FORKS: u32 = 100;

#[tokio::test]
async fn undelivered() {
    let mut setup = ForkAsyncMockSetup::<usize, 1>::new();

    assert_eq!(setup.forks[0].next(), Poll::Pending);
}

#[test]
fn simple() {
    let ForkAsyncMockSetup {
        mut sender, forks, ..
    } = ForkAsyncMockSetup::<(), 2>::new();

    let [mut fork1, mut fork2] = forks;

    assert_eq!(fork1.next(), Poll::Pending);
    assert_eq!(fork2.next(), Poll::Pending);

    block_on(async {
        let _ = sender.send(()).await;
    });
    assert_eq!(fork1.next(), Poll::Ready(Some(())));
    assert_eq!(fork1.next(), Poll::Pending);

    assert_eq!(fork2.next(), Poll::Ready(Some(())));

    assert_eq!(fork2.next(), Poll::Pending);
}

#[test]
fn second_pending() {
    let ForkAsyncMockSetup {
        mut sender, forks, ..
    } = ForkAsyncMockSetup::<(), 2>::new();

    let [mut fork1, mut fork2] = forks;

    assert_eq!(fork1.next(), Poll::Pending);
    block_on(async {
        let _ = sender.send(()).await;
    });
    assert_eq!(fork1.next(), Poll::Ready(Some(())));
    assert_eq!(fork1.next(), Poll::Pending);

    assert_eq!(fork2.next(), Poll::Pending);
}

#[test]
fn second_later_ready() {
    let ForkAsyncMockSetup {
        mut sender, forks, ..
    } = ForkAsyncMockSetup::<(), 2>::new();

    let [mut fork1, mut fork2] = forks;

    assert_eq!(fork1.next(), Poll::Pending);
    block_on(async {
        let _ = sender.send(()).await;
    });
    assert_eq!(fork1.next(), Poll::Ready(Some(())));
    assert_eq!(fork2.next(), Poll::Pending);

    block_on(async {
        let _ = sender.send(()).await;
    });

    assert_eq!(fork2.next(), Poll::Ready(Some(())));

    assert_eq!(fork1.next(), Poll::Ready(Some(())));

    assert_eq!(fork1.next(), Poll::Pending);
    assert_eq!(fork2.next(), Poll::Pending);
}

#[test]
fn multi() {
    let ForkAsyncMockSetup {
        mut sender, forks, ..
    } = ForkAsyncMockSetup::<(), 2>::new();

    let [mut fork1, _] = forks;

    assert_eq!(fork1.next(), Poll::Pending);
    block_on(async {
        let _ = sender.feed(()).await;
        let _ = sender.feed(()).await;
        let _ = sender.flush().await;
    });
    assert_eq!(fork1.next(), Poll::Ready(Some(())));
    assert_eq!(fork1.next(), Poll::Ready(Some(())));
}

#[test]
fn multi_both() {
    let ForkAsyncMockSetup {
        mut sender, forks, ..
    } = ForkAsyncMockSetup::<(), 2>::new();

    let [mut fork1, mut fork2] = forks;

    assert_eq!(fork1.next(), Poll::Pending);
    assert_eq!(fork2.next(), Poll::Pending);

    block_on(async {
        let _ = sender.feed(()).await;
        let _ = sender.feed(()).await;
        let _ = sender.flush().await;
    });

    assert_eq!(fork1.next(), Poll::Ready(Some(())));
    assert_eq!(fork1.next(), Poll::Ready(Some(())));

    assert_eq!(fork1.next(), Poll::Pending);

    assert_eq!(fork2.next(), Poll::Ready(Some(())));
    assert_eq!(fork2.next(), Poll::Ready(Some(())));

    assert_eq!(fork2.next(), Poll::Pending);
}

#[test]
fn multi_both_interleave() {
    let ForkAsyncMockSetup {
        mut sender, forks, ..
    } = ForkAsyncMockSetup::<(), 2>::new();

    let [mut fork1, mut fork2] = forks;

    assert_eq!(fork1.next(), Poll::Pending);
    assert_eq!(fork2.next(), Poll::Pending);

    block_on(async {
        let _ = sender.feed(()).await;
        let _ = sender.feed(()).await;
        let _ = sender.flush().await;
    });

    assert_eq!(fork1.next(), Poll::Ready(Some(())));
    assert_eq!(fork2.next(), Poll::Ready(Some(())));

    assert_eq!(fork1.next(), Poll::Ready(Some(())));

    assert_eq!(fork1.next(), Poll::Pending);

    assert_eq!(fork2.next(), Poll::Ready(Some(())));

    assert_eq!(fork2.next(), Poll::Pending);
}
