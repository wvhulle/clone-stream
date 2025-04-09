// A stream that implements `Clone` and takes input from the `BaseStream`i
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock},
};

use futures::Stream;
use log::warn;

use crate::bridge::{Bridge, ForkRef};

pub struct Fork<BaseStream>(Arc<RwLock<Bridge<BaseStream>>>)
where
    BaseStream: Stream<Item: Clone>;
impl<BaseStream> Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    #[must_use]
    pub fn new(mut bridge: Bridge<BaseStream>) -> Self {
        bridge.forks.clear();
        bridge.forks.insert(0, ForkRef::default());
        Self(Arc::new(RwLock::new(bridge)))
    }

    pub fn get<R>(&self, get: impl FnOnce(&Bridge<BaseStream>) -> R) -> R {
        match self.read() {
            Ok(bridge) => get(&bridge),
            Err(e) => {
                warn!("The previous task who locked the bridge to read it panicked");

                get(e.get_ref())
            }
        }
    }
}

impl<BaseStream> From<Bridge<BaseStream>> for Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn from(bridge: Bridge<BaseStream>) -> Self {
        Self::new(bridge)
    }
}

impl<BaseStream> Clone for Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<BaseStream> Deref for Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    type Target = Arc<RwLock<Bridge<BaseStream>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<BaseStream> DerefMut for Fork<BaseStream>
where
    BaseStream: Stream<Item: Clone>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
