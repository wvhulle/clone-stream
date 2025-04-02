use forked_stream::{ForkStream, ForkedStream};
use futures::Stream;

use super::{SpscSender, spsc_channel};

pub fn new_sender_and_shared_stream<T>() -> (SpscSender<T>, ForkedStream<impl Stream<Item = T>>)
where
    T: Clone,
{
    let (test_input_sender, test_input_receiver) = spsc_channel();

    (test_input_sender, test_input_receiver.fork())
}
