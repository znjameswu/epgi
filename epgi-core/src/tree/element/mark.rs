use core::sync::atomic::{AtomicBool, Ordering::*};

use crate::{
    scheduler::{AtomicLaneMask, LaneMask, LanePos},
    tree::ElementContextNode,
};

pub(crate) struct ElementMark {
    /// Lanes that are present in the mailbox
    pub(super) mailbox_lanes: AtomicLaneMask,

    pub(super) consumer_root_lanes: AtomicLaneMask,

    pub(super) descendant_lanes: AtomicLaneMask,
    /// Indicate whether this node requested for a sync poll
    pub(super) needs_poll: AtomicBool,
}

impl ElementMark {
    pub(crate) fn new() -> Self {
        Self {
            mailbox_lanes: AtomicLaneMask::new(LaneMask::new()),
            consumer_root_lanes: AtomicLaneMask::new(LaneMask::new()),
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

    pub(crate) fn mark_root(&self, lane_pos: LanePos) {
        self.mark
            .mailbox_lanes
            .fetch_insert_single(lane_pos, Relaxed);
        self.mark_up(lane_pos)
    }

    pub(crate) fn mark_point_rebuild(&self) {
        self.mark.needs_poll.store(true, Relaxed);
        self.mark_up(LanePos::Sync)
    }

    fn mark_up(&self, lane_pos: LanePos) {
        let mut curr = self;
        loop {
            let Some(parent) = &curr.parent else {
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

    pub(crate) fn mark_consumer_root(&self, lane_pos: LanePos) -> bool {
        let old_consumer_root_lanes = self
            .mark
            .consumer_root_lanes
            .fetch_insert_single(lane_pos, Relaxed);
        if old_consumer_root_lanes.contains(lane_pos) {
            return false;
        }
        let mut curr = self;
        loop {
            let Some(parent) = &curr.parent else {
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
        return true;
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
        if lane_pos.is_sync() {
            self.mark.needs_poll.store(false, Relaxed)
        }
    }
}
