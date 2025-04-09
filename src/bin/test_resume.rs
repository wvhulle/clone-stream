use forked_stream::{
    average_time_to_resume_and_receive, enable_debug_log, time_per_fork_to_receive_cold,
};

#[tokio::main]
async fn main() {
    enable_debug_log();
    let n_forks = 50;
    println!(
        "Measured: {:?}, estimated: {:?}",
        average_time_to_resume_and_receive(n_forks).await,
        time_per_fork_to_receive_cold(n_forks)
    );
}
