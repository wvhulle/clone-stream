use std::{
    collections::{BTreeMap, VecDeque},
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};
use log::trace;

#[derive(Default)]
pub struct ForkRef<Item> {
    pub pending_waker: Option<Waker>,
    pub items: VecDeque<Item>,
}

pub struct Bridge<BaseStream>
where
    BaseStream: Stream,
{
    pub base_stream: Pin<Box<BaseStream>>,
    pub forks: BTreeMap<usize, ForkRef<Option<BaseStream::Item>>>,
}

impl<BaseStream> Bridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    pub fn new(base_stream: BaseStream) -> Self {
        Self {
            base_stream: Box::pin(base_stream),
            forks: BTreeMap::default(),
        }
    }

    pub fn clear(&mut self) {
        self.forks.clear();
    }

    pub fn poll(&mut self, fork_id: usize, fork_waker: &Waker) -> Poll<Option<BaseStream::Item>> {
        trace!("Fork {fork_id} is being polled on the bridge.");
        let fork = self.forks.get_mut(&fork_id).unwrap();

        match fork.items.pop_front() {
            Some(item) => {
                trace!("Popping item for fork {fork_id} from queue");
                Poll::Ready(item)
            }
            None => {
                match self
                    .base_stream
                    .poll_next_unpin(&mut Context::from_waker(fork_waker))
                {
                    Poll::Pending => {
                        trace!("No ready item from input stream available for fork {fork_id}");
                        fork.pending_waker = Some(fork_waker.clone());
                        Poll::Pending
                    }
                    Poll::Ready(item) => {
                        trace!("Item ready from input stream for fork {fork_id}");

                        fork.pending_waker = None;

                        self.forks
                            .iter_mut()
                            .filter(|(other_fork, _)| fork_id != **other_fork)
                            .for_each(|(other_fork_id, other_fork)| {
                                if let Some(waker) = &other_fork.pending_waker {
                                     trace!("Pushing item from input stream on queue of other fork {other_fork_id} because fork {fork_id} was polled");
                                    other_fork.items.push_back(item.clone());
                                     trace!("Waking up fork {other_fork_id} because it was polled");
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

impl<BaseStream> Deref for Bridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    type Target = Pin<Box<BaseStream>>;

    fn deref(&self) -> &Self::Target {
        &self.base_stream
    }
}
impl<BaseStream> DerefMut for Bridge<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_stream
    }
}
