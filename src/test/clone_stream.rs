use std::{
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
    task::Poll,
    time::Duration,
};

use chrono::format::Item;
use futures::{
    FutureExt, Stream, StreamExt,
    channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded},
    future::{join_all, try_join_all},
};
use log::{info, trace};
use tokio::{
    select,
    task::JoinHandle,
    time::{Instant, sleep_until, timeout},
};

use super::{MockWaker, TimeRange, set_log_level::log_init};
use crate::{CloneStream, Fork, ForkStream};

type Receiver = UnboundedReceiver<usize>;

static TEST_FORK_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct StreamWithWakers<const N_WAKERS: usize> {
    pub wakers: [MockWaker; N_WAKERS],
    pub stream: CloneStream<Receiver>,
}

pub enum StreamNextPollError {
    NotReady,
    NotPending,
    WrongRestult { expected: usize, actual: usize },
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

    /// # Errors
    ///
    /// Errors when not as expected.
    ///
    /// # Panics
    ///
    ///
    pub async fn assert_poll_now(
        &mut self,
        expected_poll_at_deadline: Poll<Option<usize>>,
        deadline: Instant,
    ) -> Result<(), StreamNextPollError> {
        match expected_poll_at_deadline {
            Poll::Pending => {
                select! {
                    () = sleep_until(deadline) => {
                        Ok(())
                    }
                    _ = self.stream.next() => {
                        Err(StreamNextPollError::NotPending)
                    }

                }
            }
            Poll::Ready(expected) => {
                select! {
                    () = sleep_until(deadline) => {
                       Err(StreamNextPollError::NotReady)
                    }
                    actual = self.stream.next() => {
                        if actual == expected {
                            Ok(())
                        } else {
                            Err(StreamNextPollError::WrongRestult {
                                expected: expected.unwrap(),
                                actual: actual.unwrap(),
                            })
                        }
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
    ) -> JoinHandle<Result<(), StreamNextPollError>> {
        tokio::spawn(async move {
            sleep_until(start_await_cancel_await.start).await;
            self.assert_poll_now(expected_poll_at_end, start_await_cancel_await.end)
                .await
        })
    }
}
