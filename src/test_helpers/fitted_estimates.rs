use std::time::Duration;

pub const LOAD_TOKIO_TASK: Duration = Duration::from_micros(1000);

#[must_use]
pub fn tokio_task_load_time(n_tokio_tasks: usize) -> Duration {
    let n_tokio_tasks = n_tokio_tasks as f32;
    let n_mul_factor = if n_tokio_tasks < 12.0 {
        0.07
    } else if n_tokio_tasks < 300.0 {
        0.11
    } else if n_tokio_tasks < 530.0 {
        0.21
    } else if n_tokio_tasks < 810.0 {
        0.31
    } else {
        0.41
    };

    LOAD_TOKIO_TASK.mul_f32(2.0 * n_mul_factor)
}

#[must_use]
pub fn fork_warmup(n_forks: usize) -> Duration {
    tokio_task_load_time(n_forks).mul_f32(5.0)
}

#[must_use]
pub fn time_per_fork_to_receive_cold(n_fokrs: usize) -> Duration {
    let n_forks = n_fokrs as f32;
    let n_mul_factor = if n_forks < 10.0 {
        0.15
    } else if n_forks < 50.0 {
        0.06
    } else {
        0.2
    };
    LOAD_TOKIO_TASK.mul_f32(n_forks * n_mul_factor)
}

#[must_use]
pub fn min_spacing_seq_polled_forks(n_forks: usize) -> Duration {
    let n_forks = n_forks as f32;

    let n_mul_factor = if n_forks < 10.0 {
        0.4
    } else if n_forks < 50.0 {
        0.2
    } else if n_forks < 100.0 {
        0.1
    } else if n_forks < 200.0 {
        0.06
    } else {
        0.055
    };

    LOAD_TOKIO_TASK.mul_f32(n_forks * n_mul_factor)
}
