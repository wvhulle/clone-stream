use forked_stream::{
    LOAD_TOKIO_TASK, enable_debug_log, find_average_min, floats_from_to, ints_from_to,
    spacing_wide_enough,
};

#[tokio::main]
async fn main() {
    enable_debug_log();

    let results = find_average_min(
        |n_forks, factor| async move {
            spacing_wide_enough(n_forks, LOAD_TOKIO_TASK.mul_f32(n_forks as f32 * factor)).await
        },
        ints_from_to(2, 200, 50),
        floats_from_to(0.05, 0.4, 0.1),
        10,
    )
    .await;

    println!("Results: {results:#?}");
}
