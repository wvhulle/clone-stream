use std::{task::Poll, time::Duration};

use forked_stream::{ForkStream, ForkedStream};
use futures::{FutureExt, Stream, StreamExt};
use log::info;
use tokio::{
    task::JoinHandle,
    time::{Instant, timeout},
};

use super::{SpscSender, TimeRange, spsc_channel, test_log::log_init};

pub type SimpleForkedStream<Item> = ForkedStream<super::spsc::Receiver<Item>>;

pub fn new_sender_and_shared_stream<Item>() -> (SpscSender<Item>, SimpleForkedStream<Item>)
where
    Item: Clone,
{
    let (test_input_sender, test_input_receiver) = spsc_channel();

    (test_input_sender, test_input_receiver.fork())
}

pub struct ConcurrentSetup<Item>
where
    Item: Clone,
{
    pub input_sink: SpscSender<Item>,
    pub forked_stream: SimpleForkedStream<Item>,
    pub time_range: TimeRange,
}

impl<Item> ConcurrentSetup<Item>
where
    Item: Clone,
{
    pub fn new() -> Self {
        log_init();
        let (input, output_stream) = new_sender_and_shared_stream::<Item>();

        ConcurrentSetup {
            input_sink: input,
            forked_stream: output_stream,
            time_range: TimeRange::until(Duration::from_millis(10)),
        }
    }
}
