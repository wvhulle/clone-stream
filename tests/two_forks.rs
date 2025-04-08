mod mock;

use std::task::Poll;

use futures::{SinkExt, executor::block_on};
use log::info;
use mock::ForkAsyncMockSetup;

#[test]
fn nothing() {
    let mut setup = ForkAsyncMockSetup::<2, 1>::new();

    assert_eq!(setup.poll_stream(0), Poll::Pending);
    assert_eq!(setup.poll_stream(1), Poll::Pending);
}

#[test]
fn one_pending_send_one() {
    let mut setup = ForkAsyncMockSetup::<2, 1>::new();

    assert_eq!(setup.poll_stream(0), Poll::Pending);

    block_on(async {
        let _ = setup.sender.send(0).await;
    });
    assert_eq!(setup.poll_stream(0), Poll::Ready(Some(0)));

    assert_eq!(setup.poll_stream(1), Poll::Pending);
}

#[test]
fn both_pending_send_one() {
    let mut setup = ForkAsyncMockSetup::<2, 1>::new();

    assert_eq!(setup.poll_stream(0), Poll::Pending);
    assert_eq!(setup.poll_stream(1), Poll::Pending);
    block_on(async {
        let _ = setup.sender.send(0).await;
    });
    assert_eq!(setup.poll_stream(0), Poll::Ready(Some(0)));

    assert_eq!(setup.poll_stream(1), Poll::Ready(Some(0)));
}

#[test]
fn both_pending_send_two_receive_one() {
    let mut setup = ForkAsyncMockSetup::<2, 1>::new();

    assert_eq!(setup.poll_stream(0), Poll::Pending);
    assert_eq!(setup.poll_stream(1), Poll::Pending);
    block_on(async {
        let _ = setup.sender.send(0).await;
    });
    assert_eq!(setup.poll_stream(0), Poll::Ready(Some(0)));
    block_on(async {
        let _ = setup.sender.send(0).await;
    });
    assert_eq!(setup.poll_stream(0), Poll::Ready(Some(0)));
}
#[test]
fn both_pending_send_two_receive_one_late() {
    let mut setup = ForkAsyncMockSetup::<2, 1>::new();

    assert_eq!(setup.poll_stream(0), Poll::Pending);
    assert_eq!(setup.poll_stream(1), Poll::Pending);
    block_on(async {
        let _ = setup.sender.send(0).await;
    });
    assert_eq!(setup.poll_stream(0), Poll::Ready(Some(0)));
    block_on(async {
        let _ = setup.sender.send(1).await;
    });
    assert_eq!(setup.poll_stream(1), Poll::Ready(Some(0)));
}

#[test]
fn both_pending_send_two_receive_two_twice() {
    let mut setup = ForkAsyncMockSetup::<2, 1>::new();

    assert_eq!(setup.poll_stream(0), Poll::Pending);
    assert_eq!(setup.poll_stream(1), Poll::Pending);
    block_on(async {
        let _ = setup.sender.start_send(0);
        let _ = setup.sender.start_send(1);
        let _ = setup.sender.flush().await;
    });

    info!("Polling the first stream for the first time.");
    assert_eq!(setup.poll_stream(0), Poll::Ready(Some(0)));
    info!("Polling the second stream for the first time.");
    assert_eq!(setup.poll_stream(1), Poll::Ready(Some(0)));
    info!("Polling the first stream for the second time.");
    assert_eq!(setup.poll_stream(0), Poll::Ready(Some(1)));
    info!("Polling the second stream for the second time.");
    assert_eq!(setup.poll_stream(1), Poll::Ready(Some(1)));
}
