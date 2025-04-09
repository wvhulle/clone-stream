use std::time::Duration;

use forked_stream::{
    TOKIO_TASK_STARTUP, average_warmup, find_average_min, floats_from_to, ints_from_to,
};

fn warmup(n: usize, factor: f32) -> Duration {
    let n = f32::from(u16::try_from(n).unwrap());

    TOKIO_TASK_STARTUP.mul_f32(n * factor)
}

#[tokio::main]
async fn main() {
    let results = find_average_min(
        |n_forks, factor| async move { average_warmup(n_forks).await < warmup(n_forks, factor) },
        ints_from_to(2, 1000, 30),
        floats_from_to(0.001, 0.02, 0.01),
        20,
    )
    .await;

    println!("Results: {results:?}");
}
