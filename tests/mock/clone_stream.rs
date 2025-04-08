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
    future::{join_all, try_join_all},
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

pub enum AssertError {
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
    ) -> Result<(), AssertError> {
        match expected_poll_at_deadline {
            Poll::Pending => {
                select! {
                    () = sleep_until(deadline) => {
                        Ok(())
                    }
                    _ = self.stream.next() => {
                        Err(AssertError::NotPending)
                    }

                }
            }
            Poll::Ready(expected) => {
                select! {
                    () = sleep_until(deadline) => {
                       Err(AssertError::NotReady)
                    }
                    actual = self.stream.next() => {
                        if actual == expected {
                            Ok(())
                        } else {
                            Err(AssertError::WrongRestult {
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
    ) -> JoinHandle<Result<(), AssertError>> {
        tokio::spawn(async move {
            assert!(
                start_await_cancel_await.start > Instant::now(),
                "The background for fork {} should start later on.",
                self.stream.id
            );

            trace!("Waiting until stream should be polled in the background.");
            sleep_until(start_await_cancel_await.start).await;
            self.assert_poll_now(expected_poll_at_end, start_await_cancel_await.end)
                .await
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

    pub async fn launch(
        &mut self,
        sub_range: impl Fn(usize) -> TimeRange,
        do_inbetween: impl Future,
    ) -> Metrics {
        let wait_for_all = join_all(self.forks.iter_mut().enumerate().map(|(index, fork)| {
            fork.take()
                .unwrap()
                .assert_background(Poll::Ready(Some(0)), sub_range(index))
        }));

        do_inbetween.await;

        let results: Vec<_> = wait_for_all.await.into_iter().map(Result::unwrap).collect();

        Metrics {
            not_pending: results
                .iter()
                .filter(|result| matches!(result, Err(AssertError::NotPending)))
                .count(),

            not_ready: results
                .iter()
                .filter(|result| matches!(result, Err(AssertError::NotReady)))
                .count(),
            wrong_result: results
                .iter()
                .filter(|result| matches!(result, Err(AssertError::WrongRestult { .. })))
                .count(),
        }
    }
}

#[derive(Debug)]
pub struct Metrics {
    pub not_ready: usize,
    pub not_pending: usize,
    pub wrong_result: usize,
}

impl Metrics {
    pub fn total(&self) -> usize {
        self.not_ready + self.not_pending + self.wrong_result
    }

    pub fn is_successful(&self) -> bool {
        self.not_ready == 0 && self.not_pending == 0 && self.wrong_result == 0
    }
}
