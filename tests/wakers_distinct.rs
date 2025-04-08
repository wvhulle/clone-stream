mod mock;

use mock::MockWaker;

#[test]
fn two_wakers_data_different() {
    let waker1 = MockWaker::new();
    let waker2 = MockWaker::new();
    assert!(waker1.data() != waker2.data());
}

#[test]
fn two_wakers_wake_different() {
    let waker1 = MockWaker::new();
    let waker2 = MockWaker::new();
    assert!(waker1.data() != waker2.data());
}
