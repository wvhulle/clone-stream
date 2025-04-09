use std::task::Poll;

use forked_stream::TestSetup;
use futures::{SinkExt, executor::block_on};

#[test]
fn first_waker_unaffected() {
    let mut setup: TestSetup<2> = TestSetup::new(1);

    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Pending);

    block_on(async {
        let _ = setup.sender.send(0).await;
    });
    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Ready(Some(0)));
}

#[test]
fn second_waker_also_consumed() {
    let mut setup: TestSetup<2> = TestSetup::new(1);

    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Pending);
    assert_eq!(setup.poll_fork_waker_now(0, 1), Poll::Pending);
    block_on(async {
        let _ = setup.sender.send(0).await;
    });
    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Ready(Some(0)));
    assert_eq!(setup.poll_fork_waker_now(0, 1), Poll::Pending);
}

#[test]
fn first_waker_also_consumed() {
    let mut setup: TestSetup<2> = TestSetup::new(1);

    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Pending);
    assert_eq!(setup.poll_fork_waker_now(0, 1), Poll::Pending);
    block_on(async {
        let _ = setup.sender.send(0).await;
    });
    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Ready(Some(0)));
    assert_eq!(setup.poll_fork_waker_now(0, 1), Poll::Pending);
}

#[test]
fn send_multiple_types() {
    let mut setup: TestSetup<2> = TestSetup::new(1);

    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Pending);

    block_on(async {
        let _ = setup.sender.send(0).await;
        let _ = setup.sender.send(1).await;
        let _ = setup.sender.send(2).await;
    });
    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Ready(Some(0)));
    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Ready(Some(1)));
    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Ready(Some(2)));
}

#[test]
fn send_over_different_channels() {
    let mut setup: TestSetup<2> = TestSetup::new(1);

    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Pending);

    block_on(async {
        let _ = setup.sender.send(0).await;
        let _ = setup.sender.send(1).await;
    });
    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Ready(Some(0)));
    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Ready(Some(1)));
}

#[test]
fn send_some_items_over_multiple_channels() {
    let mut setup: TestSetup<2> = TestSetup::new(1);

    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Pending);

    block_on(async {
        let _ = setup.sender.send(0).await;
        let _ = setup.sender.send(1).await;
        let _ = setup.sender.send(2).await;
    });
    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Ready(Some(0)));
    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Ready(Some(1)));
    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Ready(Some(2)));
}

#[test]
fn clone_streams() {
    let mut setup: TestSetup<2> = TestSetup::new(1);

    let cloned_stream = setup.forks[0].as_ref().unwrap().stream.clone();

    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Pending);
    assert_eq!(cloned_stream.poll_next_unpin(&mut setup.forks[0].as_mut().unwrap().wakers[0].context()), Poll::Pending);

    block_on(async {
        let _ = setup.sender.send(0).await;
    });

    assert_eq!(setup.poll_fork_waker_now(0, 0), Poll::Ready(Some(0)));
    assert_eq!(cloned_stream.poll_next_unpin(&mut setup.forks[0].as_mut().unwrap().wakers[0].context()), Poll::Ready(Some(0)));
}
