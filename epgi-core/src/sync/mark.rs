use core::sync::atomic::{AtomicBool, Ordering::*};

use crate::{
    foundation::PtrEq,
    scheduler::{AtomicLaneMask, LaneMask, LanePos},
    tree::{ArcElementContextNode, Element, ElementContextNode, ElementNode},
};

pub(crate) struct ElementMark {
    /// Sum of all the `self_lanes` in the subtree, including this node, plus `is_poll_ready` for sync lane.
    pub(super) subtree_lanes: AtomicLaneMask,
    /// Lanes in the mailbox + lanes marked as secondary roots
    pub(super) self_lanes: AtomicLaneMask,
    /// Lanes that are present in the mailbox
    pub(super) self_mailbox_lanes: AtomicLaneMask,
    /// Indicate whether this node requested for a sync poll
    pub(super) needs_poll: AtomicBool,
}

impl ElementMark {
    pub(crate) fn new() -> Self {
        Self {
            subtree_lanes: AtomicLaneMask::new(LaneMask::new()),
            self_lanes: AtomicLaneMask::new(LaneMask::new()),
            self_mailbox_lanes: AtomicLaneMask::new(LaneMask::new()),
            needs_poll: AtomicBool::new(false),
        }
    }
}

impl ElementContextNode {
    pub(crate) fn subtree_lanes(&self) -> LaneMask {
        self.mark.subtree_lanes.load(Relaxed)
    }

    pub(crate) fn self_lanes(&self) -> LaneMask {
        self.mark.self_lanes.load(Relaxed)
    }

    pub(crate) fn needs_poll(&self) -> bool {
        self.mark.needs_poll.load(Relaxed)
    }

    // pub(crate) fn needs_layout(&self) -> bool {
    //     self.mark.needs_layout.load(Relaxed)
    // }

    // pub(crate) fn needs_paint(&self) -> bool {
    //     self.mark.needs_paint.load(Relaxed)
    // }

    pub(crate) fn mark_secondary_root(&self, lane_pos: LanePos) {
        self.mark.self_lanes.fetch_insert_single(lane_pos, Relaxed);
        let mut cur = self;
        loop {
            let old_subtree_lanes = cur
                .mark
                .subtree_lanes
                .fetch_insert_single(lane_pos, Relaxed);
            if old_subtree_lanes.contains(lane_pos) {
                break;
            }
            let Some(parent) = &cur.parent else {
                break;
            };
            cur = parent.as_ref();
        }
    }
}

impl<E> ElementNode<E>
where
    E: Element,
{
    // SAFETY: this function must be called while holding the element node lock. Otherwise the relaxed load becomes a problem.
    // There are no state update, no widget update, no provider update, no poll ready, nothing on this node for this lane.
    pub(crate) fn can_skip_work(
        new_widget: &Option<E::ArcWidget>,
        old_widget: &E::ArcWidget,
        lane_pos: LanePos,
        element_context: &ArcElementContextNode,
    ) -> bool {
        // An update and a secondary root should mark ElementContextNode.self_lanes
        let no_widget_update = if let Some(widget) = new_widget {
            PtrEq(widget) == PtrEq(old_widget)
        } else {
            true
        };
        let no_rebuild = !element_context.self_lanes().contains(lane_pos);
        let no_poll = !element_context.needs_poll();
        return no_widget_update && no_rebuild && no_poll;
    }

    // There are no state update, no widget update, no provider update on this node for this lane. But there may be poll ready on this lane.
    pub(crate) fn can_skip_rebuild(
        new_widget: &Option<E::ArcWidget>,
        old_widget: &E::ArcWidget,
        lane_pos: LanePos,
        element_context: &ArcElementContextNode,
    ) -> bool {
        let no_widget_update = if let Some(widget) = new_widget {
            PtrEq(widget) == PtrEq(old_widget)
        } else {
            true
        };
        let no_rebuild = !element_context.self_lanes().contains(lane_pos);
        return no_widget_update && no_rebuild;
    }
}
