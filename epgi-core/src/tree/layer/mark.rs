use std::sync::atomic::{AtomicBool, Ordering::*};

pub(crate) struct LayerMark {
    pub(crate) needs_paint: AtomicBool,
    pub(crate) needs_composite: AtomicBool,
    pub(crate) subtree_has_composite: AtomicBool,
}

impl LayerMark {
    pub(crate) fn new() -> Self {
        Self {
            needs_paint: true.into(),
            needs_composite: true.into(),
            subtree_has_composite: true.into(),
        }
    }

    pub(crate) fn needs_paint(&self) -> bool {
        self.needs_paint.load(Relaxed)
    }

    pub(crate) fn needs_composite(&self) -> bool {
        self.needs_composite.load(Relaxed)
    }

    pub(crate) fn subtree_has_composite(&self) -> bool {
        self.subtree_has_composite.load(Relaxed)
    }

    pub(crate) fn clear_needs_paint(&self) {
        self.needs_paint.store(false, Relaxed)
    }

    pub(crate) fn clear_needs_composite(&self) {
        self.needs_composite.store(false, Relaxed)
    }

    pub(crate) fn clear_subtree_has_composite(&self) {
        self.subtree_has_composite.store(false, Relaxed)
    }

    pub(crate) fn set_needs_paint(&self) {
        self.needs_paint.store(true, Relaxed)
    }

    pub(crate) fn set_needs_composite(&self) {
        self.needs_composite.store(true, Relaxed)
    }

    pub(crate) fn set_subtree_has_composite(&self) {
        self.subtree_has_composite.store(true, Relaxed)
    }
}
