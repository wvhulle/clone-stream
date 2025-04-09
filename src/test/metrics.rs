use std::time::Duration;

use futures::{SinkExt, StreamExt, channel::mpsc, future::try_join_all};
use log::info;
use tokio::time::{Instant, sleep_until, timeout};

use super::test_setup::FinalState;
use crate::{
    ForkStream, TOKIO_TASK_STARTUP, TestSetup, TimeRange, time_fork_needs_to_wake_and_receive,
    time_start_tokio_task,
};

pub async fn average_warmup(n_forks: usize) -> Duration {
    let (lat_tx, mut lat_rx) = mpsc::unbounded();
    let now = Instant::now();
    let task = try_join_all((0..n_forks).map(|i| {
        let mut lat_tx = lat_tx.clone();
        tokio::spawn(async move {
            lat_tx.send((now.elapsed(), i)).await.unwrap();
        })
    }));

    let mut latencies = Vec::new();
    while let Some((lat, i)) = lat_rx.next().await {
        info!("Fork {i} lat: {lat:?}");
        latencies.push(lat);
        if latencies.len() == n_forks {
            break;
        }
    }

    task.await.unwrap();

    latencies
        .iter()
        .sum::<Duration>()
        .mul_f64(1.0 / n_forks as f64)
}

pub async fn average_time_to_resume_and_receive(n_forks: usize) -> Duration {
    let (mut tx, rx) = mpsc::unbounded::<()>();
    let fork = rx.fork();
    let (lat_tx, mut lat_rx) = mpsc::unbounded();
    let now = Instant::now();

    let send_instant = now + time_fork_needs_to_wake_and_receive(n_forks);

    let task = try_join_all((0..n_forks).map(|i| {
        let mut lat_tx = lat_tx.clone();
        let mut fork = fork.clone();
        tokio::spawn(async move {
            assert!(
                send_instant > Instant::now(),
                "Fork {} woke up {:?} too late",
                i,
                Instant::now().duration_since(send_instant)
            );
            fork.next().await;
            lat_tx.send((send_instant.elapsed(), i)).await.unwrap();
        })
    }));
    drop(task);

    sleep_until(send_instant).await;

    let _ = tx.send(()).await;

    let mut latencies = Vec::new();

    while let Some((lat, i)) = lat_rx.next().await {
        info!("Fork {i} lat: {lat:?}");
        latencies.push(lat);
        if latencies.len() == n_forks {
            break;
        }
    }

    latencies
        .iter()
        .sum::<Duration>()
        .mul_f64(1.0 / n_forks as f64)
}

pub async fn spacing_wide_enough(n_forks: usize, duration: Duration) -> bool {
    let sub_poll_time_ranges =
        TimeRange::consecutive(time_start_tokio_task(n_forks), n_forks, duration);

    let last = sub_poll_time_ranges.last().unwrap();

    let mut setup: TestSetup = TestSetup::new(n_forks);

    let mut sender = setup.sender.clone();

    let final_state = setup
        .launch(|i| sub_poll_time_ranges[i], async move {
            info!("Waiting to send until the middle of the send phase");
            sleep_until(last.middle()).await;

            info!("Sending item");
            let _ = sender.send(0).await;
        })
        .await;

    final_state.success()
}
