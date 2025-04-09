use std::time::Duration;

use futures::{SinkExt, StreamExt, channel::mpsc, future::try_join_all};
use log::info;
use tokio::time::{Instant, sleep_until, timeout};

use super::test_setup::Metrics;
use crate::{ForkAsyncMockSetup, ForkStream, TOKIO_TASK_STARTUP, TimeRange, fork_warmup, warmup};

fn warm_up(n: usize, factor: f32) -> Duration {
    let n = f32::from(u16::try_from(n).unwrap());
    TOKIO_TASK_STARTUP.mul_f32(n * factor)
}

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
    println!("Testing resume time with {n_forks} forks");
    let (mut tx, rx) = mpsc::unbounded::<()>();
    let fork = rx.fork();
    let (lat_tx, mut lat_rx) = mpsc::unbounded();
    let now = Instant::now();

    let send_instant = now + fork_warmup(n_forks);

    println!("Forking stream");
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

    println!("Waiting to send");
    sleep_until(send_instant).await;

    info!("Sending item");
    let _ = tx.send(()).await;

    let mut latencies = Vec::new();

    println!("Waiting for the forks to send their latencies.");
    while let Some((lat, i)) = lat_rx.next().await {
        info!("Fork {i} lat: {lat:?}");
        latencies.push(lat);
        if latencies.len() == n_forks {
            break;
        }
    }

    println!("Waiting for all forks to finish");

    latencies
        .iter()
        .sum::<Duration>()
        .mul_f64(1.0 / n_forks as f64)
}

pub async fn spacing_wide_enough(n_forks: usize, duration: Duration) -> bool {
    println!("Testing with {n_forks} forks and spaced apart {duration:?}");
    let sub_poll_time_ranges =
        TimeRange::sequential_overlapping_sub_ranges_from(warmup(n_forks), n_forks, duration);

    let last = sub_poll_time_ranges.last().unwrap();

    let mut setup = ForkAsyncMockSetup::<1>::new(n_forks);

    let mut sender = setup.sender.clone();

    let metrics = setup
        .launch(|i| sub_poll_time_ranges[i], async move {
            info!("Waiting to send until the middle of the send phase");
            sleep_until(last.middle()).await;

            info!("Sending item");
            let _ = sender.send(0).await;
        })
        .await;

    metrics.is_successful()
}
