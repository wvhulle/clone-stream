use futures::{FutureExt, SinkExt, StreamExt};
mod spsc_without_buffer;
use spsc_without_buffer::Setup;

#[test]
fn h() {
    let Setup {
        mut sender,
        mut fork_0,
        ..
    } = Setup::new();

    assert!(fork_0.next().now_or_never().is_none());

    assert!(sender.send('a').now_or_never().is_some());

    assert_eq!(fork_0.next().now_or_never(), Some(Some('a')));
}

#[test]
fn k() {
    let Setup {
        mut sender,
        mut fork_0,
        ..
    } = Setup::new();

    assert!(fork_0.next().now_or_never().is_none());

    assert!(sender.send('a').now_or_never().is_some());

    assert!(sender.send('b').now_or_never().is_none());

    assert_eq!(fork_0.next().now_or_never(), Some(Some('a')));

    assert_eq!(fork_0.next().now_or_never(), None);
}

#[test]
fn d() {
    let Setup {
        mut sender,
        mut fork_0,
        ..
    } = Setup::new();

    assert!(fork_0.next().now_or_never().is_none());

    assert!(sender.send('a').now_or_never().is_some());
}

#[test]
fn e() {
    let Setup {
        mut sender,
        mut fork_0,
        ..
    } = Setup::new();

    assert!(sender.send('a').now_or_never().is_none());

    assert!(fork_0.next().now_or_never().is_none());
}

#[test]
fn a() {
    let Setup { mut fork_0, .. }: Setup = Setup::new();

    assert!(fork_0.next().now_or_never().is_none());
}
