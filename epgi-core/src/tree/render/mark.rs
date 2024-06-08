use std::sync::atomic::{AtomicBool, Ordering::*};

pub(crate) struct RenderMark {
    needs_layout: AtomicBool,
    descendant_has_layout: AtomicBool,
    parent_use_size: AtomicBool,
    is_detached: AtomicBool,
}

pub struct LayerMark {
    // pub(crate) needs_paint: AtomicBool,
    pub(crate) needs_composite: AtomicBool,
    // pub(crate) subtree_has_composite: AtomicBool,
}

// Nonconstructible ZST
#[derive(Clone, Copy)]
pub(crate) struct NoRelayoutToken(());

#[derive(Clone, Copy)]
pub(crate) struct NotDetachedToken(());

pub(crate) struct NoRecompositeToken(());

impl RenderMark {
    pub(crate) fn new() -> Self {
        Self {
            needs_layout: true.into(),
            descendant_has_layout: true.into(),
            parent_use_size: true.into(),
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

    pub(crate) fn assert_not_detached(&self) -> NotDetachedToken {
        debug_assert!(
            !self.is_detached.load(Relaxed),
            "We assumed this render object to be attached"
        );
        NotDetachedToken(())
    }

    pub(crate) fn set_is_detached(&self) {
        self.is_detached.store(true, Relaxed)
    }

    pub(crate) fn parent_use_size(&self) -> bool {
        self.parent_use_size.load(Relaxed)
    }

    pub(crate) fn clear_parent_use_size(&self) {
        self.parent_use_size.store(false, Relaxed)
    }

    pub(crate) fn set_parent_use_size(&self) {
        self.parent_use_size.store(true, Relaxed)
    }

    pub(crate) fn needs_layout(&self) -> Result<(), NoRelayoutToken> {
        if self.needs_layout.load(Relaxed) {
            Ok(())
        } else {
            Err(NoRelayoutToken(()))
        }
    }

    pub(crate) fn assert_not_needing_layout(&self) -> NoRelayoutToken {
        debug_assert!(
            !self.needs_layout.load(Relaxed),
            "We assumed this render object to be not needing relayout"
        );
        NoRelayoutToken(())
    }

    pub(crate) fn clear_self_needs_layout(&self) {
        self.needs_layout.store(false, Relaxed)
    }

    pub(crate) fn set_self_needs_layout(&self) {
        self.needs_layout.store(true, Relaxed)
    }

    pub(crate) fn descendant_has_layout(&self) -> bool {
        self.descendant_has_layout.load(Relaxed)
    }

    pub(crate) fn clear_descendant_has_layout(&self) {
        self.descendant_has_layout.store(false, Relaxed)
    }

    pub(crate) fn set_descendant_has_layout(&self) {
        self.descendant_has_layout.store(true, Relaxed)
    }
}

impl Default for LayerMark {
    fn default() -> Self {
        Self::new()
    }
}

impl LayerMark {
    pub(crate) fn new() -> Self {
        Self {
            needs_composite: true.into(),
        }
    }

    pub(crate) fn needs_composite(&self) -> Result<(), NoRecompositeToken> {
        if self.needs_composite.load(Relaxed) {
            Ok(())
        } else {
            Err(NoRecompositeToken(()))
        }
    }

    pub(crate) fn assert_not_needing_composite(&self) -> NoRecompositeToken {
        debug_assert!(
            !self.needs_composite.load(Relaxed),
            "We assumed this render object to be not needing recomposite"
        );
        NoRecompositeToken(())
    }

    pub(crate) fn set_needs_composite(&self) {
        self.needs_composite.store(true, Relaxed)
    }

    pub(crate) fn clear_needs_composite(&self) {
        self.needs_composite.store(false, Relaxed)
    }
}
