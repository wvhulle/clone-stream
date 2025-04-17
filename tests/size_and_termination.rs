use std::future::ready;

use clone_stream::ForkStream;
use futures::{FutureExt, Stream, StreamExt, executor::block_on, stream::FusedStream};

#[test]
fn test_one_clone_terminated() {
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
fn test_two_clones_terminated() {
    let stream = ready(1).into_stream();

    let clone = stream.fork();

    let mut clone = clone.clone();

    assert!(!clone.is_terminated());
    assert!(!clone.is_terminated());

    clone.next().now_or_never();
    clone.next().now_or_never();

    block_on(async {
        let _ = clone.next().await;
        assert!(clone.is_terminated());
        let _ = clone.next().await;
        assert!(clone.is_terminated());
    });
    assert!(clone.is_terminated());
    assert!(clone.is_terminated());
}

#[test]
fn test_one_clone_size() {
    let stream = ready(1).into_stream();

    let mut clone = stream.fork();

    assert_eq!(clone.size_hint(), (1, Some(1)));

    block_on(async {
        let _ = clone.next().await;
    });

    assert_eq!(clone.size_hint(), (0, Some(0)));
}

#[test]
fn test_two_clone_size() {
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
