use std::sync::atomic::{AtomicIsize, Ordering::*};

use crate::foundation::Asc;

pub struct CommitBarrier {
    inner: Asc<CommitBarrierInner>,
}

// We need to ensure the commit barrier can only be created while holding the TreeScheduler lock.
pub(super) struct CommitBarrierInner {
    counter: AtomicIsize,
    // // We chose to use an event passing system, rather than calling scheduler. Because calling scheduler requires polymorphism on generics and async-await inside Drop
    // complete_event: event_listener::Event,
}

impl CommitBarrierInner {
    pub(crate) fn dec(&self) {
        self.counter.fetch_sub(1, Relaxed);
    }

    pub(super) fn inc(&self) {
        self.counter.fetch_add(1, Relaxed);
    }

    pub(super) fn new() -> Self {
        Self {
            counter: AtomicIsize::new(0),
            // complete_event: event_listener::Event::new(),
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.counter.load(Relaxed) == 0
    }
}

impl CommitBarrier {
    // We need to ensure the commit barrier can only be created while holding the TreeScheduler lock.
    // Hence the limited visibility under the current module
    // No one should be able to increment the counter from 0 to 1 without holding the TreeScheduler lock.
    // Otherwise the increase from 0 to 1 WILL cause data race during the commit.
    pub(super) fn from_inner(inner: Asc<CommitBarrierInner>) -> Self {
        inner.counter.fetch_add(1, Relaxed);
        Self { inner }
    }
}

impl Clone for CommitBarrier {
    fn clone(&self) -> Self {
        self.inner.counter.fetch_add(1, Relaxed);
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Drop for CommitBarrier {
    fn drop(&mut self) {
        let old_val = self.inner.counter.fetch_sub(1, Release);
        if old_val == 0 {
            std::sync::atomic::fence(Acquire);
            // self.inner.complete_event.notify_relaxed(usize::MAX);
        }
    }
}
