use std::collections::BTreeMap;

/// A ring buffer queue that wraps around at a maximum capacity.
/// Provides proper ordering semantics for ring buffer indices.
#[derive(Debug)]
pub(crate) struct RingQueue<T>
where
    T: Clone,
{
    items: BTreeMap<usize, T>,
    pub(crate) oldest: Option<usize>,
    pub(crate) newest: Option<usize>,
    capacity: usize,
}

impl<T> RingQueue<T>
where
    T: Clone,
{
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            items: BTreeMap::new(),
            oldest: None,
            newest: None,
            capacity,
        }
    }

    /// Inserts an item at the given index.
    pub(crate) fn insert(&mut self, item: T) {
        if self.capacity == 0 {
            return;
        }
        if let Some(newest) = self.newest {
            let next_index = (newest + 1) % self.capacity;
            // Check if we're overwriting an existing item
            if self.items.contains_key(&next_index) {
                self.items.remove(&next_index);
                // If we're removing the oldest item, update oldest pointer
                if self.oldest == Some(next_index) {
                    // Find the next oldest item in ring buffer order
                    let mut next_oldest = (next_index + 1) % self.capacity;
                    while !self.items.contains_key(&next_oldest) && next_oldest != next_index {
                        next_oldest = (next_oldest + 1) % self.capacity;
                    }
                    self.oldest = if next_oldest == next_index {
                        None
                    } else {
                        Some(next_oldest)
                    };
                }
            }
            self.items.insert(next_index, item);
            self.newest = Some(next_index);
        } else {
            self.newest = Some(0);
            self.oldest = Some(0);
            self.items.insert(0, item);
        }
    }

    pub(crate) fn remove(&mut self, index: usize) -> Option<T> {
        let removed = self.items.remove(&index);
        if Some(index) == self.oldest {
            // Update oldest to the next item
            self.oldest = self
                .items
                .range((index + 1)..self.capacity)
                .chain(self.items.range(0..index))
                .next()
                .map(|(k, _)| *k);
        }
        if Some(index) == self.newest {
            // Update newest to the previous item
            self.newest = self
                .items
                .range((index + 1)..self.capacity)
                .chain(self.items.range(0..index))
                .next_back()
                .map(|(k, _)| *k);
        }
        removed
    }

    /// Gets an item at the given index.
    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        self.items.get(&index)
    }

    pub(crate) fn is_strictly_newer_then(&self, maybe_newer: usize, current: usize) -> bool {
        match (self.oldest, self.newest) {
            (Some(oldest), Some(newest)) => {
                if oldest <= newest {
                    // Normal case: no wraparound
                    oldest <= current && current < maybe_newer && maybe_newer <= newest
                } else {
                    // Wraparound case
                    (oldest <= current && current < self.capacity && maybe_newer <= newest)
                        || (current < maybe_newer && maybe_newer <= newest)
                        || (oldest <= current
                            && current < maybe_newer
                            && maybe_newer < self.capacity)
                }
            }
            _ => false, // Empty queue case
        }
    }

    /// Returns true if the queue is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns an iterator over the keys.
    pub(crate) fn keys(&self) -> std::collections::btree_map::Keys<'_, usize, T> {
        self.items.keys()
    }

    /// Removes and returns the first key-value pair.
    pub(crate) fn oldest_with_index(&mut self) -> Option<(usize, T)> {
        if let Some(oldest) = self.oldest
            && let Some(item) = self.items.remove(&oldest)
        {
            // Update oldest pointer
            if self.items.is_empty() {
                self.oldest = None;
                self.newest = None;
            } else {
                // Find the next oldest item in ring buffer order
                let mut next_oldest = (oldest + 1) % self.capacity;
                while !self.items.contains_key(&next_oldest) && next_oldest != oldest {
                    next_oldest = (next_oldest + 1) % self.capacity;
                }
                if next_oldest == oldest {
                    self.oldest = None;
                    self.newest = None;
                } else {
                    self.oldest = Some(next_oldest);
                }
            }
            return Some((oldest, item));
        }
        None
    }

    pub(crate) fn newest_index(&self) -> Option<usize> {
        self.newest
    }
}

impl<'a, T> IntoIterator for &'a RingQueue<T>
where
    T: Clone,
{
    type Item = (usize, &'a T);
    type IntoIter = RingQueueIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        RingQueueIter::new(self)
    }
}

pub(crate) struct RingQueueIter<'a, T>
where
    T: Clone,
{
    queue: &'a RingQueue<T>,
    current: Option<usize>,
    remaining: usize,
}

impl<'a, T> RingQueueIter<'a, T>
where
    T: Clone,
{
    fn new(queue: &'a RingQueue<T>) -> Self {
        Self {
            queue,
            current: queue.oldest,
            remaining: queue.items.len(),
        }
    }
}

impl<'a, T> Iterator for RingQueueIter<'a, T>
where
    T: Clone,
{
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        if let Some(current_idx) = self.current
            && let Some(item) = self.queue.items.get(&current_idx)
        {
            self.remaining -= 1;

            // Find the next actual item in ring buffer order
            self.current = if self.remaining > 0 {
                let mut next_idx = (current_idx + 1) % self.queue.capacity;
                let start_idx = next_idx;

                // Find the next existing item, wrapping around if necessary
                while !self.queue.items.contains_key(&next_idx) && next_idx != start_idx {
                    next_idx = (next_idx + 1) % self.queue.capacity;
                }

                if self.queue.items.contains_key(&next_idx) {
                    Some(next_idx)
                } else {
                    None
                }
            } else {
                None
            };

            return Some((current_idx, item));
        }

        None
    }
}
