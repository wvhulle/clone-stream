use std::task::Poll;

use futures::{StreamExt, channel::mpsc::UnboundedReceiver};
use tokio::{
    select,
    task::JoinHandle,
    time::{Instant, sleep_until},
};

use super::{MockWaker, TimeRange};
use crate::CloneStream;

type Receiver = UnboundedReceiver<usize>;

pub struct ForkWithMockWakers<const N_WAKERS: usize> {
    pub wakers: [MockWaker; N_WAKERS],
    pub stream: CloneStream<Receiver>,
}

pub enum StreamNextPollError {
    NotReady,
    NotPending,
    Unexpected { expected: usize, actual: usize },
}

impl<const N_WAKERS: usize> ForkWithMockWakers<N_WAKERS> {
    pub fn poll_now(&mut self) -> Poll<Option<usize>> {
        self.stream
            .poll_next_unpin(&mut self.wakers.first().unwrap().context())
    }

    pub fn poll_waker_now(&mut self, n: usize) -> Poll<Option<usize>> {
        self.stream.poll_next_unpin(&mut self.wakers[n].context())
    }

    pub async fn poll_abort_assert(
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
                            Err(StreamNextPollError::Unexpected {
                                expected: expected.unwrap(),
                                actual: actual.unwrap(),
                            })
                        }
                    }

                }
            }
        }
    }

    #[must_use]
    pub fn move_to_background_poll_abort(
        mut self,
        expected: Poll<Option<usize>>,

        poll_abort: TimeRange,
    ) -> JoinHandle<Result<(), StreamNextPollError>> {
        tokio::spawn(async move {
            sleep_until(poll_abort.start).await;
            self.poll_abort_assert(expected, poll_abort.end).await
        })
    }
}
