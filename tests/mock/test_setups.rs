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

const TIME_PER_FORK_TO_RESOLVE: Duration = Duration::from_micros(500);

pub fn send_fork<Item>(cache: Option<usize>) -> (SpscSender<Item>, SimpleForkedStream<Item>)
where
    Item: Clone,
{
    let (test_input_sender, test_input_receiver) = spsc_channel();

    (test_input_sender, test_input_receiver.fork(cache))
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
    pub fn new(cache: impl Into<Option<usize>>, estimated_forks: usize) -> Self {
        log_init();
        let (input, output_stream) = send_fork::<Item>(cache.into());

        ConcurrentSetup {
            input_sink: input,
            forked_stream: output_stream,
            time_range: TimeRange::from(
                TIME_PER_FORK_TO_RESOLVE * estimated_forks.try_into().unwrap(),
            ),
        }
    }
}
