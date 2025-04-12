use std::future::ready;

use forked_stream::ForkStream;
use futures::{FutureExt, Stream, StreamExt, executor::block_on, stream::FusedStream};

#[test]
fn test_one_fork_terminated() {
    let stream = ready(1).into_stream();

    let mut fork = stream.fork();

    assert!(!fork.is_terminated());

    block_on(async {
        let _ = fork.next().await;
        let _ = fork.next().await;
    });

    assert!(fork.is_terminated());
}

#[test]
fn test_two_forks_terminated() {
    let stream = ready(1).into_stream();

    let mut fork = stream.fork();

    let mut clone = fork.clone();

    assert!(!clone.is_terminated());
    assert!(!fork.is_terminated());

    fork.next().now_or_never();
    clone.next().now_or_never();

    block_on(async {
        let _ = fork.next().await;
        assert!(fork.is_terminated());
        let _ = clone.next().await;
        assert!(clone.is_terminated());
    });
    assert!(clone.is_terminated());
    assert!(fork.is_terminated());
}

#[test]
fn test_one_fork_size() {
    let stream = ready(1).into_stream();

    let mut fork = stream.fork();

    assert_eq!(fork.size_hint(), (1, Some(1)));

    block_on(async {
        let _ = fork.next().await;
    });

    assert_eq!(fork.size_hint(), (0, Some(0)));
}

#[test]
fn test_two_fork_size() {
    let stream = ready(1).into_stream();

    let mut fork = stream.fork();

    let mut clone = fork.clone();

    assert_eq!(fork.size_hint(), (1, Some(1)));
    assert_eq!(clone.size_hint(), (1, Some(1)));

    block_on(async {
        let _ = fork.next().await;
        let _ = clone.next().await;
    });

    assert_eq!(fork.size_hint(), (0, Some(0)));

    assert_eq!(clone.size_hint(), (0, Some(0)));
}
