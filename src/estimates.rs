use std::time::Duration;

pub const TOKIO_TASK_STARTUP: Duration = Duration::from_micros(1000);

#[must_use]
pub fn time_start_tokio_task(n: usize) -> Duration {
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

    TOKIO_TASK_STARTUP.mul_f32(2.0 * factor)
}

#[must_use]
pub fn time_fork_needs_to_wake_and_receive(n: usize) -> Duration {
    time_start_tokio_task(n).mul_f32(5.0)
}

#[must_use]
pub fn time_to_receive_after_send(n: usize) -> Duration {
    let n = f32::from(u16::try_from(n).unwrap());
    let factor = if n < 10.0 {
        0.15
    } else if n < 50.0 {
        0.06
    } else {
        0.2
    };
    TOKIO_TASK_STARTUP.mul_f32(n * factor)
}

#[must_use]
pub fn time_to_receive_on_fork_after_input(n: usize) -> Duration {
    time_to_receive_after_send(n).mul_f32(5.0)
}

#[must_use]
pub fn space_required_consecutive_fork_polls(n: usize) -> Duration {
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
