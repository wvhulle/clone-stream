use core::ops::Deref;
use std::{
    collections::{BTreeMap, HashMap},
    pin::Pin,
    task::{Poll, Waker},
};

use futures::Stream;
use log::trace;

use crate::transitions::{CloneState, OutputStatePoll};

pub(crate) struct Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) base_stream: Pin<Box<BaseStream>>,
    pub(crate) queue: BTreeMap<usize, Option<BaseStream::Item>>,
    pub(crate) clones: HashMap<usize, CloneState>,
    next_clone_index: usize,
    pub(crate) next_queue_index: usize,
}

impl<BaseStream> Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub(crate) fn new(base_stream: BaseStream) -> Self {
        Self {
            base_stream: Box::pin(base_stream),
            clones: HashMap::default(),
            queue: BTreeMap::new(),
            next_queue_index: 0,
            next_clone_index: 0,
        }
    }

    pub(crate) fn poll_clone(
        &mut self,
        clone_id: usize,
        clone_waker: &Waker,
    ) -> Poll<Option<BaseStream::Item>> {
        trace!("Clone {clone_id} is being polled on the split.");
        let current_state = self.clones.remove(&clone_id).unwrap();

        let OutputStatePoll {
            poll_result,
            state: new_state,
        } = match current_state {
            CloneState::UpToDate => self.fetch_input_item(clone_waker),
            CloneState::Suspended(suspended) => suspended.wake_up(clone_waker, self),
            CloneState::ReadyToPop(_) => self.try_pop_queue(clone_waker),
        };

        trace!("Inserting clone {clone_id} back into the clone with state: {new_state:?}.");
        self.clones.insert(clone_id, new_state);
        poll_result
    }

    #[allow(clippy::maybe_infinite_iter)]
    pub(crate) fn register(&mut self) -> usize {
        let min_available = self.next_clone_index;

        trace!("Registering clone {min_available}.");
        self.clones.insert(min_available, CloneState::default());

        self.next_clone_index += 1;
        min_available
    }

    pub(crate) fn unregister(&mut self, clone_id: usize) {
        trace!("Unregistering clone {clone_id}.");
        self.clones.remove(&clone_id).unwrap();

        self.queue.retain(|item_index, _| {
            self.clones
                .values()
                .any(|state| state.older_than(*item_index))
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
