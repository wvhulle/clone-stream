use std::task::{Context, Poll, Waker};

use futures::{Stream, StreamExt};
use log::trace;

use crate::Fork;

/// Common utility for polling the base stream and inserting items into queue if
/// needed
pub(super) fn poll_base_stream<BaseStream>(
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
            trace!("The base stream is ready.");
            if fork.has_other_clones_waiting(clone_id) {
                trace!("Other clones are waiting for the new item.");
                fork.queue.insert(item.clone());
            } else {
                trace!("No other clone is interested in the new item.");
            }
            Poll::Ready(item)
        }
        Poll::Pending => {
            trace!("The base stream is pending.");
            Poll::Pending
        }
    }
}
