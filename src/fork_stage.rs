/// Each fork has its own set of wakers.
use std::{collections::VecDeque, task::Waker};

use crate::OutputStreamId;

#[derive(Default)]
pub struct Waking<Item> {
    /// The most recent item:
    /// - it comes from the input stream
    /// - it may be null, just like all `next().await` values.
    /// - it is held here to be accessible to the remaining wakers.
    /// - this item is dropped by switching `WakerStatus` to `Waiting` or removing.
    pub(crate) processed: Option<Item>,
    /// The wakers that are waiting to be woken up and process the processed item.
    pub(crate) remaining: VecDeque<Waker>,
}

#[derive(Default)]
pub struct Waiting {
    /// The tasks that have not seen any item yet since their last next().await on the fork.
    pub(crate) suspended_tasks: VecDeque<Waker>,
}

/// Either a fork is still waiting to wake up, or it is already waking up.
pub enum ForkStage<Item> {
    Waiting(Waiting),
    WakingUp(Waking<Item>),
}

impl<Item> Default for ForkStage<Item> {
    fn default() -> Self {
        Self::Waiting(Waiting {
            suspended_tasks: VecDeque::new(),
        })
    }
}
