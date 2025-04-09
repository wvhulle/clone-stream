use std::{ops::Deref, sync::atomic::AtomicUsize, task::Context};

use futures::task::{RawWaker, RawWakerVTable, Waker};
// Define a simple raw waker implementation that does nothing

fn raw_waker(data: *const ()) -> RawWaker {
    RawWaker::new(data, &VTABLE)
}
const VTABLE: RawWakerVTable = RawWakerVTable::new(
    raw_waker, // clone
    |_| {},    // wake
    |_| {},    // wake_by_ref
    |_| {},    // drop
);

static WAKER_ID: AtomicUsize = AtomicUsize::new(0);

pub struct MockWaker(Waker);

impl MockWaker {
    pub fn new() -> Self {
        let uniq_heap_alloc = Box::new(WAKER_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst));
        let raw_ptr = Box::into_raw(uniq_heap_alloc) as *const ();
        Self(unsafe { Waker::from_raw(raw_waker(raw_ptr)) })
    }

    pub fn context(&self) -> Context<'_> {
        Context::from_waker(self)
    }
}

impl Default for MockWaker {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MockWaker {
    fn drop(&mut self) {
        unsafe {
            drop(Box::from_raw(self.0.data() as *mut usize));
        };
    }
}

impl Deref for MockWaker {
    type Target = Waker;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[test]
fn two_wakers_data_different() {
    let waker1 = MockWaker::new();
    let waker2 = MockWaker::new();
    assert!(!std::ptr::eq(waker1.data(), waker2.data()));
}

#[test]
fn two_wakers_wake_different() {
    let waker1 = MockWaker::new();
    let waker2 = MockWaker::new();
    assert!(!std::ptr::eq(waker1.data(), waker2.data()));
}
