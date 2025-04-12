mod spsc;

use forked_stream::{ForkStream, enable_debug_log};
use futures::{FutureExt, SinkExt, StreamExt};
use spsc::{Setup, channel};

#[test]
fn i() {
    let Setup {
        mut sender,
        mut fork_0,
        mut fork_1,
    } = Setup::new();

    assert!(fork_0.next().now_or_never().is_none());

    assert!(sender.send('a').now_or_never().is_some());

    assert_eq!(fork_1.next().now_or_never(), Some(Some('a')));
}

#[test]
fn j() {
    let Setup {
        mut sender,
        mut fork_0,
        mut fork_1,
    } = Setup::new();

    assert!(fork_0.next().now_or_never().is_none());

    assert!(sender.send('a').now_or_never().is_some());

    assert_eq!(fork_0.next().now_or_never(), Some(Some('a')));

    assert_eq!(fork_1.next().now_or_never(), None);
}

#[test]
fn g() {
    let Setup {
        mut sender,
        mut fork_0,
        mut fork_1,
    } = Setup::new();

    assert!(fork_0.next().now_or_never().is_none());

    assert!(fork_1.next().now_or_never().is_none());

    assert!(sender.send('a').now_or_never().is_some());
}

#[test]
fn c() {
    let Setup {
        mut fork_0,
        mut fork_1,
        ..
    }: Setup = Setup::new();

    assert!(fork_0.next().now_or_never().is_none());

    assert!(fork_1.next().now_or_never().is_none());
}

#[test]
fn active() {
    enable_debug_log();
    let (mut tx, rx) = channel::<char>();

    let mut fork_0 = rx.fork();
    let mut fork_1 = fork_0.clone();

    log::info!("Just polling fork {}", fork_1.id);
    assert_eq!(fork_1.next().now_or_never(), None);

    assert!(fork_1.active());
    assert!(!fork_0.active());
    log::info!("Sending a");
    assert_eq!(tx.send('a').now_or_never(), Some(Ok(())));

    assert!(fork_1.active());
    assert!(!fork_0.active());

    assert_eq!(fork_1.next().now_or_never(), Some(Some('a')));

    assert!(!fork_0.active());

    // assert_eq!(fork_0.next().now_or_never(), None);
    log::info!("Sending b");
    assert_eq!(tx.send('b').now_or_never(), None);

    assert!(!fork_0.active());
    assert!(fork_1.active());

    assert_eq!(fork_0.next().now_or_never(), None);
    assert_eq!(fork_1.next().now_or_never(), None);

    log::info!("Sending c");
    assert_eq!(tx.send('c').now_or_never(), Some(Ok(())));

    assert!(fork_0.active());
    assert!(fork_1.active());
}

#[test]
fn queued_items() {
    enable_debug_log();
    let (mut tx, rx) = channel::<char>();

    let mut fork_0 = rx.fork();
    let mut fork_1 = fork_0.clone();

    log::info!("Just polling fork {}", fork_1.id);
    assert_eq!(fork_1.next().now_or_never(), None);

    log::info!("Sending a");
    assert_eq!(tx.send('a').now_or_never(), Some(Ok(())));

    assert_eq!(fork_1.next().now_or_never(), Some(Some('a')));

    log::info!("Sending b");
    assert_eq!(tx.send('b').now_or_never(), None);

    assert_eq!(fork_0.queued_items(), 0);
    assert_eq!(fork_1.queued_items(), 0);

    assert_eq!(fork_0.next().now_or_never(), None);
    assert_eq!(fork_1.next().now_or_never(), None);

    log::info!("Sending c");
    assert_eq!(tx.send('c').now_or_never(), Some(Ok(())));

    assert_eq!(fork_0.queued_items(), 0);
    assert_eq!(fork_1.queued_items(), 0);

    assert_eq!(fork_0.next().now_or_never(), Some(Some('c')));

    assert_eq!(fork_1.queued_items(), 1);
    assert_eq!(fork_0.queued_items(), 0);

    assert_eq!(fork_1.next().now_or_never(), Some(Some('c')));

    assert_eq!(fork_1.queued_items(), 0);
    assert_eq!(fork_0.queued_items(), 0);
}
