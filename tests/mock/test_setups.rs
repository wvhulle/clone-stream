use std::{task::Poll, time::Duration};

use forked_stream::{ForkStream, ForkedStream};
use futures::{FutureExt, Stream, StreamExt};
use log::info;
use tokio::{
    task::JoinHandle,
    time::{Instant, timeout},
};

use super::{SpscSender, spsc_channel, test_log::log_init};

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
        }
    }
}

/// Function that generates random `Instants` in between start Instant and end Instant
pub fn instants_between(start: Instant, end: Instant, n: u32) -> Vec<Instant> {
    let duration = end.duration_since(start);
    (0..n)
        .map(|i| start + duration.mul_f64(f64::from(i) / f64::from(n)))
        .collect()
}
