/// Represents the state of an item in the clone stream queue
#[derive(Debug, Clone)]
pub(crate) enum QueueItem<T> {
    /// An item that is available and ready to be consumed by clones
    Available(T),
    /// An item that was consumed but may still be referenced by some clones
    /// This could be useful for tracking purposes or future optimizations
    Consumed(T),
    /// A placeholder for future extensibility - could represent items that
    /// are being processed or in some intermediate state
    Processing(T),
}

impl<T> QueueItem<T> {
    /// Get the item value regardless of its state
    pub(crate) fn value(&self) -> &T {
        match self {
            QueueItem::Available(item) => item,
            QueueItem::Consumed(item) => item,
            QueueItem::Processing(item) => item,
        }
    }

    /// Take the item value, consuming the QueueItem
    pub(crate) fn into_value(self) -> T {
        match self {
            QueueItem::Available(item) => item,
            QueueItem::Consumed(item) => item,
            QueueItem::Processing(item) => item,
        }
    }

    /// Check if the item is available for consumption
    pub(crate) fn is_available(&self) -> bool {
        matches!(self, QueueItem::Available(_))
    }

    /// Mark the item as consumed
    pub(crate) fn mark_consumed(self) -> Self {
        match self {
            QueueItem::Available(item) => QueueItem::Consumed(item),
            other => other, // Already consumed or processing
        }
    }
}
