mod mock;

use futures::{FutureExt, StreamExt};
use mock::{log_init, new_sender_and_shared_stream};

#[test]
fn construction_does_not_panic() {
    log_init();
    new_sender_and_shared_stream::<usize>();
}

#[test]
fn no_send() {
    log_init();
    let (_, mut output_stream) = new_sender_and_shared_stream::<usize>();

    assert!(
        output_stream.next().now_or_never().is_none(),
        "We should have started listening too early."
    );
}

#[test]
fn await_too_late_double() {
    log_init();
    let (_, mut first_output_stream) = new_sender_and_shared_stream::<usize>();

    let mut second_output_stream = first_output_stream.clone();

    assert!(
        first_output_stream.next().now_or_never().is_none(),
        "Listening on the first output stream should have started too late."
    );

    assert!(
        second_output_stream.next().now_or_never().is_none(),
        "Listening on the second output stream should have started too late."
    )
}
