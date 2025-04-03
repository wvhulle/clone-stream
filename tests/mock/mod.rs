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

use forked_stream::{ForkStream, OutputStreamId};
use futures::{FutureExt, Stream, StreamExt, task::noop_waker};
use log::{info, trace};
pub use spsc::{Sender as SpscSender, channel as spsc_channel};
pub use test_log::log_init;
pub use test_setups::{ConcurrentSetup, instants, new_sender_and_shared_stream};
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

    async fn expect_at(&mut self, value: Option<Self::Item>, deadline: Instant) -> bool {
        sleep_until(deadline).await;
        match self.ready() {
            Poll::Ready(item) => value == item,
            Poll::Pending => false,
        }
    }

    fn expect_background(&self, value: Option<Self::Item>, deadline: Duration) -> JoinHandle<()> {
        let mut background_stream = self.clone();
        info!("Spawning a background task to await next of cloned stream.",);
        tokio::spawn(async move {
            info!("Waiting for the next method on the background stream to resolve.",);
            let item = timeout(deadline, background_stream.next())
                .await
                .expect("Timed out in background.");
            assert_eq!(item, value, "Background task received an unexpected value.");
        })
    }

    fn expect_background_at(
        &self,
        expected: Poll<Option<Self::Item>>,
        deadline: Instant,
    ) -> JoinHandle<()> {
        let mut background_stream = self.clone();

        tokio::spawn(async move {
            match expected {
                Poll::Pending => {
                    select! {
                        () = sleep_until(deadline) => {
                            trace!("Background task for fork timed out before a next element was served as expected.");


                        }
                        item = background_stream.next() => {
                            trace!("Got the next element on the background task.");
                            panic!("Fork should not have received an item, but it did: {:?}",  item);
                        }

                    }
                }
                Poll::Ready(expected) => {
                    select! {
                        () = sleep_until(deadline) => {

                            panic!("Fork should have received an item, but it didn't.");
                        }
                        actual = background_stream.next() => {
                            trace!("Got the next element on the background task.");
                            assert_eq!(actual, expected, "The item that was received by fork  did not match the expectation.");
                        }

                    }
                }
            }
        })
    }

    /// Find out whether the streams is ready right now to return an `Option<Item>`.
    fn ready(&mut self) -> Poll<Option<Self::Item>> {
        match self.next().now_or_never() {
            None => Poll::Pending,
            Some(item) => Poll::Ready(item),
        }
    }
}

impl<ForkedStream> TestableStream for ForkedStream where
    ForkedStream: Stream<Item: PartialEq + Debug + Send> + Clone + Unpin + Send + 'static
{
}
