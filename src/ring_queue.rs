use std::collections::BTreeMap;

use log::trace;

/// A ring buffer queue that wraps around at a maximum capacity.
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

        // If queue is at capacity, remove oldest item first
        if self.items.len() >= self.capacity
            && let Some(oldest) = self.oldest
        {
            self.items.remove(&oldest);
            self.oldest = self.next_ring_index(oldest);
        }

        if let Some(newest) = self.newest {
            let next_index = (newest + 1) % self.capacity;
            self.items.insert(next_index, item);
            self.newest = Some(next_index);

            // Update oldest if this is the first item after being empty
            if self.oldest.is_none() {
                self.oldest = Some(next_index);
            }
        } else {
            // First item
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

    pub fn oldest_index(&self) -> Option<usize> {
        if self.is_empty() { None } else { self.oldest }
    }

    pub(crate) fn clear(&mut self) {
        self.items.clear();
        self.oldest = None;
        self.newest = None;
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(&index)
    }

    /// Checks if an index is within the valid range of the ring
    /// buffer.boundary.
    fn is_valid_index(&self, index: usize) -> bool {
        if let (Some(oldest), Some(newest)) = (self.oldest, self.newest) {
            (oldest <= newest && index >= oldest && index <= newest)
                || (oldest > newest && (index >= oldest || index <= newest))
        } else {
            false
        }
    }

    /// Calculates the logical distance from one index to another in ring buffer
    /// order.
    fn ring_distance(&self, from: usize, to: usize) -> Option<usize> {
        if self.is_valid_index(from) && self.is_valid_index(to) {
            let (oldest, newest) = (self.oldest?, self.newest?);

            if oldest <= newest {
                if to >= from { Some(to - from) } else { None }
            } else {
                // Wraparound case
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

    /// Returns the first valid index newer than `current_index`, or None if no
    /// such index exists.
    pub(crate) fn find_next_newer_index(&self, current_index: usize) -> Option<usize> {
        let (oldest, newest) = (self.oldest?, self.newest?);
        trace!("Finding next newer index after {current_index}, oldest={oldest}, newest={newest}");
        trace!("Current queue has length {:?}", self.items.len());
        // Check consecutive index first
        let next_consecutive = (current_index + 1) % self.capacity;

        trace!("Next consecutive index is {next_consecutive}");
        if self.items.contains_key(&next_consecutive)
            && self.is_newer_than(next_consecutive, current_index)
        {
            return Some(next_consecutive);
        }

        self.ring_indices_from(oldest)
            .take_while(|&idx| idx != newest)
            .find(|&idx| self.is_newer_than(idx, current_index))
            .or_else(|| {
                // Check newest index last
                self.is_newer_than(newest, current_index).then_some(newest)
            })
    }

    /// Generate an iterator of valid indices starting from a given index in
    /// ring order
    fn ring_indices_from(&self, start: usize) -> impl Iterator<Item = usize> + '_ {
        (0..self.capacity)
            .map(move |offset| (start + offset) % self.capacity)
            .filter(|&idx| self.items.contains_key(&idx))
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
