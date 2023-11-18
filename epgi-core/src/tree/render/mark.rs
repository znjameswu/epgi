use std::sync::atomic::{AtomicBool, Ordering::*};

use crate::tree::LayerOrUnit;

use super::{Render, RenderAction, RenderObject};

pub(crate) struct RenderMark {
    needs_layout: AtomicBool,
    subtree_has_layout: AtomicBool,
    is_relayout_boundary: AtomicBool,
    is_detached: AtomicBool,
}

// Nonconstructible ZST
#[derive(Clone, Copy)]
pub(crate) struct NoRelayoutToken(());

#[derive(Clone, Copy)]
pub(crate) struct NotDetachedToken(());

impl RenderMark {
    pub(crate) fn new() -> Self {
        Self {
            needs_layout: true.into(),
            subtree_has_layout: true.into(),
            is_relayout_boundary: false.into(),
            is_detached: false.into(),
        }
    }

    pub(crate) fn is_detached(&self) -> Result<(), NotDetachedToken> {
        if self.is_detached.load(Relaxed) {
            Ok(())
        } else {
            Err(NotDetachedToken(()))
        }
    }

    pub(crate) fn assume_not_detached(&self) -> NotDetachedToken {
        debug_assert!(
            !self.is_detached.load(Relaxed),
            "We assumed this render object to be attached"
        );
        NotDetachedToken(())
    }

    pub(crate) fn set_is_detached(&self) {
        self.is_detached.store(true, Relaxed)
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

    pub(crate) fn needs_layout(&self) -> Result<(), NoRelayoutToken> {
        if self.needs_layout.load(Relaxed) {
            Ok(())
        } else {
            Err(NoRelayoutToken(()))
        }
    }

    pub(crate) fn assume_not_needing_layout(&self) -> NoRelayoutToken {
        debug_assert!(
            !self.needs_layout.load(Relaxed),
            "We assumed this render object to be not needing relayout"
        );
        NoRelayoutToken(())
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
        todo!()
    }
}
