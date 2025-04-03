use std::{
    collections::{BTreeMap, VecDeque},
    pin::Pin,
    sync::Mutex,
    task::{Context, Poll, Waker},
};

use dashmap::DashMap;
use futures::{Stream, StreamExt};
use log::trace;

use crate::fork_stage::{ForkStage, Waiting, Waking};

pub(crate) type ForkId = usize;

pub(crate) struct ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(super) stream: Mutex<Pin<Box<BaseStream>>>,
    pub(super) outputs: DashMap<ForkId, ForkStage<BaseStream::Item>>,
}

impl<BaseStream> ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub fn new_fork(&self) -> ForkId {
        let entry = self
            .outputs
            .entry(if self.outputs.is_empty() {
                0
            } else {
                self.outputs.iter().map(|e| *e.pair().0).max().unwrap() + 1
            })
            .or_default();

        *entry.key()
    }

    fn new_fork_waker(&self, waker: &Waker) {
        self.outputs
            .entry(if self.outputs.is_empty() {
                0
            } else {
                self.outputs.iter().map(|e| *e.pair().0).max().unwrap() + 1
            })
            .or_insert(ForkStage::Waiting(Waiting {
                suspended_tasks: VecDeque::from([waker.clone()]),
            }));
    }

    pub(super) fn handle_fork(
        &self,
        polled_output: ForkId,
        current_task: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        trace!("The input stream is being polled.");
        let poll = self
            .stream
            .lock()
            .unwrap()
            .poll_next_unpin(&mut Context::from_waker(current_task));
        trace!("The input stream has been polled.");
        match poll {
            Poll::Ready(maybe_item) => {
                trace!(
                    "The input stream has resolved an item. Emitting it to all output streams. Not storing any waker for the current fork {polled_output}."
                );

                self.notify_all_other_waiters_of_all_forks(polled_output, maybe_item.as_ref());

                Poll::Ready(maybe_item)
            }
            Poll::Pending => {
                if self.outputs.contains_key(&polled_output) {
                    let mut entry = self.outputs.get_mut(&polled_output).unwrap();
                    let mut status = entry.value_mut();

                    match &mut status {
                        ForkStage::Waking(waking) => {
                            trace!(
                                "The input stream did not yield any new value and fork {polled_output} was waking."
                            );

                            waking.remaining.retain(|w| !w.will_wake(current_task));

                            Poll::Ready(waking.processed.clone())
                        }
                        ForkStage::Waiting(waiting) => {
                            if !waiting
                                .suspended_tasks
                                .iter()
                                .any(|w| w.will_wake(current_task))
                            {
                                trace!("This task will be woken up in the future..");
                                waiting.suspended_tasks.push_back(current_task.clone());
                            }

                            Poll::Pending
                        }
                    }
                } else {
                    trace!(
                        "The input stream is not ready yet. Marking fork {polled_output} as waiting."
                    );
                    self.new_fork_waker(current_task);

                    Poll::Pending
                }
            }
        }
    }

    fn notify_all_other_waiters_of_all_forks(
        &self,
        id: ForkId,
        maybe_item: Option<&BaseStream::Item>,
    ) {
        trace!("A poll on the output stream {id} triggered a result from the input stream.");

        if self.outputs.is_empty() {
            trace!("No other forked output streams are waiting that should be woken.");
        } else {
            trace!(
                "Checking {} other forked output streams for wakeups.",
                self.outputs.len()
            );

            self.outputs.retain(|_, status| match status {
                ForkStage::Waking(wakers) => !wakers.remaining.is_empty(),
                ForkStage::Waiting(_) => true,
            });

            self.outputs.iter_mut().for_each(|mut entry| {
                let (fork_id, status) = entry.pair_mut();
                match status {
                    ForkStage::Waiting(waiting) => {
                        trace!("Fork {fork_id} was waiting for a new item.");

                        let remaining = waiting.suspended_tasks.clone();

                        *status = ForkStage::Waking(Waking {
                            processed: maybe_item.cloned(),
                            remaining: remaining.clone(),
                        });

                        trace!(
                            "The remaining {} wakers for fork {} are woken up.",
                            remaining.len(),
                            fork_id
                        );

                        remaining.iter().for_each(Waker::wake_by_ref);
                    }
                    ForkStage::Waking(waking_up) => {
                        let remaining: VecDeque<_> = waking_up.remaining.clone();

                        remaining.iter().for_each(Waker::wake_by_ref);

                        *status = ForkStage::Waking(Waking {
                            processed: maybe_item.cloned(),
                            remaining: remaining.clone(),
                        });
                    }
                }
            });
        }
    }
}

impl<BaseStream> From<BaseStream> for ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(stream: BaseStream) -> Self {
        Self {
            stream: Mutex::new(Box::pin(stream)),
            outputs: DashMap::new(),
        }
    }
}
