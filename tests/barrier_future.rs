use core::task::{Poll, Waker};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use futures::FutureExt;
use log::trace;

pub enum SharedBarriedFuture {
    ThresholdNotReached {
        polled_pending_not_woken_yet: VecDeque<Waker>,
        remaining: usize,
        observers: VecDeque<Waker>,
    },
    ReachedThreshold {
        polled_pending_not_woken_yet: VecDeque<Waker>,
        remaining_observers: VecDeque<Waker>,
    },
    AllWoken,
    Unstarted {
        required_amount: usize,
        observers: VecDeque<Waker>,
    },
}

impl SharedBarriedFuture {
    #[must_use]
    pub fn new(required_amount: usize) -> Self {
        SharedBarriedFuture::Unstarted {
            required_amount,
            observers: VecDeque::new(),
        }
    }
}

#[derive(Clone)]
pub struct BarrierFuture<Fut>
where
    Fut: Future + Unpin,
{
    future: Fut,
    shared_state: Arc<Mutex<SharedBarriedFuture>>,
}

impl<Fut> BarrierFuture<Fut>
where
    Fut: Future + Unpin,
{
    pub fn new(future: Fut, shared_state: Arc<Mutex<SharedBarriedFuture>>) -> Self {
        BarrierFuture {
            future,
            shared_state,
        }
    }
}

impl<F> Future for BarrierFuture<F>
where
    F: Future + Unpin,
{
    type Output = F::Output;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        trace!("Polling the shared state");
        let mut shared_state = self.shared_state.lock().unwrap();

        match &mut *shared_state {
            SharedBarriedFuture::ThresholdNotReached {
                polled_pending_not_woken_yet,
                remaining: still_to_wake_up,
                observers,
            } => {
                trace!("The threshold is not reached yet.");

                if still_to_wake_up == &1
                    && !polled_pending_not_woken_yet
                        .iter()
                        .any(|w| w.will_wake(cx.waker()))
                {
                    polled_pending_not_woken_yet
                        .iter()
                        .for_each(std::task::Waker::wake_by_ref);
                    observers.iter().for_each(std::task::Waker::wake_by_ref);

                    *shared_state = SharedBarriedFuture::ReachedThreshold {
                        polled_pending_not_woken_yet: polled_pending_not_woken_yet.clone(),
                        remaining_observers: observers.clone(),
                    };
                } else {
                    trace!("At least one more task is required to start a poll.");
                    if !polled_pending_not_woken_yet
                        .iter()
                        .any(|w| w.will_wake(cx.waker()))
                    {
                        trace!("The current task has not polled yet, it is added to the queue.");

                        polled_pending_not_woken_yet.push_back(cx.waker().clone());
                        *still_to_wake_up -= 1;
                    }
                }
            }
            SharedBarriedFuture::ReachedThreshold {
                polled_pending_not_woken_yet,
                ..
            } => {
                trace!("Enough tasks have polled and the threshold was reached.");
                polled_pending_not_woken_yet.retain(|w| !w.will_wake(cx.waker()));
            }
            SharedBarriedFuture::AllWoken => {
                unreachable!();
            }
            SharedBarriedFuture::Unstarted {
                required_amount,
                observers,
            } => {
                if required_amount == &0 {
                    trace!("No tasks should have polled. Just returning poll immediately.");
                    observers.iter().for_each(std::task::Waker::wake_by_ref);
                    *shared_state = SharedBarriedFuture::AllWoken;
                } else {
                    trace!(
                        "No tasks have polled before. This task is the first task to be added to \
                         the queue."
                    );
                    *shared_state = SharedBarriedFuture::ThresholdNotReached {
                        polled_pending_not_woken_yet: VecDeque::from([cx.waker().clone()]),

                        remaining: *required_amount - 1,
                        observers: observers.clone(),
                    };
                }
            }
        }
        drop(shared_state);
        self.future.poll_unpin(cx)
    }
}

pub struct PendingFutureObserver {
    shared_state: Arc<Mutex<SharedBarriedFuture>>,
}

impl PendingFutureObserver {
    pub fn new(shared_state: Arc<Mutex<SharedBarriedFuture>>) -> Self {
        PendingFutureObserver { shared_state }
    }
}

impl Future for PendingFutureObserver {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        trace!("Polling the observer");
        let mut shared = self.shared_state.lock().unwrap();
        match &mut *shared {
            SharedBarriedFuture::ThresholdNotReached { observers, .. } => {
                if !observers.iter().any(|w| w.will_wake(cx.waker())) {
                    trace!(
                        "The current observer task has not been polled yet, it is added to the \
                         queue."
                    );
                    observers.push_back(cx.waker().clone());
                }
                Poll::Pending
            }
            SharedBarriedFuture::ReachedThreshold {
                remaining_observers,
                ..
            } => {
                trace!(
                    "The observer task was polled, but the polling tasks already reached a \
                     treshold. Removing this obsevert task from the queue."
                );
                remaining_observers.retain(|w| !w.will_wake(cx.waker()));

                Poll::Ready(())
            }
            SharedBarriedFuture::AllWoken => Poll::Ready(()),
            SharedBarriedFuture::Unstarted { observers, .. } => {
                if !observers.iter().any(|w| w.will_wake(cx.waker())) {
                    trace!(
                        "The current observer task has not been polled yet, it is added to the \
                         queue."
                    );
                    observers.push_back(cx.waker().clone());
                }
                Poll::Pending
            }
        }
    }
}
