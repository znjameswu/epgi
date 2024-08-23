use std::{
    sync::atomic::{AtomicBool, AtomicUsize, Ordering::*},
    usize,
};

use crate::scheduler::LAYOUT_PASS_ID;

pub struct RenderMark {
    needs_layout: AtomicBool,
    descendant_has_layout: AtomicBool,
    parent_use_size: ParentUseSize,
    pub(crate) is_detached: AtomicBool,
}

struct ParentUseSize(AtomicUsize);

impl ParentUseSize {
    const FLAG_MASK: usize = 1 << (8 * size_of::<usize>() - 1);
    fn get(&self) -> bool {
        self.0.load(Relaxed) & Self::FLAG_MASK != 0
    }

    fn try_clear(&self) {
        loop {
            let layout_pass = LAYOUT_PASS_ID.load(Relaxed);
            let stamp = self.0.load(Relaxed);
            if stamp & (!Self::FLAG_MASK) != layout_pass {
                let new_stamp = layout_pass & (!Self::FLAG_MASK);
                if let Ok(_) = self
                    .0
                    .compare_exchange_weak(stamp, new_stamp, Relaxed, Relaxed)
                {
                    break;
                }
            }
        }
    }

    fn set(&self) {
        loop {
            let layout_pass = LAYOUT_PASS_ID.load(Relaxed);
            let stamp = self.0.load(Relaxed);
            if stamp & (!Self::FLAG_MASK) != layout_pass {
                let new_stamp = layout_pass | Self::FLAG_MASK;
                if let Ok(_) = self
                    .0
                    .compare_exchange_weak(stamp, new_stamp, Relaxed, Relaxed)
                {
                    break;
                }
            }
        }
    }
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
            parent_use_size: ParentUseSize(ParentUseSize::FLAG_MASK.into()),
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

    // pub(crate) fn set_is_detached(&self) {
    //     self.is_detached.store(true, Relaxed)
    // }

    pub(crate) fn parent_use_size(&self) -> bool {
        self.parent_use_size.get()
    }

    pub(crate) fn try_clear_parent_use_size(&self) {
        self.parent_use_size.try_clear()
    }

    pub(crate) fn set_parent_use_size(&self) {
        self.parent_use_size.set()
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
