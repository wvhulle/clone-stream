use std::time::Duration;

use forked_stream::{TOKIO_TASK_STARTUP, average_time_to_resume_and_receive};

fn estimated_resume(n: usize) -> Duration {
    let n = f32::from(u16::try_from(n).unwrap());
    let factor = if n < 10.0 {
        0.15
    } else if n < 50.0 {
        0.06
    } else {
        0.02
    };
    TOKIO_TASK_STARTUP.mul_f32(n * factor)
}

#[tokio::main]
async fn main() {
    println!("Testing resume time...");
    let n_forks = 50;
    println!(
        "Measured: {:?}, estimated: {:?}",
        average_time_to_resume_and_receive(n_forks).await,
        estimated_resume(n_forks)
    );
}
