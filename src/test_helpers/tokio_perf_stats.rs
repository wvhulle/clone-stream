use std::time::Duration;

use futures::{SinkExt, StreamExt, channel::mpsc, future::try_join_all};
use log::info;
use tokio::time::Instant;

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
