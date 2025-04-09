use std::time::Duration;

pub const TOKIO_TASK_STARTUP: Duration = Duration::from_micros(1000);

#[must_use]
pub fn warmup(n: usize) -> Duration {
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
pub fn fork_warmup(n: usize) -> Duration {
    warmup(n).mul_f32(5.0)
}

#[must_use]
pub fn resume(n: usize) -> Duration {
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
pub fn resume_forks(n: usize) -> Duration {
    resume(n).mul_f32(5.0)
}

#[must_use]
pub fn wake_up_time(n: usize) -> Duration {
    let n = f32::from(u16::try_from(n).unwrap());

    let factor = 3.0;

    TOKIO_TASK_STARTUP.mul_f32(n * factor)
}

#[must_use]
pub fn spacing_required(n: usize) -> Duration {
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
