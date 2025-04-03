use std::time::Duration;

use tokio::time::Instant;

#[derive(Copy, Clone)]
pub struct TimeRange {
    pub start: Instant,
    pub end: Instant,
}

impl TimeRange {
    pub fn until(duration: Duration) -> Self {
        let start = Instant::now();
        let end = start + duration;
        Self { start, end }
    }

    pub fn split_into_consecutive_ranges(&self, n: u32) -> Vec<TimeRange> {
        let total_duration = self.end.duration_since(self.start);
        let duration_per_range = total_duration / n;
        (0..n)
            .map(|i| TimeRange {
                start: self.start + duration_per_range * i,
                end: self.start + duration_per_range * (i + 1),
            })
            .collect()
    }

    pub fn split_into_consecutive_range_with_gaps(&self, n: u32, gap: Duration) -> Vec<TimeRange> {
        let total_duration = self.end.duration_since(self.start);
        let duration_per_range = (total_duration - gap * (n - 1)) / n;
        (0..n)
            .map(|i| TimeRange {
                start: self.start + duration_per_range * i + gap * i,
                end: self.start + duration_per_range * (i + 1) + gap * i,
            })
            .collect()
    }

    pub fn split_into_consecutive_instants(&self, n: u32) -> Vec<Instant> {
        let total_duration = self.end.duration_since(self.start);
        let duration_per_range = total_duration / n;
        (0..n)
            .map(|i| self.start + duration_per_range * i)
            .collect()
    }

    pub fn middle(&self) -> Instant {
        let total_duration = self.end.duration_since(self.start);
        self.start + total_duration / 2
    }

    pub fn duration(&self) -> Duration {
        self.end.duration_since(self.start)
    }
}
