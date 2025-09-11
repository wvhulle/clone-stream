use std::{
    pin::Pin,
    task::{Context, Poll},
    sync::atomic::{AtomicUsize, Ordering},
};

use clone_stream::ForkStream;
use futures::{Stream, StreamExt};

struct TemporarilyExhaustedStream {
    counter: AtomicUsize,
}

impl TemporarilyExhaustedStream {
    fn new() -> Self {
        Self {
            counter: AtomicUsize::new(0),
        }
    }
}

impl Stream for TemporarilyExhaustedStream {
    type Item = i32;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let count = self.counter.fetch_add(1, Ordering::Relaxed);
        match count {
            0 => Poll::Ready(Some(1)),
            2 => Poll::Ready(Some(2)),
            _ => Poll::Ready(None), // Covers both temporarily exhausted (1) and permanently exhausted (3+)
        }
    }
}

#[tokio::test]
async fn temporarily_exhausted_stream_handling() {
    let stream = TemporarilyExhaustedStream::new();
    let mut clone1 = stream.fork();
    
    assert_eq!(clone1.next().await, Some(1));
    assert_eq!(clone1.next().await, None);
    assert_eq!(clone1.next().await, Some(2));
    assert_eq!(clone1.next().await, None);
    
    let mut clone2 = clone1.clone();
    assert_eq!(clone2.next().await, None);
}

#[tokio::test]
async fn clone_after_advancement() {
    let stream = TemporarilyExhaustedStream::new();
    let mut clone1 = stream.fork();
    
    assert_eq!(clone1.next().await, Some(1));
    assert_eq!(clone1.next().await, None);
    assert_eq!(clone1.next().await, Some(2));
    
    let mut clone2 = clone1.clone();
    assert_eq!(clone2.next().await, None);
}

#[tokio::test]
async fn concurrent_clone_polling() {
    let stream = TemporarilyExhaustedStream::new();
    let mut clone1 = stream.fork();
    let mut clone2 = clone1.clone();
    
    assert_eq!(clone1.next().await, Some(1));
    assert_eq!(clone2.next().await, None);
    assert_eq!(clone1.next().await, Some(2));
    assert_eq!(clone2.next().await, None);
}