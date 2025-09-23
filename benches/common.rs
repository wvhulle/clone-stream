#![allow(clippy::cast_precision_loss)]

use std::collections::HashMap;

pub const CLONE_COUNTS: &[usize] = &[1, 2, 4, 8];
pub const ITEM_COUNTS: &[usize] = &[10, 50, 100];

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct BenchmarkConfig {
    pub clones: usize,
    pub items: usize,
}

/// Performance statistics collector
#[derive(Default, Clone)]
pub struct PerformanceStats {
    pub results: HashMap<BenchmarkConfig, f64>, // (clones, items) -> avg_time_ns
}

impl PerformanceStats {
    #[must_use]
    pub fn with_result(mut self, clones: usize, items: usize, time_ns: f64) -> Self {
        self.results
            .insert(BenchmarkConfig { clones, items }, time_ns);
        self
    }

    #[must_use]
    pub fn generate_summary(&self) -> Vec<String> {
        let header = vec![
            "\n=== Performance Summary (clones x items) ===".to_string(),
            "Format: clones x items: avg_time | throughput".to_string(),
        ];

        let item_sections = ITEM_COUNTS
            .iter()
            .flat_map(|&items| {
                let section_header = format!("\n{items} items:");
                let clone_results = CLONE_COUNTS
                    .iter()
                    .filter_map(|&clones| {
                        self.results
                            .get(&BenchmarkConfig { clones, items })
                            .map(|&time_ns| {
                                let total_ops = clones * items;
                                let ops_per_sec = (total_ops as f64 * 1_000_000_000.0) / time_ns;
                                format!(
                                    "  {clones}x{items}: {:.2}μs | {:.1}K ops/sec",
                                    time_ns / 1000.0,
                                    ops_per_sec / 1000.0
                                )
                            })
                    })
                    .collect::<Vec<_>>();

                std::iter::once(section_header).chain(clone_results)
            })
            .collect::<Vec<_>>();

        let best_throughput = self
            .find_best_throughput()
            .map(|(clones, items)| format!("\nBest throughput: {clones}x{items} clonesxitems"))
            .into_iter()
            .collect::<Vec<_>>();

        header
            .into_iter()
            .chain(item_sections)
            .chain(best_throughput)
            .collect()
    }

    /// Find best throughput combination
    ///
    /// # Panics
    /// Panics if `partial_cmp` returns `None` when comparing throughput values.
    #[must_use]
    pub fn find_best_throughput(&self) -> Option<(usize, usize)> {
        self.results
            .iter()
            .max_by(|a, b| {
                let throughput_a = (a.0.clones * a.0.items) as f64 / a.1;
                let throughput_b = (b.0.clones * b.0.items) as f64 / b.1;
                throughput_a.partial_cmp(&throughput_b).unwrap()
            })
            .map(|(BenchmarkConfig { clones, items }, _)| (*clones, *items))
    }

    pub fn print_summary(&self) {
        self.generate_summary()
            .into_iter()
            .for_each(|combination_line| println!("{combination_line}"));
    }
}

/// Generate all benchmark configurations
pub fn benchmark_configurations() -> impl Iterator<Item = BenchmarkConfig> {
    CLONE_COUNTS.iter().flat_map(|&clones| {
        ITEM_COUNTS
            .iter()
            .map(move |&items| BenchmarkConfig { clones, items })
    })
}

#[must_use]
pub const fn test_items(size: usize) -> std::ops::Range<usize> {
    0..size
}

/// Functional pipeline trait
pub trait Pipe: Sized {
    fn pipe<F, T>(self, f: F) -> T
    where
        F: FnOnce(Self) -> T,
    {
        f(self)
    }
}

impl<T> Pipe for T {}
