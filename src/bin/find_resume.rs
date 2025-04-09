use forked_stream::{
    LOAD_TOKIO_TASK, average_time_to_resume_and_receive, enable_debug_log, find_average_min,
    floats_from_to, ints_from_to,
};

#[tokio::main]
async fn main() {
    enable_debug_log();

    let results = find_average_min(
        |n_forks, factor| async move {
            average_time_to_resume_and_receive(n_forks).await
                < LOAD_TOKIO_TASK.mul_f32(n_forks as f32 * factor)
        },
        ints_from_to(2, 100, 10),
        floats_from_to(0.02, 0.2, 0.01),
        10,
    )
    .await;

    println!("Results: {results:?}");
}
