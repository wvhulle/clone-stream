use std::time::Duration;

use forked_stream::{TOKIO_TASK_STARTUP, spacing_wide_enough};

fn estimated_spacing(n: usize) -> Duration {
    let n = f32::from(u16::try_from(n).unwrap());

    let factor = if n < 10.0 {
        0.4
    } else if n < 50.0 {
        0.2
    } else if n < 100.0 {
        0.1
    } else if n < 200.0 {
        0.06
    } else {
        0.055
    };

    TOKIO_TASK_STARTUP.mul_f32(n * factor)
}

#[tokio::main]
async fn main() {
    let n_forks = 200;
    let results = spacing_wide_enough(n_forks, estimated_spacing(n_forks)).await;

    println!("Results: {results:?}");
}
