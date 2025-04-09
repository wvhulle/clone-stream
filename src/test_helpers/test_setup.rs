use std::task::Poll;

use futures::{
    channel::mpsc::{UnboundedSender, unbounded},
    future::join_all,
};

use super::{ForkWithMockWakers, MockWaker, StreamNextPollError, TimeRange};
use crate::ForkStream;

const DEFAULT_N_WAKERS: usize = 1;

/// A container for a bunch of simple clone-able streams for testing.
pub struct TestSetup<const N_WAKERS: usize = DEFAULT_N_WAKERS> {
    pub sender: UnboundedSender<usize>,
    pub forks: Vec<Option<ForkWithMockWakers<N_WAKERS>>>,
}

impl<const N_WAKERS: usize> TestSetup<N_WAKERS>
where
    usize: Clone,
{
    #[must_use]
    pub fn new(n_forks: usize) -> Self {
        let (input, output) = unbounded();

        let fork = output.fork();

        TestSetup {
            sender: input,
            forks: (0..n_forks)
                .map(|_| {
                    Some(ForkWithMockWakers {
                        wakers: [0; N_WAKERS].map(|_| MockWaker::new()),
                        stream: fork.clone(),
                    })
                })
                .collect(),
        }
    }

    pub fn poll_fork_waker_now(&mut self, stream_id: usize, waker: usize) -> Poll<Option<usize>> {
        self.forks[stream_id]
            .as_mut()
            .unwrap()
            .poll_waker_now(waker)
    }

    pub fn poll_fork_now(&mut self, stream_id: usize) -> Poll<Option<usize>> {
        self.forks[stream_id].as_mut().unwrap().poll_now()
    }

    pub fn poll_now(&mut self) -> Poll<Option<usize>> {
        self.forks[0].as_mut().unwrap().poll_now()
    }

    pub async fn poll_forks_background(
        &mut self,
        poll_abort: impl Fn(usize) -> TimeRange,
        sender_action: impl Future,
    ) -> ForkAbortState {
        let wait_for_all = join_all(self.forks.iter_mut().enumerate().map(|(index, fork)| {
            fork.take()
                .unwrap()
                .move_to_background_poll_abort(Poll::Ready(Some(0)), poll_abort(index))
        }));

        sender_action.await;

        let fork_task_results: Vec<_> =
            wait_for_all.await.into_iter().map(Result::unwrap).collect();

        ForkAbortState {
            not_pending: fork_task_results
                .iter()
                .filter(|result| matches!(result, Err(StreamNextPollError::NotPending)))
                .count(),

            not_ready: fork_task_results
                .iter()
                .filter(|result| matches!(result, Err(StreamNextPollError::NotReady)))
                .count(),
            wrong_result: fork_task_results
                .iter()
                .filter(|result| matches!(result, Err(StreamNextPollError::Unexpected { .. })))
                .count(),
        }
    }
}

#[derive(Debug)]
pub struct ForkAbortState {
    pub not_ready: usize,
    pub not_pending: usize,
    pub wrong_result: usize,
}

impl ForkAbortState {
    pub fn total(&self) -> usize {
        self.not_ready + self.not_pending + self.wrong_result
    }

    pub fn success(&self) -> bool {
        self.total() == 0
    }
}
