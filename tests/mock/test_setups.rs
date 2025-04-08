use std::{task::Poll, time::Duration};

use forked_stream::{ForkStream, ForkedStream};
use futures::{
    FutureExt, Stream, StreamExt,
    channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded},
};
use log::info;
use tokio::{
    task::JoinHandle,
    time::{Instant, timeout},
};

use super::{MockWaker, TimeRange, test_log::log_init};

pub type SimpleForkedStream<Item> = ForkedStream<UnboundedReceiver<Item>>;

const TIME_PER_FORK_TO_RESOLVE: Duration = Duration::from_micros(500);

pub fn send_fork<Item>() -> (UnboundedSender<Item>, SimpleForkedStream<Item>)
where
    Item: Clone,
{
    let (test_input_sender, test_input_receiver) = unbounded();

    (test_input_sender, test_input_receiver.fork())
}

pub struct WakerStream<Item>
where
    Item: Clone,
{
    waker_a: MockWaker,
    waker_b: MockWaker,
    stream: SimpleForkedStream<Item>,
}


impl<Item> WakerStream<Item>
where
    Item: Clone,
{
    pub fn next_a(&mut self) -> Poll<Option<Item>> {
        self.stream.poll_next_unpin(&mut self.waker_a.context())
    }
}
impl<Item> WakerStream<Item>
where
    Item: Clone,
{
    pub fn next_b(&mut self) -> Poll<Option<Item>> {
        self.stream.poll_next_unpin(&mut self.waker_b.context())
    }
}


pub struct ForkAsyncMockSetup<Item, const N: usize>
where
    Item: Clone,
{
    pub sender: UnboundedSender<Item>,
    pub forks: [WakerStream<Item>; N],
    pub time_range: TimeRange,
}

impl<Item, const N: usize> ForkAsyncMockSetup<Item, N>
where
    Item: Clone,
{
    pub fn new() -> Self {
        log_init();
        let (input, output_stream) = send_fork::<Item>();

        ForkAsyncMockSetup {
            sender: input,
            forks: [0; N].map(|i| WakerStream {
                waker_a: MockWaker::new(2*i),
                waker_b: MockWaker::new(2*i + 1),
                stream: output_stream.clone(),
            }),
            time_range: TimeRange::from(TIME_PER_FORK_TO_RESOLVE * N.try_into().unwrap()),
        }
    }
}
