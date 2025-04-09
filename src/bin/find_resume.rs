use forked_stream::{
    TOKIO_TASK_STARTUP, average_time_to_resume_and_receive, find_average_min, floats_from_to,
    ints_from_to,
};

#[tokio::main]
async fn main() {
    let results = find_average_min(
        |n_forks, factor| async move {
            average_time_to_resume_and_receive(n_forks).await
                < TOKIO_TASK_STARTUP.mul_f32(n_forks as f32 * factor)
        },
        ints_from_to(2, 100, 10),
        floats_from_to(0.02, 0.2, 0.01),
        10,
    )
    .await;

    println!("Results: {results:?}");
}
