use std::task::Poll;

use forked_stream::TestSetup;
use futures::{SinkExt, executor::block_on};
use log::info;

#[test]
fn nothing() {
    let mut setup: TestSetup = TestSetup::new(2);

    assert_eq!(setup.poll_fork_now(0), Poll::Pending);
    assert_eq!(setup.poll_fork_now(1), Poll::Pending);
}

#[test]
fn one_pending_send_one() {
    let mut setup: TestSetup = TestSetup::new(2);

    assert_eq!(setup.poll_fork_now(0), Poll::Pending);

    block_on(async {
        let _ = setup.sender.send(0).await;
    });
    assert_eq!(setup.poll_fork_now(0), Poll::Ready(Some(0)));

    assert_eq!(setup.poll_fork_now(1), Poll::Pending);
}

#[test]
fn both_pending_send_one() {
    let mut setup: TestSetup = TestSetup::new(2);

    assert_eq!(setup.poll_fork_now(0), Poll::Pending);
    assert_eq!(setup.poll_fork_now(1), Poll::Pending);
    block_on(async {
        let _ = setup.sender.send(0).await;
    });
    assert_eq!(setup.poll_fork_now(0), Poll::Ready(Some(0)));

    assert_eq!(setup.poll_fork_now(1), Poll::Ready(Some(0)));
}

#[test]
fn both_pending_send_two_receive_one() {
    let mut setup: TestSetup = TestSetup::new(2);

    assert_eq!(setup.poll_fork_now(0), Poll::Pending);
    assert_eq!(setup.poll_fork_now(1), Poll::Pending);
    block_on(async {
        let _ = setup.sender.send(0).await;
    });
    assert_eq!(setup.poll_fork_now(0), Poll::Ready(Some(0)));
    block_on(async {
        let _ = setup.sender.send(0).await;
    });
    assert_eq!(setup.poll_fork_now(0), Poll::Ready(Some(0)));
}
#[test]
fn both_pending_send_two_receive_one_late() {
    let mut setup: TestSetup = TestSetup::new(2);

    assert_eq!(setup.poll_fork_now(0), Poll::Pending);
    assert_eq!(setup.poll_fork_now(1), Poll::Pending);
    block_on(async {
        let _ = setup.sender.send(0).await;
    });
    assert_eq!(setup.poll_fork_now(0), Poll::Ready(Some(0)));
    block_on(async {
        let _ = setup.sender.send(1).await;
    });
    assert_eq!(setup.poll_fork_now(1), Poll::Ready(Some(0)));
}

#[test]
fn both_pending_send_two_receive_two_twice() {
    let mut setup: TestSetup = TestSetup::new(2);

    assert_eq!(setup.poll_fork_now(0), Poll::Pending);
    assert_eq!(setup.poll_fork_now(1), Poll::Pending);
    block_on(async {
        let _ = setup.sender.start_send(0);
        let _ = setup.sender.start_send(1);
        let _ = setup.sender.flush().await;
    });

    info!("Polling the first stream for the first time.");
    assert_eq!(setup.poll_fork_now(0), Poll::Ready(Some(0)));
    info!("Polling the second stream for the first time.");
    assert_eq!(setup.poll_fork_now(1), Poll::Ready(Some(0)));
    info!("Polling the first stream for the second time.");
    assert_eq!(setup.poll_fork_now(0), Poll::Ready(Some(1)));
    info!("Polling the second stream for the second time.");
    assert_eq!(setup.poll_fork_now(1), Poll::Ready(Some(1)));
}

#[test]
fn send_multiple_types() {
    let mut setup: TestSetup = TestSetup::new(2);

    assert_eq!(setup.poll_fork_now(0), Poll::Pending);

    block_on(async {
        let _ = setup.sender.send(0).await;
        let _ = setup.sender.send(1).await;
        let _ = setup.sender.send(2).await;
    });
    assert_eq!(setup.poll_fork_now(0), Poll::Ready(Some(0)));
    assert_eq!(setup.poll_fork_now(0), Poll::Ready(Some(1)));
    assert_eq!(setup.poll_fork_now(0), Poll::Ready(Some(2)));
}

#[test]
fn send_over_different_channels() {
    let mut setup: TestSetup = TestSetup::new(2);

    assert_eq!(setup.poll_fork_now(0), Poll::Pending);

    block_on(async {
        let _ = setup.sender.send(0).await;
        let _ = setup.sender.send(1).await;
    });
    assert_eq!(setup.poll_fork_now(0), Poll::Ready(Some(0)));
    assert_eq!(setup.poll_fork_now(0), Poll::Ready(Some(1)));
}

#[test]
fn send_some_items_over_multiple_channels() {
    let mut setup: TestSetup = TestSetup::new(2);

    assert_eq!(setup.poll_fork_now(0), Poll::Pending);

    block_on(async {
        let _ = setup.sender.send(0).await;
        let _ = setup.sender.send(1).await;
        let _ = setup.sender.send(2).await;
    });
    assert_eq!(setup.poll_fork_now(0), Poll::Ready(Some(0)));
    assert_eq!(setup.poll_fork_now(0), Poll::Ready(Some(1)));
    assert_eq!(setup.poll_fork_now(0), Poll::Ready(Some(2)));
}

#[test]
fn clone_streams() {
    let mut setup: TestSetup = TestSetup::new(2);

    let cloned_stream = setup.forks[0].as_ref().unwrap().stream.clone();

    assert_eq!(setup.poll_fork_now(0), Poll::Pending);
    assert_eq!(cloned_stream.poll_next_unpin(&mut setup.forks[0].as_mut().unwrap().wakers[0].context()), Poll::Pending);

    block_on(async {
        let _ = setup.sender.send(0).await;
    });

    assert_eq!(setup.poll_fork_now(0), Poll::Ready(Some(0)));
    assert_eq!(cloned_stream.poll_next_unpin(&mut setup.forks[0].as_mut().unwrap().wakers[0].context()), Poll::Ready(Some(0)));
}
