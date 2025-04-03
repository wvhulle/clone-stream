use std::{
    collections::{BTreeMap, VecDeque},
    pin::Pin,
    process::Output,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};
use log::trace;

use crate::fork_stage::{ForkStage, Waiting, Waking};

pub type OutputStreamId = usize;

pub(crate) struct ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(super) stream: Pin<Box<BaseStream>>,
    pub(super) outputs: BTreeMap<OutputStreamId, ForkStage<BaseStream::Item>>,
}

impl<BaseStream> ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub fn free_output_index(&self) -> OutputStreamId {
        if self.outputs.is_empty() {
            0
        } else {
            self.outputs.keys().max().unwrap() + 1
        }
    }

    pub fn handle_fork(
        &mut self,
        output_index: OutputStreamId,
        new_context: &mut Context,
    ) -> Poll<Option<<BaseStream as Stream>::Item>> {
        let mut status = self.outputs.entry(output_index).or_default();

        match &mut status {
            ForkStage::WakingUp(waking) => {
                let item = waking.processed.clone();
                waking
                    .remaining
                    .retain(|already_waiting| !already_waiting.will_wake(new_context.waker()));

                if waking.remaining.is_empty() {
                    self.outputs.remove(&output_index);
                }

                Poll::Ready(item)
            }
            ForkStage::Waiting(_) => self.wake_others_or_queue(output_index, new_context.waker()),
        }
    }

    pub fn notify_all_other_waiters_of_all_forks(
        &mut self,
        id: OutputStreamId,
        maybe_item: Option<&BaseStream::Item>,
        current_waker: &Waker,
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
                ForkStage::WakingUp(wakers) => !wakers.remaining.is_empty(),
                ForkStage::Waiting(_) => true,
            });

            self.outputs.values_mut().for_each(|status| match status {
                ForkStage::Waiting(waiting) => {
                    trace!("Found a waiting output stream. Waking it up.");
                    let remaining_tasks = waiting
                        .suspended_tasks
                        .iter()
                        .filter(|waiting_taks| !waiting_taks.will_wake(current_waker))
                        .cloned()
                        .collect::<VecDeque<_>>();

                    *status = ForkStage::WakingUp(Waking {
                        processed: maybe_item.cloned(),
                        remaining: remaining_tasks.clone(),
                    });
                    remaining_tasks.into_iter().for_each(Waker::wake);
                }
                ForkStage::WakingUp(waking_up) => {
                    let remaining: VecDeque<_> = waking_up
                        .remaining
                        .clone()
                        .into_iter()
                        .filter(|w| !w.will_wake(current_waker))
                        .collect();

                    remaining.iter().for_each(Waker::wake_by_ref);

                    *status = ForkStage::WakingUp(Waking {
                        processed: maybe_item.cloned(),
                        remaining: remaining.clone(),
                    });
                }
            });
        }
    }

    pub(super) fn wake_others_or_queue(
        &mut self,
        polled_output: OutputStreamId,
        current_task: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        match self
            .stream
            .poll_next_unpin(&mut Context::from_waker(current_task))
        {
            Poll::Ready(maybe_item) => {
                trace!(
                    "The input stream has resolved an item. Emitting it to all output streams. Not storing any waker for the current fork {polled_output}."
                );

                self.notify_all_other_waiters_of_all_forks(
                    polled_output,
                    maybe_item.as_ref(),
                    current_task,
                );

                Poll::Ready(maybe_item)
            }
            Poll::Pending => {
                trace!(
                    "The input stream is not ready yet. Marking fork {polled_output} as waiting."
                );
                self.outputs.insert(
                    polled_output,
                    ForkStage::Waiting(Waiting {
                        suspended_tasks: VecDeque::from([current_task.clone()]),
                    }),
                );

                Poll::Pending
            }
        }
    }
}

impl<BaseStream> From<BaseStream> for ForkBridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(stream: BaseStream) -> Self {
        Self {
            stream: Box::pin(stream),
            outputs: BTreeMap::new(),
        }
    }
}
