use core::sync::atomic::{AtomicBool, Ordering::*};
use std::sync::atomic::AtomicU16;

use crate::{
    foundation::PtrEq,
    scheduler::{AtomicLaneMask, LaneMask, LanePos},
    tree::{ArcElementContextNode, Element, ElementContextNode, ElementNode},
};

pub(crate) struct ElementMark {
    /// Sum of all the `self_lanes` in the subtree, including this node, plus `is_poll_ready` for sync lane.
    pub(super) _subtree_lanes: AtomicLaneMask,
    /// Lanes in the mailbox + lanes marked as secondary roots
    pub(super) _self_lanes: AtomicLaneMask,
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
            _subtree_lanes: AtomicLaneMask::new(LaneMask::new()),
            _self_lanes: AtomicLaneMask::new(LaneMask::new()),
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
}
