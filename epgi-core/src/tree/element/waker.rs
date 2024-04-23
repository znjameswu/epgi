use std::sync::atomic::{AtomicBool, Ordering::*};

use futures::task::ArcWake;

use crate::{
    debug::debug_assert_sync_phase,
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
    completed: AtomicBool,
    pub(crate) has_woken: AtomicBool,
    pub(crate) node: AweakElementContextNode,
}

impl ArcWake for SyncSuspendWaker {
    fn wake_by_ref(arc_self: &std::sync::Arc<Self>) {
        if !arc_self.completed.load(Relaxed) {
            get_current_scheduler().push_point_rebuild(arc_self.clone());
        }
    }
}

impl SyncSuspendWaker {
    pub(crate) fn completed(&self) -> bool {
        debug_assert_sync_phase();
        self.completed.load(Relaxed)
    }
}

pub(crate) struct AsyncSuspendWaker {
    completed: AtomicBool,
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
            completed: false.into(),
            has_woken: AtomicBool::new(false),
            node,
        })))
    }

    pub(crate) fn new_async(
        node: AweakElementContextNode,
        lane_pos: LanePos,
        batch_id: BatchId,
    ) -> Self {
        Self(SuspendWakerInner::Async(Asc::new(AsyncSuspendWaker {
            completed: false.into(),
            node,
            async_batch: Some((lane_pos, batch_id)),
        })))
    }

    pub(crate) fn completed(&self) -> bool {
        debug_assert_sync_phase();
        match &self.0 {
            SuspendWakerInner::Sync(waker) => waker.completed.load(Relaxed),
            SuspendWakerInner::Async(waker) => waker.completed.load(Relaxed),
        }
    }

    pub(crate) fn set_completed(&self) {
        debug_assert_sync_phase();
        match &self.0 {
            SuspendWakerInner::Sync(waker) => waker.completed.store(true, Relaxed),
            SuspendWakerInner::Async(waker) => waker.completed.store(true, Relaxed),
        }
    }

    // pub(crate) fn suspend(&self) -> WakerState {
    //     let state = self.state_atomic();

    //     match state.compare_exchange(WakerState::Polling, WakerState::Ready, Release, Relaxed) {
    //         Ok(old_state) | Err(old_state) => old_state,
    //     }
    // }

    pub(crate) fn into_waker(self) -> std::task::Waker {
        match self.0 {
            SuspendWakerInner::Sync(waker) => futures::task::waker(waker),
            SuspendWakerInner::Async(waker) => futures::task::waker(waker),
        }
    }
}
