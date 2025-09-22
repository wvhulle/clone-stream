use std::collections::BTreeMap;

/// A ring buffer queue that wraps around at a maximum capacity.
/// Provides proper ordering semantics for ring buffer indices.
#[derive(Debug)]
pub(crate) struct RingQueue<T>
where
    T: Clone,
{
    pub(crate) items: BTreeMap<usize, T>,
    pub(crate) oldest: Option<usize>,
    pub(crate) newest: Option<usize>,
    capacity: usize,
}

impl<T> RingQueue<T>
where
    T: Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            items: BTreeMap::new(),
            oldest: None,
            newest: None,
            capacity,
        }
    }

    pub fn push(&mut self, item: T) {
        if self.capacity == 0 {
            return;
        }
        if let Some(newest) = self.newest {
            let next_index = (newest + 1) % self.capacity;
            if self.items.contains_key(&next_index) {
                self.items.remove(&next_index);
                if self.oldest == Some(next_index) {
                    self.oldest = self.next_ring_index(next_index);
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
        if self.capacity == 0 {
            return None;
        }
        let removed = self.items.remove(&index);
        if Some(index) == self.oldest {
            self.oldest = self.next_ring_index(index);
        }
        if Some(index) == self.newest {
            if self.oldest == Some(index) {
                self.newest = None;
            } else {
                self.newest = self.prev_ring_index(index);
            }
        }
        removed
    }

    pub fn pop_oldest(&mut self) -> Option<T> {
        if self.capacity == 0 {
            return None;
        }
        if let Some(oldest) = self.oldest
            && let Some(item) = self.items.remove(&oldest)
        {
            if self.items.is_empty() {
                self.oldest = None;
                self.newest = None;
            } else {
                self.oldest = self.next_ring_index(oldest);
            }
            return Some(item);
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.items.clear();
        self.oldest = None;
        self.newest = None;
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(&index)
    }

    /// Checks if an index is within the valid range of the ring buffer.
    /// Returns true if the index falls within the range from oldest to newest,
    /// accounting for wraparound when the buffer spans across the capacity boundary.
    fn is_valid_index(&self, index: usize) -> bool {
        if let (Some(oldest), Some(newest)) = (self.oldest, self.newest) {
            (oldest <= newest && index >= oldest && index <= newest)
                || (oldest > newest && (index >= oldest || index <= newest))
        } else {
            false
        }
    }

    /// Calculates the logical distance from one index to another in ring buffer order.
    /// Returns Some(distance) if both indices are valid and `to` comes after `from`,
    /// or None if the indices are invalid or `to` comes before `from` in the ordering.
    /// Handles wraparound correctly when the buffer spans across the capacity boundary.
    fn ring_distance(&self, from: usize, to: usize) -> Option<usize> {
        if self.is_valid_index(from) && self.is_valid_index(to) {
            let (oldest, newest) = (self.oldest?, self.newest?);

            if oldest <= newest {
                // No wraparound case
                if to >= from { Some(to - from) } else { None }
            } else {
                // Wraparound case - use modular arithmetic
                let distance = (to + self.capacity - from) % self.capacity;
                Some(distance)
            }
        } else {
            None
        }
    }

    fn next_ring_index(&self, from: usize) -> Option<usize> {
        self.items
            .range((from + 1)..)
            .chain(self.items.range(..from))
            .next()
            .map(|(k, _)| *k)
    }

    fn prev_ring_index(&self, from: usize) -> Option<usize> {
        self.items
            .range(..from)
            .chain(self.items.range((from + 1)..))
            .next_back()
            .map(|(k, _)| *k)
    }

    pub(crate) fn is_newer_than(&self, maybe_newer: usize, current: usize) -> bool {
        self.ring_distance(current, maybe_newer)
            .is_some_and(|distance| distance > 0)
    }
}

pub struct RingQueueIter<'a, T>
where
    T: Clone,
{
    queue: &'a RingQueue<T>,
    current_index: Option<usize>,
    remaining_items: usize,
}

impl<'a, T> RingQueueIter<'a, T>
where
    T: Clone,
{
    fn new(queue: &'a RingQueue<T>) -> Self {
        Self {
            queue,
            current_index: queue.oldest,
            remaining_items: queue.items.len(),
        }
    }
}

impl<'a, T> Iterator for RingQueueIter<'a, T>
where
    T: Clone,
{
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining_items == 0 {
            return None;
        }

        if let Some(index) = self.current_index
            && let Some(item) = self.queue.items.get(&index)
        {
            self.remaining_items -= 1;

            self.current_index = if self.remaining_items > 0 {
                self.queue.next_ring_index(index)
            } else {
                None
            };

            return Some((index, item));
        }

        None
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
