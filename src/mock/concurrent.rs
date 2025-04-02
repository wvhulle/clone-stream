#![allow(unused_imports)]

use std::{task::Poll, time::Duration};

use futures::{FutureExt, Stream, StreamExt};
use log::info;
use tokio::{task::JoinHandle, time::timeout};

use super::{single_task::new_sender_and_shared_stream, test_log::log_init};
use crate::{ForkedStream, SpscSender};

pub struct ConcurrentSetup<BaseStream>
where
    BaseStream: Stream<Item = i32>,
{
    pub input_sink: SpscSender<i32>,
    forked_stream: ForkedStream<BaseStream>,
}

impl<BaseStream> ConcurrentSetup<BaseStream>
where
    BaseStream: Stream<Item = i32> + Send + 'static,
{
    pub fn current_next(&mut self) -> Poll<Option<i32>> {
        match self.forked_stream.next().now_or_never() {
            Some(value) => Poll::Ready(value),
            None => Poll::Pending,
        }
    }

    pub fn expect_background(&self, value: Option<i32>, deadline: Duration) -> JoinHandle<()> {
        let mut background_stream = self.forked_stream.clone();
        info!("Spawning a background task to await next of cloned stream.",);
        tokio::spawn(async move {
            info!("Waiting for the next method on the background stream to resolve.",);
            let item = timeout(deadline, background_stream.next())
                .await
                .expect("Timed out in background.");
            assert_eq!(item, value, "Background task received an unexpected value.");
        })
    }
}

pub fn new_concurrent_setup() -> ConcurrentSetup<impl Stream<Item = i32>> {
    log_init();
    let (input, forked_stream) = new_sender_and_shared_stream();

    ConcurrentSetup {
        input_sink: input,
        forked_stream,
    }
}
