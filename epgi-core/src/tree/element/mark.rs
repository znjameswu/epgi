use core::sync::atomic::{AtomicBool, Ordering::*};
use std::ops::{AddAssign, SubAssign};

use crate::{
    foundation::{SmallMap, SyncMutex},
    scheduler::{AtomicLaneMask, LaneMask, LanePos},
    tree::ElementContextNode,
};

use super::NotUnmountedToken;

pub(crate) struct ElementMark {
    /// Lanes that are present in the mailbox
    pub(super) mailbox_lanes: AtomicLaneMask,

    pub(super) consumer_root_lanes: AtomicLaneMask, // ConsumerRootRefCount,

    pub(super) async_consumer_root_lane_update_pending: AtomicBool,

    pub(super) async_consumer_root_refcount: SyncMutex<SmallMap<u8, u8>>,

    pub(super) descendant_lanes: AtomicLaneMask,
    /// Indicate whether this node requested for a sync poll
    pub(super) needs_poll: AtomicBool,
}

impl ElementMark {
    pub(crate) fn new() -> Self {
        Self {
            mailbox_lanes: AtomicLaneMask::new(LaneMask::new()),
            consumer_root_lanes: AtomicLaneMask::new(LaneMask::new()),
            async_consumer_root_lane_update_pending: AtomicBool::new(false),
            async_consumer_root_refcount: Default::default(),
            descendant_lanes: AtomicLaneMask::new(LaneMask::new()),
            needs_poll: AtomicBool::new(false),
        }
    }
}

impl ElementContextNode {
    pub(crate) fn mailbox_lanes(&self) -> LaneMask {
        self.mark.mailbox_lanes.load(Relaxed)
    }

    pub(crate) fn consumer_root_lanes(&self) -> LaneMask {
        self.mark.consumer_root_lanes.load(Relaxed)
    }

    pub(crate) fn descendant_lanes(&self) -> LaneMask {
        self.mark.descendant_lanes.load(Relaxed)
    }

    pub(crate) fn needs_poll(&self) -> bool {
        self.mark.needs_poll.load(Relaxed)
    }

    pub(crate) fn mark_root(&self, lane_pos: LanePos, not_unmounted: NotUnmountedToken) {
        self.mark
            .mailbox_lanes
            .fetch_insert_single(lane_pos, Relaxed);
        self.mark_up(lane_pos, not_unmounted)
    }

    pub(crate) fn mark_point_rebuild(&self, not_unmounted: NotUnmountedToken) {
        self.mark.needs_poll.store(true, Relaxed);
        self.mark_up(LanePos::SYNC, not_unmounted)
    }

    pub(crate) fn mark_consumer_root(
        &self,
        lane_pos: LanePos,
        not_unmounted: NotUnmountedToken,
    ) -> bool {
        let old_consumer_root_lanes = self
            .mark
            .consumer_root_lanes
            .fetch_insert_single(lane_pos, Relaxed);
        if let Some(pos) = lane_pos.async_lane_pos() {
            self.mark
                .async_consumer_root_refcount
                .lock()
                .entry(pos)
                .or_insert(0)
                .add_assign(1);
        }
        if old_consumer_root_lanes.contains(lane_pos) {
            return false;
        }
        self.mark_up(lane_pos, not_unmounted);
        return true;
    }

    fn mark_up(&self, lane_pos: LanePos, not_unmounted: NotUnmountedToken) {
        let mut curr = self;
        loop {
            let Some(parent) = curr.parent(not_unmounted) else {
                break;
            };
            curr = parent.as_ref();
            let old_descendant_lanes = curr
                .mark
                .descendant_lanes
                .fetch_insert_single(lane_pos, Relaxed);
            if old_descendant_lanes.contains(lane_pos) {
                break;
            }
        }
    }

    pub(crate) fn dec_async_consumer_root(&self, pos: u8) {
        let mut lock = self.mark.async_consumer_root_refcount.lock();
        let linear_map::Entry::Occupied(mut entry) = lock.entry(pos) else {
            panic!("Tried to decrement ref count on non-existing ref count.")
        };
        let ref_count = entry.get_mut();
        ref_count.sub_assign(1);
        if *ref_count == 0 {
            self.mark
                .async_consumer_root_lane_update_pending
                .store(true, Relaxed);
            entry.remove();
        }
    }

    pub(crate) fn purge_lane(&self, lane_pos: LanePos) {
        self.mark
            .mailbox_lanes
            .fetch_remove_single(lane_pos, Relaxed);
        self.mark
            .consumer_root_lanes
            .fetch_remove_single(lane_pos, Relaxed);
        self.mark
            .descendant_lanes
            .fetch_remove_single(lane_pos, Relaxed);
        if let Some(pos) = lane_pos.async_lane_pos() {
            if let Some(_) = self.mark.async_consumer_root_refcount.lock().remove(&pos) {
                self.mark
                    .async_consumer_root_lane_update_pending
                    .store(true, Relaxed);
            }
        } else {
            self.mark.needs_poll.store(false, Relaxed)
        }
    }
}
