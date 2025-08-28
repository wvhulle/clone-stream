use std::time::Instant;

use clone_stream::ForkStream;
use futures::{StreamExt, future::join_all, stream};

#[tokio::main]
async fn main() {
    println!("=== Rust Clone Stream Performance ===");

    // Test 1: Single clone, 1000 items
    print!("Single clone, 1000 items: ");
    let start = Instant::now();

    let stream = stream::iter(0..1000);
    let items: Vec<_> = stream.fork().collect().await;

    let duration = start.elapsed();
    println!("{:?} ({} items)", duration, items.len());

    // Test 2: 10 clones, 100 items each
    print!("10 clones, 100 items each: ");
    let start = Instant::now();

    let stream = stream::iter(0..100);
    let template = stream.fork();

    let tasks: Vec<_> = (0..10)
        .map(|_| {
            let clone = template.clone();
            tokio::spawn(async move { clone.collect::<Vec<_>>().await })
        })
        .collect();

    let results = join_all(tasks).await;
    let lengths: Vec<_> = results.into_iter().map(|r| r.unwrap().len()).collect();
    let duration = start.elapsed();
    println!("{duration:?} ({lengths:?})");

    // Test 3: 100 clones, 100 items each
    print!("100 clones, 100 items each: ");
    let start = Instant::now();

    let stream = stream::iter(0..100);
    let template = stream.fork();

    let tasks: Vec<_> = (0..100)
        .map(|_| {
            let clone = template.clone();
            tokio::spawn(async move { clone.collect::<Vec<_>>().await.len() })
        })
        .collect();

    let results = join_all(tasks).await;
    let lengths: Vec<_> = results.into_iter().map(|r| r.unwrap()).collect();
    let all_correct = lengths.iter().all(|&len| len == 100);
    let duration = start.elapsed();
    println!("{duration:?} (all correct: {all_correct})");

    // Test 4: Late clone test (simplified)
    print!("Late clone creation test: ");
    let start = Instant::now();

    let (tx, rx) = futures::channel::mpsc::unbounded::<i32>();
    let mut original = rx.fork();

    // Send first batch
    for i in 0..50 {
        tx.unbounded_send(i).unwrap();
    }

    // Read some from original
    let mut early_items = Vec::new();
    for _ in 0..25 {
        if let Some(item) = original.next().await {
            early_items.push(item);
        }
    }

    // Create late clone
    let late_clone = original.clone();

    // Send second batch
    for i in 50..100 {
        tx.unbounded_send(i).unwrap();
    }
    drop(tx);

    // Collect remaining items
    let remaining: Vec<_> = original.collect().await;
    let late_items: Vec<_> = late_clone.collect().await;

    let duration = start.elapsed();
    println!(
        "{:?} (early: {}, remaining: {}, late: {})",
        duration,
        early_items.len(),
        remaining.len(),
        late_items.len()
    );
}
