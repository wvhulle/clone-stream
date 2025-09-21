use core::time::Duration;

use clone_stream::CloneStream;
use futures::{StreamExt, future::join_all, join, stream};
use tokio::time::{Instant, sleep_until};

const N_STREAM_CLONES: usize = 50;

const N_ITEMS_SENT: usize = 50;

use criterion::{Criterion, criterion_group, criterion_main};

async fn mass_send() {
    let (sender, receiver) = futures::channel::mpsc::unbounded::<usize>();

    let template_clone: CloneStream<_> = receiver.into();

    let expect_numbers = (0..N_ITEMS_SENT).collect::<Vec<_>>();

    let start = Instant::now();

    let get_ready = start + std::time::Duration::from_millis(10) * N_STREAM_CLONES as u32;

    let wait_for_receive_all = join_all((0..N_STREAM_CLONES).map(|_i| async {
        sleep_until(get_ready).await;
        template_clone.clone().collect::<Vec<_>>().await
    }));

    let send = tokio::spawn(async move {
        sleep_until(get_ready + Duration::from_millis(10) * N_STREAM_CLONES as u32).await;
        stream::iter(0..N_ITEMS_SENT)
            .map(Ok)
            .forward(sender)
            .await
            .unwrap();
    });

    let (_, outputs) = join!(send, wait_for_receive_all);

    for (i, output) in outputs.into_iter().enumerate() {
        assert_eq!(
            output,
            expect_numbers,
            "Clone {} received unexpected items",
            i + 1
        );
    }
}

fn mass_send_benchmark(c: &mut Criterion) {
    c.bench_function("mass_send", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(mass_send);
    });
}

fn fast_criterion() -> Criterion {
    Criterion::default().sample_size(10)
}

criterion_group! {
    name = stress_tests;
    config = fast_criterion();
    targets = mass_send_benchmark
}
criterion_main!(stress_tests);
