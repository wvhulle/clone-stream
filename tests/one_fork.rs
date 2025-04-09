use std::task::Poll;

use forked_stream::TestSetup;
use futures::{SinkExt, executor::block_on};

#[test]
fn nothing() {
    let mut setup: TestSetup = TestSetup::new(1);

    assert_eq!(setup.poll_now(), Poll::Pending);
}

#[test]
fn send_one() {
    let mut setup: TestSetup = TestSetup::new(1);

    assert_eq!(setup.poll_now(), Poll::Pending);
    block_on(async {
        let _ = setup.sender.start_send(0);
        let _ = setup.sender.flush().await;
    });
    assert_eq!(setup.poll_now(), Poll::Ready(Some(0)));
}

#[test]
fn send_one_back_to_pending() {
    let mut setup: TestSetup = TestSetup::new(1);

    assert_eq!(setup.poll_now(), Poll::Pending);
    block_on(async {
        let _ = setup.sender.start_send(0);
        let _ = setup.sender.flush().await;
    });
    assert_eq!(setup.poll_now(), Poll::Ready(Some(0)));

    assert_eq!(setup.poll_now(), Poll::Pending);
}

#[test]
fn send_two() {
    let mut setup: TestSetup = TestSetup::new(1);

    assert_eq!(setup.poll_now(), Poll::Pending);
    block_on(async {
        let _ = setup.sender.start_send(0);
        let _ = setup.sender.start_send(1);
        let _ = setup.sender.flush().await;
    });
    assert_eq!(setup.poll_now(), Poll::Ready(Some(0)));
    assert_eq!(setup.poll_now(), Poll::Ready(Some(1)));
}

#[test]
fn send_two_back_to_pending() {
    let mut setup: TestSetup = TestSetup::new(1);

    assert_eq!(setup.poll_now(), Poll::Pending);
    block_on(async {
        let _ = setup.sender.start_send(0);
        let _ = setup.sender.start_send(1);
        let _ = setup.sender.flush().await;
    });
    assert_eq!(setup.poll_now(), Poll::Ready(Some(0)));
    assert_eq!(setup.poll_now(), Poll::Ready(Some(1)));
    assert_eq!(setup.poll_now(), Poll::Pending);
}
