use std::time::Duration;

use forked_stream::{TOKIO_TASK_STARTUP, average_warmup, enable_debug_log as enable_debug_log};

fn estimated_warmup(n: usize) -> Duration {
    let n = f32::from(u16::try_from(n).unwrap());

    let factor = if n < 12.0 {
        0.07
    } else if n < 300.0 {
        0.11
    } else if n < 530.0 {
        0.21
    } else if n < 810.0 {
        0.31
    } else {
        0.41
    };

    TOKIO_TASK_STARTUP.mul_f32(3.0 * factor)
}

#[tokio::main]
async fn main() {
    enable_debug_log();
    let n_forks = 10;

    println!(
        "Measured: {:?} vs. estimated: {:?}",
        average_warmup(n_forks).await,
        estimated_warmup(n_forks)
    );
}
