use core::ops::Deref;
use std::{
    collections::BTreeMap,
    pin::Pin,
    task::{Poll, Waker},
};

use futures::Stream;
use log::trace;

pub struct SiblingClone {
    pub(crate) id: usize,
    pub(crate) last_seen: Option<usize>,
    pub(crate) waker: Option<Waker>,
    pub(crate) state: CloneState,
}

#[derive(Default)]
pub(crate) enum CloneState {
    #[default]
    UpToDate,
    ReadyToPop,
    Suspended,
}

pub(crate) struct Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) base_stream: Pin<Box<BaseStream>>,
    pub(crate) queue: BTreeMap<usize, Option<BaseStream::Item>>,
    pub(crate) clones: BTreeMap<usize, SiblingClone>,
    pub(crate) next_queue_index: usize,
}

impl<BaseStream> Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) fn new(base_stream: BaseStream) -> Self {
        Self {
            base_stream: Box::pin(base_stream),
            clones: BTreeMap::default(),
            queue: BTreeMap::new(),
            next_queue_index: 0,
        }
    }

    pub(crate) fn poll_clone(
        &mut self,
        clone_id: usize,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        trace!("Clone {clone_id} is being polled on the split.");
        let mut clone = self.clones.remove(&clone_id).unwrap();

        let poll = match &mut clone.state {
            CloneState::UpToDate => self.fetch_input_item(&mut clone, clone_waker),
            CloneState::Suspended => self.wake_up(&mut clone, clone_waker),
            CloneState::ReadyToPop => self.try_pop_queue(&mut clone, clone_waker),
        };

        self.clones.insert(clone_id, clone);
        poll
    }

    pub(crate) fn register(&mut self) -> usize {
        let min_available = (0..)
            .filter(|n| !self.clones.contains_key(n))
            .nth(0)
            .unwrap();

        trace!("Registering clone {min_available}.");
        self.clones.insert(
            min_available,
            SiblingClone {
                state: CloneState::default(),
                id: min_available,
                last_seen: None,
                waker: None,
            },
        );
        min_available
    }

    pub(crate) fn unregister(&mut self, clone_id: usize) {
        trace!("Unregistering clone {clone_id}.");
        self.clones.remove(&clone_id).unwrap();

        self.queue.retain(|item_index, _| {
            self.clones.values().any(|state| {
                state
                    .last_seen
                    .is_some_and(|last_index| *item_index > last_index)
            })
        });
    }
}

impl<BaseStream> Deref for Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    type Target = BaseStream;

    fn deref(&self) -> &Self::Target {
        &self.base_stream
    }
}
