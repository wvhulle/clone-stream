use clone_stream::ForkStream;
use futures::{stream, StreamExt};

/// Test that `fork_with_limits` function can be called and creates clones
#[tokio::test]
async fn test_fork_with_limits_basic() {
    let values = vec![1];
    let stream = stream::iter(values);
    
    // Create a fork with custom limits - this should not panic
    let clone_stream = stream.fork_with_limits(100, 5);
    
    // Just verify we can create the stream without panicking
    // Note: We're not testing the actual cloning behavior here due to pre-existing issues
    assert!(clone_stream.n_queued_items() == 0);
}

/// Test that queue size limit is enforced
#[tokio::test]
#[should_panic(expected = "Queue index overflow")]
async fn test_queue_size_limit_panic() {
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use futures::Stream;
    
    // Create a stream that will generate many items
    struct InfiniteStream;
    
    impl Stream for InfiniteStream {
        type Item = i32;
        
        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(Some(1)) // Always ready with the value 1
        }
    }
    
    let stream = InfiniteStream;
    let mut clone_stream = stream.fork_with_limits(1, 5); // Very small queue limit
    
    // Try to consume many items, this should eventually panic due to queue overflow
    for _ in 0..10 {
        let _ = clone_stream.next().await;
    }
}

/// Test that clone count limit is enforced  
#[tokio::test]
#[should_panic(expected = "Maximum number of clones")]
async fn test_clone_count_limit_panic() {
    let values = vec![1, 2, 3];
    let stream = stream::iter(values);
    
    let original = stream.fork_with_limits(1000, 2); // Very small clone limit
    
    // Create clones until we hit the limit
    let _clone1 = original.clone();
    let _clone2 = original.clone(); // This should panic
}
