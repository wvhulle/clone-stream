use forked_stream::{
    LOAD_TOKIO_TASK, average_warmup, enable_debug_log, find_average_min, floats_from_to,
    ints_from_to,
};

#[tokio::main]
async fn main() {
    enable_debug_log();
    let results = find_average_min(
        |n_forks, samp_mul_fact| async move {
            average_warmup(n_forks).await < LOAD_TOKIO_TASK.mul_f32(n_forks as f32 * samp_mul_fact)
        },
        ints_from_to(2, 1000, 30),
        floats_from_to(0.001, 0.02, 0.01),
        20,
    )
    .await;

    println!("Results: {results:?}");
}
