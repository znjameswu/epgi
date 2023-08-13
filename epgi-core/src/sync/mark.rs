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
    pub(super) is_poll_ready: AtomicBool,

    pub(super) needs_relayout: AtomicBool,

    pub(super) descendants_contain_relayout: AtomicBool,

    pub(super) is_relayout_boundary: AtomicBool,

    pub(super) needs_repaint: AtomicBool,

    pub(super) descendants_contain_repaint: AtomicBool,

    pub(super) is_repaint_boundary: bool,
}

impl ElementContextNode {
    pub(crate) fn subtree_lanes(&self) -> LaneMask {
        self.mark.subtree_lanes.load(Relaxed)
    }

    pub(crate) fn self_lanes(&self) -> LaneMask {
        self.mark.self_lanes.load(Relaxed)
    }

    pub(crate) fn needs_poll(&self) -> bool {
        self.mark.is_poll_ready.load(Relaxed)
    }

    pub(crate) fn needs_relayout(&self) -> bool {
        self.mark.needs_relayout.load(Relaxed)
    }

    pub(crate) fn needs_repaint(&self) -> bool {
        self.mark.needs_repaint.load(Relaxed)
    }

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

    pub(super) fn mark_needs_layout(&self) {
        let mut cur = self;
        // Mark up to the nearest relayout boundary
        loop {
            let old_needs_relayout = cur.mark.needs_relayout.swap(true, Relaxed);
            cur.mark.descendants_contain_relayout.store(true, Relaxed);
            cur.mark_needs_paint();
            if cur.mark.is_relayout_boundary.load(Relaxed) {
                break;
            }
            if old_needs_relayout {
                break;
            }
            let Some(parent) = &cur.parent else {
                break;
            };
            cur = parent.as_ref();
        }
        // Then, mark all the way up to the root
        loop {
            let Some(parent) = &cur.parent else {
                break;
            };
            cur = parent.as_ref();
            let old_subtree_contains_relayout =
                cur.mark.descendants_contain_relayout.swap(true, Relaxed);
            if old_subtree_contains_relayout {
                break;
            }
        }
    }

    pub(super) fn mark_needs_paint(&self) {
        let mut cur = self;
        // Mark up to the nearest repaint boundary
        loop {
            let old_needs_repaint = cur.mark.needs_repaint.swap(true, Relaxed);
            cur.mark.descendants_contain_repaint.store(true, Relaxed);
            if cur.mark.is_repaint_boundary {
                break;
            }
            if old_needs_repaint {
                break;
            }
            let Some(parent) = &cur.parent else {
                break;
            };
            cur = parent.as_ref();
        }
        // Then, mark all the way up to the root
        loop {
            let Some(parent) = &cur.parent else {
                break;
            };
            cur = parent.as_ref();
            let old_subtree_contains_repaint =
                cur.mark.descendants_contain_repaint.swap(true, Relaxed);
            if old_subtree_contains_repaint {
                break;
            }
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
