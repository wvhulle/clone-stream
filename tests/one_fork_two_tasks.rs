use std::task::Poll;

use forked_stream::ForkAsyncMockSetup;
use futures::{SinkExt, executor::block_on};

#[test]
fn first_waker_unaffected() {
    let mut setup = ForkAsyncMockSetup::<2>::new(1);

    assert_eq!(setup.poll_stream_with_waker(0, 0), Poll::Pending);

    block_on(async {
        let _ = setup.sender.send(0).await;
    });
    assert_eq!(setup.poll_stream_with_waker(0, 0), Poll::Ready(Some(0)));
}

#[test]
fn second_waker_also_consumed() {
    let mut setup = ForkAsyncMockSetup::<2>::new(1);

    assert_eq!(setup.poll_stream_with_waker(0, 0), Poll::Pending);
    assert_eq!(setup.poll_stream_with_waker(0, 1), Poll::Pending);
    block_on(async {
        let _ = setup.sender.send(0).await;
    });
    assert_eq!(setup.poll_stream_with_waker(0, 0), Poll::Ready(Some(0)));
    assert_eq!(setup.poll_stream_with_waker(0, 1), Poll::Pending);
}

#[test]
fn first_waker_also_consumed() {
    let mut setup = ForkAsyncMockSetup::<2>::new(1);

    assert_eq!(setup.poll_stream_with_waker(0, 0), Poll::Pending);
    assert_eq!(setup.poll_stream_with_waker(0, 1), Poll::Pending);
    block_on(async {
        let _ = setup.sender.send(0).await;
    });
    assert_eq!(setup.poll_stream_with_waker(0, 0), Poll::Ready(Some(0)));
    assert_eq!(setup.poll_stream_with_waker(0, 1), Poll::Pending);
}
