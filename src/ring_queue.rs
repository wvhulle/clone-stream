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

    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.items.clear();
        self.oldest = None;
        self.newest = None;
    }

    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        self.items.get(&index)
    }

    pub(crate) fn keys(&self) -> std::collections::btree_map::Keys<'_, usize, T> {
        self.items.keys()
    }

    pub(crate) fn push(&mut self, item: T) {
        if self.capacity == 0 {
            return;
        }
        if let Some(newest) = self.newest {
            let next_index = (newest + 1) % self.capacity;
            if self.items.contains_key(&next_index) {
                self.items.remove(&next_index);
                if self.oldest == Some(next_index) {
                    let next_oldest = self
                        .items
                        .range((next_index + 1)..)
                        .map(|(k, _)| *k)
                        .next()
                        .or_else(|| {
                            // Wrap around to beginning if needed
                            self.items.range(..next_index).map(|(k, _)| *k).next()
                        });
                    self.oldest = next_oldest;
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
        if Some(index) == self.newest
            && let Some(oldest) = self.oldest
        {
            match oldest.cmp(&index) {
                std::cmp::Ordering::Equal => {
                    self.newest = None;
                }
                std::cmp::Ordering::Less => {
                    // Not wrapping
                    self.newest = self
                        .items
                        .range((oldest)..index)
                        .next_back()
                        .map(|(k, _)| *k);
                }
                std::cmp::Ordering::Greater => {
                    // Wrapping
                    self.newest = self
                        .items
                        .range((oldest)..)
                        .chain(self.items.range(..index))
                        .next_back()
                        .map(|(k, _)| *k);
                }
            }
        }
        removed
    }

    pub fn pop_oldest(&mut self) -> Option<(usize, T)> {
        if let Some(oldest) = self.oldest
            && let Some(item) = self.items.remove(&oldest)
        {
            if self.items.is_empty() {
                self.oldest = None;
                self.newest = None;
            } else {
                // Use BTreeMap's range to efficiently find next item
                self.oldest = self
                    .items
                    .range((oldest + 1)..)
                    .next()
                    .map(|(k, _)| *k)
                    .or_else(|| self.items.keys().next().copied());
            }
            return Some((oldest, item));
        }
        None
    }

    pub(crate) fn is_newer_than(&self, maybe_newer: usize, current: usize) -> bool {
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
                // Use BTreeMap's range for efficient next lookup
                self.queue
                    .items
                    .range((current_idx + 1)..)
                    .chain(self.queue.items.range(..current_idx))
                    .next()
                    .map(|(k, _)| *k)
            } else {
                None
            };

            return Some((current_idx, item));
        }

        None
    }
}
