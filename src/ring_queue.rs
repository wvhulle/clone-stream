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

    pub(crate) fn insert(&mut self, item: T) {
        if self.capacity == 0 {
            return;
        }
        if let Some(newest) = self.newest {
            let next_index = (newest + 1) % self.capacity;
            if self.items.contains_key(&next_index) {
                self.items.remove(&next_index);
                if self.oldest == Some(next_index) {
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
            self.oldest = self
                .items
                .range((index + 1)..self.capacity)
                .chain(self.items.range(0..index))
                .next()
                .map(|(k, _)| *k);
        }
        if Some(index) == self.newest {
            self.newest = self
                .items
                .range((index + 1)..self.capacity)
                .chain(self.items.range(0..index))
                .next_back()
                .map(|(k, _)| *k);
        }
        removed
    }

    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        self.items.get(&index)
    }

    pub(crate) fn is_strictly_newer_then(&self, maybe_newer: usize, current: usize) -> bool {
        match (self.oldest, self.newest) {
            (Some(oldest), Some(newest)) => {
                if oldest <= newest {
                    oldest <= current && current < maybe_newer && maybe_newer <= newest
                } else {
                    (oldest <= current && current < self.capacity && maybe_newer <= newest)
                        || (current < maybe_newer && maybe_newer <= newest)
                        || (oldest <= current
                            && current < maybe_newer
                            && maybe_newer < self.capacity)
                }
            }
            _ => false,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn keys(&self) -> std::collections::btree_map::Keys<'_, usize, T> {
        self.items.keys()
    }

    pub(crate) fn oldest_with_index(&mut self) -> Option<(usize, T)> {
        if let Some(oldest) = self.oldest
            && let Some(item) = self.items.remove(&oldest)
        {
            if self.items.is_empty() {
                self.oldest = None;
                self.newest = None;
            } else {
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

            self.current = if self.remaining > 0 {
                let mut next_idx = (current_idx + 1) % self.queue.capacity;
                let start_idx = next_idx;

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
