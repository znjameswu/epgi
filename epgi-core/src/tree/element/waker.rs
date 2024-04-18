use std::sync::atomic::{AtomicBool, Ordering::*};

use futures::task::ArcWake;

use crate::{
    foundation::Asc,
    scheduler::{get_current_scheduler, BatchId, LanePos},
};

use super::AweakElementContextNode;

#[derive(Clone)]
pub(crate) struct SuspendWaker(SuspendWakerInner);

#[derive(Clone)]
enum SuspendWakerInner {
    Sync(std::sync::Arc<SyncSuspendWaker>),
    Async(std::sync::Arc<AsyncSuspendWaker>),
}

pub(crate) struct SyncSuspendWaker {
    pub(crate) aborted: AtomicBool,
    pub(crate) node: AweakElementContextNode,
}

impl ArcWake for SyncSuspendWaker {
    fn wake_by_ref(arc_self: &std::sync::Arc<Self>) {
        get_current_scheduler().push_point_rebuild(arc_self.clone())
    }
}

pub(crate) struct AsyncSuspendWaker {
    aborted: AtomicBool,
    // poll_permit: AtomicBool, // This has no use.
    node: AweakElementContextNode,
    // lane: LanePos,
    async_batch: Option<(LanePos, BatchId)>,
}

impl ArcWake for AsyncSuspendWaker {
    fn wake_by_ref(arc_self: &std::sync::Arc<Self>) {
        todo!()
        // get_current_scheduler().
    }
}

impl SuspendWaker {
    pub(crate) fn new_sync(node: AweakElementContextNode) -> Self {
        Self(SuspendWakerInner::Sync(Asc::new(SyncSuspendWaker {
            aborted: AtomicBool::new(false),
            node,
        })))
    }

    pub(crate) fn new_async(
        node: AweakElementContextNode,
        lane_pos: LanePos,
        batch_id: BatchId,
    ) -> Self {
        Self(SuspendWakerInner::Async(Asc::new(AsyncSuspendWaker {
            aborted: AtomicBool::new(false),
            node,
            async_batch: Some((lane_pos, batch_id)),
        })))
    }

    pub(crate) fn into_waker(self) -> std::task::Waker {
        match self.0 {
            SuspendWakerInner::Sync(waker) => futures::task::waker(waker),
            SuspendWakerInner::Async(waker) => futures::task::waker(waker),
        }
    }

    pub(crate) fn abort(&self) {
        match &self.0 {
            SuspendWakerInner::Sync(waker) => waker.aborted.store(true, Relaxed),
            SuspendWakerInner::Async(waker) => waker.aborted.store(true, Relaxed),
        }
    }
}
