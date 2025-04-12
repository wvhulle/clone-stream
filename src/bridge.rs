use std::{
    collections::{BTreeMap, VecDeque},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};
use log::trace;

#[derive(Default)]
pub struct UnseenByClone<Item> {
    pub suspended_task: Option<Waker>,
    pub unseen_items: VecDeque<Item>,
}

pub struct Bridge<BaseStream>
where
    BaseStream: Stream,
{
    pub base_stream: Pin<Box<BaseStream>>,
    pub clones: BTreeMap<usize, UnseenByClone<Option<BaseStream::Item>>>,
}

impl<BaseStream> Bridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub fn new(base_stream: BaseStream) -> Self {
        Self {
            base_stream: Box::pin(base_stream),
            clones: BTreeMap::default(),
        }
    }

    pub fn clear(&mut self) {
        self.clones.clear();
    }

    pub fn poll(&mut self, clone_id: usize, clone_waker: &Waker) -> Poll<Option<BaseStream::Item>> {
        trace!("Clone {clone_id} is being polled on the bridge.");
        let clone = self.clones.get_mut(&clone_id).unwrap();

        match clone.unseen_items.pop_front() {
            Some(item) => {
                trace!("Popping item for clone {clone_id} from queue");
                Poll::Ready(item)
            }
            None => {
                match self
                    .base_stream
                    .poll_next_unpin(&mut Context::from_waker(clone_waker))
                {
                    Poll::Pending => {
                        trace!("No ready item from input stream available for clone {clone_id}");
                        clone.suspended_task = Some(clone_waker.clone());
                        Poll::Pending
                    }
                    Poll::Ready(item) => {
                        trace!("Item ready from input stream for clone {clone_id}");

                        clone.suspended_task = None;

                        self.clones
                            .iter_mut()
                            .filter(|(other_clone, _)| clone_id != **other_clone)
                            .for_each(|(other_clone_id, other_clone)| {
                                if let Some(waker) = &other_clone.suspended_task {
                                    trace!(
                                        "Pushing item from input stream on queue of other clone \
                                         {other_clone_id} because clone {clone_id} was polled"
                                    );
                                    other_clone.unseen_items.push_back(item.clone());
                                    trace!(
                                        "Waking up clone {other_clone_id} because it was polled"
                                    );
                                    waker.wake_by_ref();
                                }
                            });

                        Poll::Ready(item)
                    }
                }
            }
        }
    }
}
