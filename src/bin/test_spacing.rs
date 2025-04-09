use forked_stream::{enable_debug_log, min_spacing_seq_polled_forks, spacing_wide_enough};

#[tokio::main]
async fn main() {
    enable_debug_log();
    let n_forks = 200;
    let results = spacing_wide_enough(n_forks, min_spacing_seq_polled_forks(n_forks)).await;

    println!("Results: {results:?}");
}
