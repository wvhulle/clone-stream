use std::time::Duration;

use tokio::time::Instant;

#[derive(Copy, Clone)]
pub struct TimeRange {
    pub start: Instant,
    pub end: Instant,
}

impl TimeRange {
    pub fn split(&self, n: u32, gap_ratio: f64) -> Vec<TimeRange> {
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

    pub fn moments(&self, n: u32) -> Vec<Instant> {
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

impl From<Duration> for TimeRange {
    fn from(duration: Duration) -> Self {
        TimeRange {
            start: Instant::now(),
            end: Instant::now() + duration,
        }
    }
}
