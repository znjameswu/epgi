use std::sync::atomic::Ordering::*;

use atomic::Atomic;
use bytemuck::NoUninit;
use futures::task::ArcWake;

use crate::{
    debug::debug_assert_sync_phase,
    scheduler::{get_current_scheduler, LanePos},
};

use super::AweakElementContextNode;

pub(crate) type ArcSuspendWaker = std::sync::Arc<SuspendWaker>;

#[derive(NoUninit, PartialEq, Eq, Clone, Copy, Debug)]
#[repr(u8)]
enum SuspendWakerState {
    Suspended,
    Waken,
    Aborted,
}

pub struct SuspendWaker {
    state: Atomic<SuspendWakerState>,
    lane_pos: Atomic<LanePos>,
    pub(crate) element_context: AweakElementContextNode,
}

impl ArcWake for SuspendWaker {
    fn wake_by_ref(arc_self: &ArcSuspendWaker) {
        if arc_self
            .state
            .compare_exchange(
                SuspendWakerState::Suspended,
                SuspendWakerState::Waken,
                Release,
                Relaxed,
            )
            .is_ok()
        {
            get_current_scheduler().push_suspend_wake(arc_self.clone());
        }
    }
}

impl SuspendWaker {}

impl SuspendWaker {
    // pub(crate) fn new_sync(node: AweakElementContextNode) -> std::sync::Arc<Self> {
    //     std::sync::Arc::new(Self {
    //         state: Atomic::new(SuspendWakerState::Suspended),
    //         lane_pos: Atomic::new(LanePos::SYNC),
    //         node,
    //     })
    // }

    pub(crate) fn new(
        node: AweakElementContextNode,
        lane_pos: LanePos,
        // batch_id: BatchId,
    ) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            state: Atomic::new(SuspendWakerState::Suspended),
            lane_pos: Atomic::new(lane_pos),
            element_context: node,
        })
    }

    pub(crate) fn is_aborted(&self) -> bool {
        debug_assert_sync_phase();
        // Relaxed is okay because we assert now we are in sync phase
        self.state.load(Relaxed) == SuspendWakerState::Aborted
    }

    pub(crate) fn abort(&self) {
        debug_assert_sync_phase();
        // Relaxed is okay because we assert now we are in sync phase
        self.state.store(SuspendWakerState::Aborted, Relaxed)
    }

    pub(crate) fn lane_pos(&self) -> LanePos {
        debug_assert_sync_phase();
        // Relaxed is okay because we assert now we are in sync phase
        self.lane_pos.load(Relaxed)
    }

    pub(crate) fn make_sync(&self) {
        debug_assert_sync_phase();
        self.lane_pos.store(LanePos::SYNC, Relaxed)
    }

    pub(crate) fn into_waker(self: ArcSuspendWaker) -> std::task::Waker {
        futures::task::waker(self)
    }
}
