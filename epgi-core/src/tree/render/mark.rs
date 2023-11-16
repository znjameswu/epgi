use std::sync::atomic::{AtomicBool, Ordering::*};

use crate::tree::LayerOrUnit;

use super::{Render, RenderAction, RenderObject};

pub(crate) struct RenderMark {
    pub(crate) needs_layout: AtomicBool,
    pub(crate) subtree_has_layout: AtomicBool,
    pub(crate) is_relayout_boundary: AtomicBool,
}

impl RenderMark {
    pub(crate) fn new() -> Self {
        Self {
            needs_layout: true.into(),
            subtree_has_layout: true.into(),
            is_relayout_boundary: false.into(),
        }
    }

    pub(crate) fn is_relayout_boundary(&self) -> bool {
        self.is_relayout_boundary.load(Relaxed)
    }

    pub(crate) fn clear_is_relayout_boundary(&self) {
        self.is_relayout_boundary.store(false, Relaxed)
    }

    pub(crate) fn set_is_relayout_boundary(&self) {
        self.is_relayout_boundary.store(true, Relaxed)
    }

    pub(crate) fn needs_layout(&self) -> bool {
        self.needs_layout.load(Relaxed)
    }

    pub(crate) fn subtree_has_layout(&self) -> bool {
        self.subtree_has_layout.load(Relaxed)
    }

    pub(crate) fn clear_self_needs_layout(&self) {
        self.needs_layout.store(false, Relaxed)
    }

    pub(crate) fn clear_subtree_has_layout(&self) {
        self.subtree_has_layout.store(false, Relaxed)
    }

    pub(crate) fn set_self_needs_layout(&self) {
        self.needs_layout.store(true, Relaxed)
    }

    pub(crate) fn set_subtree_has_layout(&self) {
        self.subtree_has_layout.store(true, Relaxed)
    }
}

impl<R> RenderObject<R>
where
    R: Render,
{
    /// Returns the render action that should be passed to the parent.
    /// The render action is less or equal to the child_render_action,
    /// because some of the action may be absorbed by the corresponding boundaries.
    pub(crate) fn mark_render_action(
        &self,
        mut child_render_action: RenderAction,
        subtree_has_action: RenderAction,
    ) -> RenderAction {
        if child_render_action == RenderAction::Relayout {
            self.mark.set_self_needs_layout();
            if self.mark.is_relayout_boundary() {
                child_render_action = RenderAction::Repaint;
            }
        }
        if subtree_has_action == RenderAction::Relayout {
            self.mark.set_subtree_has_layout();
        }
        return <R::LayerOrUnit as LayerOrUnit<R>>::mark_render_action(
            &self.layer_node,
            child_render_action,
            subtree_has_action,
        );
    }
}
