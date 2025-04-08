use std::{collections::VecDeque, num::NonZero, time::Duration};

use tokio::time::Instant;

#[derive(Copy, Clone)]
pub struct TimeRange {
    pub start: Instant,
    pub end: Instant,
}

impl TimeRange {
    /// # Panics
    ///
    /// Panics when `n` is 0.
    #[must_use]
    pub fn split(&self, n: u32, gap_ratio: f64) -> Vec<TimeRange> {
        assert!(
            (0.0..=1.0).contains(&gap_ratio),
            "Gap ratio must be between 0 and 1"
        );

        let total = self.duration();

        if n == 0 {
            panic!("Cannot split into 0 ranges");
        } else if n == 1 {
            return vec![*self];
        } else {
            let n_ranges = f64::from(n);
            let n_gaps = f64::from(n - 1);
            let duration_per_range = total.mul_f64((1.0 - gap_ratio) / n_ranges);
            let gap_width = (total - duration_per_range.mul_f64(n_ranges)).mul_f64(1.0 / n_gaps);
            (0..n)
                .map(|i| TimeRange {
                    start: self.start + (duration_per_range + gap_width) * i,
                    end: self.start + duration_per_range * (i + 1) + gap_width * i,
                })
                .collect()
        }
    }

    #[must_use]
    pub fn inner(&self, padding: f64) -> Self {
        let total_duration = self.end.duration_since(self.start);
        let padding_duration = total_duration.mul_f64(padding);
        TimeRange {
            start: self.start + padding_duration / 2,
            end: self.end - padding_duration / 2,
        }
    }

    #[must_use]
    pub fn within(&self, other: &Self) -> bool {
        self.start >= other.start && self.end <= other.end
    }

    /// # Panics
    ///
    ///
    /// Panics when `n` is 0.
    #[must_use]
    pub fn moments(&self, n: usize) -> Vec<Instant> {
        match n {
            0 => panic!("Cannot split range into 0 moments"),
            1 => {
                assert_eq!(
                    self.start, self.end,
                    "Can split range into 1 moment only if start and end are equal"
                );
                vec![self.start]
            }
            2 => vec![self.start, self.end],
            n => {
                let n = n.try_into().unwrap();
                let total_duration = self.end.duration_since(self.start);
                let range_duration = total_duration / n;
                let mut intermediate_moments: VecDeque<_> = (1..(n - 1))
                    .map(|i| self.start + range_duration * i)
                    .collect();

                intermediate_moments.push_front(self.start);
                intermediate_moments.push_back(self.end);
                intermediate_moments.into()
            }
        }
    }

    #[must_use]
    pub fn sequential_overlapping_sub_ranges(&self, n: usize) -> Vec<Self> {
        let start_end = self.split(2, 0.0);

        let start_moments = start_end[0].inner(0.1).moments(n);
        let end = start_end[1].middle();
        start_moments
            .iter()
            .map(|start| Self { start: *start, end })
            .collect()
    }

    #[must_use]
    pub fn middle(&self) -> Instant {
        let total_duration = self.end.duration_since(self.start);
        self.start + total_duration / 2
    }

    #[must_use]
    pub fn duration(&self) -> Duration {
        self.end.duration_since(self.start)
    }
}

impl From<Duration> for TimeRange {
    fn from(duration: Duration) -> Self {
        TimeRange {
            start: Instant::now(),
            end: Instant::now() + duration,
        }
    }
}
