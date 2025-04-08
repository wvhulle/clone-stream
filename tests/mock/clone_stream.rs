use std::{
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
    task::Poll,
    time::Duration,
};

use chrono::format::Item;
use forked_stream::{CloneStream, Fork, ForkStream};
use futures::{
    FutureExt, Stream, StreamExt,
    channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded},
};
use log::{info, trace};
use tokio::{
    select,
    task::JoinHandle,
    time::{Instant, sleep_until, timeout},
};

use super::{MockWaker, TimeRange, set_log_level::log_init};

type Receiver = UnboundedReceiver<usize>;

static TEST_FORK_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct StreamWithWakers<const N_WAKERS: usize> {
    pub wakers: [MockWaker; N_WAKERS],
    pub stream: CloneStream<Receiver>,
}

impl<const N_WAKERS: usize> StreamWithWakers<N_WAKERS> {
    /// # Panics
    ///
    /// Panics if there if `N_WAKERS` is 0.
    pub fn poll_next(&mut self) -> Poll<Option<usize>> {
        self.stream
            .poll_next_unpin(&mut self.wakers.first().unwrap().context())
    }

    pub fn poll_next_with_waker(&mut self, n: usize) -> Poll<Option<usize>> {
        self.stream.poll_next_unpin(&mut self.wakers[n].context())
    }

    /// # Panics
    ///
    /// Panics if the task life time is not in the future.
    pub async fn assert_poll_now(
        &mut self,
        expected_poll_at_deadline: Poll<Option<usize>>,
        deadline: Instant,
    ) {
        match expected_poll_at_deadline {
            Poll::Pending => {
                println!(
                    "We expect the item for fork {} to never be delivered.",
                    self.stream.id
                );
                select! {
                    () = sleep_until(deadline) => {

                    }
                    item = self.stream.next() => {
                        panic!("Fork {} should not have received an item, but it did: {item:?}", self.stream.id);
                    }

                }
            }
            Poll::Ready(expected) => {
                println!(
                    "We expect the item for fork {} to be delivered.",
                    self.stream.id
                );
                select! {
                    () = sleep_until(deadline) => {
                        panic!("Fork {} should have received an item, but it didn't.", self.stream.id);
                    }
                    actual = self.stream.next() => {
                        assert_eq!(actual, expected, "The item that was received by fork {} did not match the expectation.", self.stream.id);
                    }

                }
            }
        }
    }

    /// # Panics
    ///
    /// Panics if the task life time is not in the future.
    #[must_use]
    pub fn assert_background(
        mut self,
        expected_poll_at_end: Poll<Option<usize>>,

        start_await_cancel_await: TimeRange,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            assert!(
                start_await_cancel_await.start > Instant::now(),
                "The background for fork {} should start later on.",
                self.stream.id
            );

            trace!("Waiting until stream should be polled in the background.");
            sleep_until(start_await_cancel_await.start).await;
            self.assert_poll_now(expected_poll_at_end, start_await_cancel_await.end)
                .await;
        })
    }
}

pub struct ForkAsyncMockSetup<const N_FORKS: usize, const N_WAKERS: usize> {
    pub sender: UnboundedSender<usize>,
    pub forks: [Option<StreamWithWakers<N_WAKERS>>; N_FORKS],
}

impl<const N_FORKS: usize, const N_WAKERS: usize> ForkAsyncMockSetup<N_FORKS, N_WAKERS>
where
    usize: Clone,
{
    pub fn new() -> Self {
        log_init();
        let (input, output) = unbounded();

        let fork = output.fork();

        ForkAsyncMockSetup {
            sender: input,
            forks: [0; N_FORKS].map(|_| {
                Some(StreamWithWakers {
                    wakers: [0; N_WAKERS].map(|_| MockWaker::new()),
                    stream: fork.clone(),
                })
            }),
        }
    }

    pub fn poll_stream_with_waker(
        &mut self,
        stream_id: usize,
        waker: usize,
    ) -> Poll<Option<usize>> {
        self.forks[stream_id]
            .as_mut()
            .unwrap()
            .poll_next_with_waker(waker)
    }

    pub fn poll_stream(&mut self, stream_id: usize) -> Poll<Option<usize>> {
        self.forks[stream_id].as_mut().unwrap().poll_next()
    }

    pub fn poll_one(&mut self) -> Poll<Option<usize>> {
        self.forks[0].as_mut().unwrap().poll_next()
    }
}
