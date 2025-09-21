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
            self.newest = Some((newest + 1) % self.capacity);
            if self.items.contains_key(&newest) {
                self.items.remove(&newest);
            }
            self.items.insert(newest, item);
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
                .as_ref()
                .map(|(k, _)| *k)
                .copied();
        }
        if Some(index) == self.newest {
            // Update newest to the previous item
            self.newest = self
                .items
                .range((index + 1)..self.capacity)
                .chain(self.items.range(0..index))
                .next_back()
                .as_ref()
                .map(|(k, _)| *k)
                .copied();
        }
        removed
    }

    /// Gets an item at the given index.
    pub(crate) fn get(&self, index: usize) -> Option<&T> {
        self.items.get(&index)
    }

    /// Checks if `index_a` comes before `index_b` in ring buffer order.
    /// This accounts for wraparound: if the indices are on different sides
    /// of the wraparound point, we need special logic.
    pub(crate) fn is_before(&self, index_a: usize, index_b: usize) -> bool {
        if index_a == index_b {
            return false;
        }

        // Calculate the distance from index_a to index_b in ring buffer terms
        let forward_distance = if index_b >= index_a {
            index_b - index_a
        } else {
            (self.capacity - index_a) + index_b
        };

        // If forward distance is less than half the capacity, then index_a comes before index_b
        forward_distance <= self.capacity / 2
    }

    /// Checks if `index_a` comes after `index_b` in ring buffer order.
    pub(crate) fn is_after(&self, index_a: usize, index_b: usize) -> bool {
        index_a != index_b && !self.is_before(index_a, index_b)
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

            // Move to next index in ring buffer order
            self.current = if self.remaining > 0 {
                Some((current_idx + 1) % self.queue.capacity)
            } else {
                None
            };

            return Some((current_idx, item));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_ordering() {
        let queue: RingQueue<i32> = RingQueue::new(10);

        // Test basic ordering
        assert!(queue.is_before(0, 1));
        assert!(queue.is_before(1, 2));
        assert!(!queue.is_before(2, 1));

        // Test wraparound cases
        assert!(queue.is_before(8, 9));
        assert!(queue.is_before(9, 0)); // 9 wraps to 0
        assert!(queue.is_before(9, 1)); // 9 wraps past 0 to 1
        assert!(!queue.is_before(0, 9)); // 0 doesn't come before 9 when 9 wraps
        assert!(!queue.is_before(1, 9)); // 1 doesn't come before 9 when 9 wraps

        // Test middle cases
        assert!(queue.is_before(8, 2)); // 8 -> 9 -> 0 -> 1 -> 2
        assert!(!queue.is_before(2, 8)); // 2 -> 3 -> ... -> 8 (longer path)
    }

    // #[test]
    // fn test_allocation_and_wraparound() {
    //     let mut queue: RingQueue<i32> = RingQueue::new(3);

    //     // Insert items to fill capacity
    //     queue.insert(10);
    //     queue.insert(20);
    //     queue.insert(30);
    //     assert_eq!(queue.len(), 3);

    //     // Should wrap around and overwrite when at capacity
    //     // assert_eq!(queue.allocate_index(), 0); // wraps back to 0
    //     queue.insert(40); // overwrites the old value

    //     // Check that old item was overwritten
    //     assert_eq!(queue.get(0), Some(&40));
    //     assert_eq!(queue.len(), 3); // size should stay the same
    // }
}
