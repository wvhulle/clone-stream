use std::task::Poll;

use futures::{
    channel::mpsc::{UnboundedSender, unbounded},
    future::join_all,
};

use super::{MockWaker, StreamNextPollError, StreamWithWakers, TimeRange, log_init};
use crate::ForkStream;

pub struct ForkAsyncMockSetup<const N_WAKERS: usize = 1> {
    pub sender: UnboundedSender<usize>,
    pub forks: Vec<Option<StreamWithWakers<N_WAKERS>>>,
}

impl<const N_WAKERS: usize> ForkAsyncMockSetup<N_WAKERS>
where
    usize: Clone,
{
    #[must_use]
    pub fn new(n_forks: usize) -> Self {
        log_init();
        let (input, output) = unbounded();

        let fork = output.fork();

        ForkAsyncMockSetup {
            sender: input,
            forks: (0..n_forks)
                .map(|_| {
                    Some(StreamWithWakers {
                        wakers: [0; N_WAKERS].map(|_| MockWaker::new()),
                        stream: fork.clone(),
                    })
                })
                .collect(),
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

        println!("Doing inbetween");
        do_inbetween.await;
        println!("Done doing inbetween");

        let results: Vec<_> = wait_for_all.await.into_iter().map(Result::unwrap).collect();

        Metrics {
            not_pending: results
                .iter()
                .filter(|result| matches!(result, Err(StreamNextPollError::NotPending)))
                .count(),

            not_ready: results
                .iter()
                .filter(|result| matches!(result, Err(StreamNextPollError::NotReady)))
                .count(),
            wrong_result: results
                .iter()
                .filter(|result| matches!(result, Err(StreamNextPollError::WrongRestult { .. })))
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
