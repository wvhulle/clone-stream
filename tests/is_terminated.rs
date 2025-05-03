use std::future::ready;

use clone_stream::ForkStream;
use futures::{FutureExt, StreamExt, executor::block_on, stream::FusedStream};

#[test]
fn ready_into_stream() {
    let stream = ready(1).into_stream();

    let mut clone = stream.fork();

    assert!(!clone.is_terminated());

    block_on(async {
        let _ = clone.next().await;
        let _ = clone.next().await;
    });

    assert!(clone.is_terminated());
}

#[test]
fn two_clones_terminated() {
    let stream = ready(1).into_stream();

    let clone = stream.fork();

    let mut clone = clone.clone();

    assert!(!clone.is_terminated());
    assert!(!clone.is_terminated());

    block_on(async {
        let _ = clone.next().await;
        assert!(clone.is_terminated());
        let _ = clone.next().await;
        assert!(clone.is_terminated());
    });
    assert!(clone.is_terminated());
    assert!(clone.is_terminated());
}
