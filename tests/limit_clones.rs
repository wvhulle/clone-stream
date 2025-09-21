use clone_stream::ForkStream;
use futures::stream;

/// Test that clone count limit is enforced by panicking
#[tokio::test]
#[should_panic(expected = "Failed to register clone - clone limit exceeded")]
async fn test_clone_count_limit_error() {
    let values = vec![1, 2, 3];
    let stream = stream::iter(values);

    let original = stream.fork_with_limits(1000, 2); // Very small clone limit

    // Create clones until we hit the limit
    let _clone1 = original.clone();
    let _clone2 = original.clone();

    // The third clone should panic
    let _clone3 = original.clone(); // This will panic
}
