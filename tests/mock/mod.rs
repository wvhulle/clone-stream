#![allow(unused_imports)]
#![allow(dead_code)]

mod spsc;
mod test_setups;

mod test_log;
use core::panic;
use std::{
    fmt::Debug,
    task::{Context, Poll},
    time::Duration,
};

use forked_stream::ForkStream;
use futures::{FutureExt, Stream, StreamExt, task::noop_waker};
use log::{info, trace};
pub use spsc::{Sender as SpscSender, channel as spsc_channel};
pub use test_log::log_init;
pub use test_setups::{ConcurrentSetup, instants_between, new_sender_and_shared_stream};
use tokio::{
    select,
    task::JoinHandle,
    time::{Instant, sleep_until, timeout},
};

/// A trait for streams that can be poll concurrently in the background.
pub trait TestableStream:
    Stream<Item: PartialEq + Debug + Send> + Clone + Unpin + Send + 'static
{
    async fn expect(&mut self, value: Option<Self::Item>, deadline: Duration) -> bool {
        let item = timeout(deadline, self.next())
            .await
            .expect("Timed out in background.");
        item == value
    }

    fn assert(
        &mut self,
        expected: Poll<Option<Self::Item>>,
        deadline: Instant,
    ) -> impl Future<Output = ()> + Send {
        async move {
            match expected {
                Poll::Pending => {
                    select! {
                        () = sleep_until(deadline) => {

                        }
                        item = self.next() => {
                            panic!("Fork should not have received an item, but it did: {:?}",  item);
                        }

                    }
                }
                Poll::Ready(expected) => {
                    select! {
                        () = sleep_until(deadline) => {

                            panic!("Fork should have received an item, but it didn't.");
                        }
                        actual = self.next() => {
                            assert_eq!(actual, expected, "The item that was received by fork  did not match the expectation.");
                        }

                    }
                }
            }
        }
    }

    fn assert_background(
        &self,
        expected: Poll<Option<Self::Item>>,
        start: Instant,
        deadline: Instant,
    ) -> JoinHandle<()> {
        let mut background_stream = self.clone();

        tokio::spawn(async move {
            sleep_until(start).await;
            background_stream.assert(expected, deadline).await;
        })
    }
}

impl<ForkedStream> TestableStream for ForkedStream where
    ForkedStream: Stream<Item: PartialEq + Debug + Send> + Clone + Unpin + Send + 'static
{
}
