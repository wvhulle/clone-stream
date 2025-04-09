use forked_stream::{average_warmup, enable_debug_log, tokio_task_load_time};

#[tokio::main]
async fn main() {
    enable_debug_log();
    let n_forks = 10;

    println!(
        "Measured: {:?} vs. estimated: {:?}",
        average_warmup(n_forks).await,
        tokio_task_load_time(n_forks)
    );
}
