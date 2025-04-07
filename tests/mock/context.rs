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

pub struct MockWaker(Waker);

impl MockWaker {
    pub fn new(count: usize) -> Self {
        let u = Box::new(count);
        let ptr = Box::into_raw(u) as *const ();
        Self(unsafe { Waker::from_raw(raw_waker(ptr)) })
    }

    pub fn context(&self) -> Context<'_> {
        Context::from_waker(self)
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
