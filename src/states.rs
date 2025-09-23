use std::{
    fmt::Debug,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};
use log::trace;

use crate::Fork;

/// Represents the state of a clone in the stream cloning state machine.
///
/// Each clone maintains its own state to track its position relative to the base stream
/// and the shared queue. The state determines how the clone should behave when polled.
#[derive(Clone, Debug)]
pub(crate) enum CloneState {
    /// Queue is empty, can poll base stream directly
    QueueEmpty,

    /// Queue is empty, base stream is pending
    QueueEmptyPending { waker: Waker },

    /// All queue items consumed, base stream is pending
    AllSeenPending {
        waker: Waker,
        last_seen_index: usize,
    },

    /// All queue items consumed, can poll base stream
    AllSeen,

    /// Has unseen queue item ready to consume
    UnseenReady { unseen_index: usize },
}

impl Default for CloneState {
    fn default() -> Self {
        Self::QueueEmpty
    }
}

impl CloneState {
    pub(crate) fn queue_empty_pending(waker: &Waker) -> Self {
        Self::QueueEmptyPending {
            waker: waker.clone(),
        }
    }

    pub(crate) fn queue_empty() -> Self {
        Self::QueueEmpty
    }

    pub(crate) fn all_seen_pending(waker: &Waker, index: usize) -> Self {
        Self::AllSeenPending {
            waker: waker.clone(),
            last_seen_index: index,
        }
    }

    pub(crate) fn all_seen() -> Self {
        Self::AllSeen
    }

    pub(crate) fn unseen_ready(index: usize) -> Self {
        Self::UnseenReady {
            unseen_index: index,
        }
    }

    pub(crate) fn should_still_see_base_item(&self) -> bool {
        match self {
            CloneState::QueueEmptyPending { .. } | CloneState::AllSeenPending { .. } => true,
            CloneState::QueueEmpty | CloneState::AllSeen | CloneState::UnseenReady { .. } => false,
        }
    }

    pub(crate) fn waker(&self) -> Option<Waker> {
        match self {
            CloneState::QueueEmptyPending { waker } | CloneState::AllSeenPending { waker, .. } => {
                Some(waker.clone())
            }
            CloneState::QueueEmpty | CloneState::AllSeen | CloneState::UnseenReady { .. } => None,
        }
    }

    #[inline]
    fn transition_on_poll<Item>(
        &mut self,
        poll_result: Poll<Option<Item>>,
        ready_state: CloneState,
        pending_state: CloneState,
    ) -> Poll<Option<Item>> {
        match poll_result {
            Poll::Ready(item) => {
                *self = ready_state;
                Poll::Ready(item)
            }
            Poll::Pending => {
                *self = pending_state;
                Poll::Pending
            }
        }
    }
}

impl CloneState {
    /// Main state machine step - processes the clone's current state
    #[inline]
    pub(crate) fn step<BaseStream>(
        &mut self,
        clone_id: usize,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> Poll<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        match self {
            CloneState::QueueEmpty => self.handle_queue_empty(clone_id, waker, fork),
            CloneState::QueueEmptyPending { .. } => {
                self.handle_queue_empty_pending(clone_id, waker, fork)
            }
            CloneState::AllSeenPending {
                last_seen_index, ..
            } => {
                let index = *last_seen_index;
                self.handle_all_seen_pending(clone_id, waker, fork, index)
            }
            CloneState::AllSeen => self.handle_all_seen(clone_id, waker, fork),
            CloneState::UnseenReady { unseen_index } => {
                let index = *unseen_index;
                self.handle_unseen_ready(clone_id, waker, fork, index)
            }
        }
    }

    #[inline]
    fn handle_queue_empty<BaseStream>(
        &mut self,
        clone_id: usize,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> Poll<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        self.transition_on_poll(
            poll_base_with_queue_check(clone_id, waker, fork),
            CloneState::queue_empty(),
            next_pending_state(waker, fork),
        )
    }

    #[inline]
    fn handle_queue_empty_pending<BaseStream>(
        &mut self,
        clone_id: usize,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> Poll<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        if fork.queue.is_empty() {
            self.transition_on_poll(
                poll_base_with_queue_check(clone_id, waker, fork),
                CloneState::queue_empty(),
                CloneState::queue_empty_pending(waker),
            )
        } else {
            let Some(queue_index) = fork.queue.oldest else {
                *self = CloneState::queue_empty_pending(waker);
                return Poll::Pending;
            };

            let item = process_oldest_queue_item(fork, clone_id, queue_index);
            *self = CloneState::unseen_ready(queue_index);
            Poll::Ready(item)
        }
    }

    #[inline]
    fn handle_all_seen_pending<BaseStream>(
        &mut self,
        clone_id: usize,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
        last_seen_index: usize,
    ) -> Poll<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        if let Some((newer_index, item)) = process_newer_queue_item(fork, last_seen_index) {
            *self = CloneState::unseen_ready(newer_index);
            Poll::Ready(item)
        } else {
            self.transition_on_poll(
                poll_base_stream(clone_id, waker, fork),
                CloneState::all_seen(),
                CloneState::all_seen_pending(waker, last_seen_index),
            )
        }
    }

    #[inline]
    fn handle_all_seen<BaseStream>(
        &mut self,
        clone_id: usize,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
    ) -> Poll<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        let pending_state = if let Some(oldest_index) = fork.queue.oldest {
            CloneState::all_seen_pending(waker, oldest_index)
        } else {
            CloneState::queue_empty_pending(waker)
        };

        self.transition_on_poll(
            poll_base_stream(clone_id, waker, fork),
            CloneState::all_seen(),
            pending_state,
        )
    }

    #[inline]
    fn handle_unseen_ready<BaseStream>(
        &mut self,
        clone_id: usize,
        waker: &Waker,
        fork: &mut Fork<BaseStream>,
        current_index: usize,
    ) -> Poll<Option<BaseStream::Item>>
    where
        BaseStream: Stream<Item: Clone>,
    {
        if let Some((newer_index, item)) = process_newer_queue_item(fork, current_index) {
            *self = CloneState::unseen_ready(newer_index);
            Poll::Ready(item)
        } else {
            self.transition_on_poll(
                poll_base_stream(clone_id, waker, fork),
                CloneState::all_seen(),
                CloneState::all_seen_pending(waker, current_index),
            )
        }
    }
}

pub(crate) fn poll_base_stream<BaseStream>(
    clone_id: usize,
    waker: &Waker,
    fork: &mut Fork<BaseStream>,
) -> Poll<Option<BaseStream::Item>>
where
    BaseStream: Stream<Item: Clone>,
{
    match fork
        .base_stream
        .poll_next_unpin(&mut Context::from_waker(&fork.waker(waker)))
    {
        Poll::Ready(item) => {
            trace!("Base stream ready with item");
            if fork.has_other_clones_waiting(clone_id) {
                trace!("Queuing item for other waiting clones");
                fork.queue.push(item.clone());
            } else {
                trace!("No other clones waiting, not queuing item");
            }
            Poll::Ready(item)
        }
        Poll::Pending => {
            trace!("Base stream pending");
            Poll::Pending
        }
    }
}

fn poll_base_with_queue_check<BaseStream>(
    clone_id: usize,
    waker: &Waker,
    fork: &mut Fork<BaseStream>,
) -> Poll<Option<BaseStream::Item>>
where
    BaseStream: Stream<Item: Clone>,
{
    match fork
        .base_stream
        .poll_next_unpin(&mut Context::from_waker(&fork.waker(waker)))
    {
        Poll::Ready(item) => {
            trace!("Base stream ready with item");

            if fork.has_other_clones_waiting(clone_id) {
                trace!("Queuing item for other interested clones");
                fork.queue.push(item.clone());
            } else {
                trace!("No other clones need this item");
            }
            Poll::Ready(item)
        }
        Poll::Pending => {
            trace!("Base stream pending");
            Poll::Pending
        }
    }
}

#[inline]
fn next_queue_item_after<BaseStream>(fork: &Fork<BaseStream>, current_index: usize) -> Option<usize>
where
    BaseStream: Stream<Item: Clone>,
{
    fork.queue.find_next_newer_index(current_index)
}

fn next_pending_state<BaseStream>(waker: &Waker, fork: &Fork<BaseStream>) -> CloneState
where
    BaseStream: Stream<Item: Clone>,
{
    if fork.queue.is_empty() {
        CloneState::queue_empty_pending(waker)
    } else if let Some(newest_index) = fork.queue.newest {
        CloneState::all_seen_pending(waker, newest_index)
    } else {
        CloneState::queue_empty_pending(waker)
    }
}

#[inline]
fn process_oldest_queue_item<BaseStream>(
    fork: &mut Fork<BaseStream>,
    clone_id: usize,
    queue_index: usize,
) -> Option<BaseStream::Item>
where
    BaseStream: Stream<Item: Clone>,
{
    if fork.active_clone_count() <= 1 {
        return fork.queue.pop_oldest().unwrap();
    }

    if other_clones_need_item(fork, clone_id, queue_index) {
        fork.queue.get(queue_index).unwrap().clone()
    } else {
        fork.queue.pop_oldest().unwrap()
    }
}

#[inline]
fn other_clones_need_item<BaseStream>(
    fork: &Fork<BaseStream>,
    exclude_clone_id: usize,
    queue_index: usize,
) -> bool
where
    BaseStream: Stream<Item: Clone>,
{
    fork.clones.iter().enumerate().any(|(clone_id, state_opt)| {
        clone_id != exclude_clone_id
            && state_opt.is_some()
            && fork.clone_should_still_see_item(clone_id, queue_index)
    })
}

#[inline]
fn process_newer_queue_item<BaseStream>(
    fork: &mut Fork<BaseStream>,
    current_index: usize,
) -> Option<(usize, Option<BaseStream::Item>)>
where
    BaseStream: Stream<Item: Clone>,
{
    let newer_index = next_queue_item_after(fork, current_index)?;
    let item = get_queue_item_optimally(fork, newer_index);
    Some((newer_index, item))
}

#[inline]
fn get_queue_item_optimally<BaseStream>(
    fork: &mut Fork<BaseStream>,
    index: usize,
) -> Option<BaseStream::Item>
where
    BaseStream: Stream<Item: Clone>,
{
    if fork.active_clone_count() <= 1 {
        return fork.queue.remove(index).unwrap();
    }

    let clones_needing_item = count_clones_needing_item(fork, index);

    match clones_needing_item {
        0 | 1 => fork.queue.remove(index).unwrap(),
        _ => fork.queue.get(index).unwrap().clone(),
    }
}

#[inline]
fn count_clones_needing_item<BaseStream>(fork: &Fork<BaseStream>, index: usize) -> usize
where
    BaseStream: Stream<Item: Clone>,
{
    fork.clones
        .iter()
        .enumerate()
        .filter(|(_, state_opt)| state_opt.is_some())
        .filter(|(clone_id, _)| fork.clone_should_still_see_item(*clone_id, index))
        .count()
}
