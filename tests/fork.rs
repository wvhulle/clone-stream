mod mock;

use futures::{FutureExt, StreamExt};
use mock::{log_init, send_fork};

#[test]
fn construction_does_not_panic() {
    log_init();
    send_fork::<usize>(None);
}

#[test]
fn construct_fork_no_panic() {
    log_init();
    let (_, mut fork) = send_fork::<usize>(None);

    assert!(
        fork.next().now_or_never().is_none(),
        "We should have started listening too early."
    );
}

#[test]
fn nothing_sent_nothing_received() {
    log_init();
    let (_, mut fork) = send_fork::<usize>(None);

    assert!(
        fork.next().now_or_never().is_none(),
        "Listening on the first output stream should have started too late."
    );
}
