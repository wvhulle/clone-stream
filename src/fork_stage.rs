/// Each fork has its own set of wakers.
use std::{
    collections::{BTreeSet, VecDeque},
    sync::Weak,
    task::Waker,
};

use futures::Stream;

use crate::ForkedStream;

#[derive(Default)]
pub struct Waking<Item> {
    /// The most recent item:
    /// - it comes from the input stream
    /// - it may be null, just like all `next().await` values.
    /// - it is held here to be accessible to the remaining wakers.
    /// - this item is dropped by switching `WakerStatus` to `Waiting` or removing.
    pub(crate) latest_value: Option<Item>,
    /// The wakers that are waiting to be woken up and process the processed item.
    pub(crate) remaining: BTreeSet<Weak<Box<dyn Stream<Item = Item>>>>,
}

#[derive(Default)]
pub struct Waiting {
    /// The tasks that have not seen any item yet since their last next().await on the fork.
    pub(crate) waiting: BTreeSet<Weak<Box<dyn Stream<Item = ()>>>>,
}

/// Either a fork is still waiting to wake up, or it is already waking up.
pub enum ForkStage<Item> {
    Waiting(Waiting),
    Waking(Waking<Item>),
}
