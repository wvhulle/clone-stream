use std::future::ready;

use clone_stream::ForkStream;
use futures::{FutureExt, Stream, StreamExt, executor::block_on};

#[test]
fn one_clone_size() {
    let stream = ready(1).into_stream();

    let mut clone = stream.fork();

    assert_eq!(clone.size_hint(), (1, Some(1)));

    block_on(async {
        let _ = clone.next().await;
    });

    assert_eq!(clone.size_hint(), (0, Some(0)));
}

#[test]
fn two_clone_size() {
    let stream = ready(1).into_stream();

    let clone = stream.fork();

    let mut clone = clone.clone();

    assert_eq!(clone.size_hint(), (1, Some(1)));
    assert_eq!(clone.size_hint(), (1, Some(1)));

    block_on(async {
        let _ = clone.next().await;
        let _ = clone.next().await;
    });

    assert_eq!(clone.size_hint(), (0, Some(0)));

    assert_eq!(clone.size_hint(), (0, Some(0)));
}
