use std::sync::atomic::{AtomicBool, Ordering::*};

use crate::foundation::Asc;

pub type AscLayerScopeContextNode = Asc<LayerScopeContextNode>;

pub struct LayerScopeContextNode {
    pub(crate) parent: Option<AscLayerScopeContextNode>,
    pub(crate) needs_paint: AtomicBool,
    pub(crate) needs_composite: AtomicBool,
    pub(crate) subtree_has_composite: AtomicBool,
}

impl LayerScopeContextNode {
    pub fn needs_paint(&self) -> bool {
        self.needs_paint.load(Relaxed)
    }

    pub(crate) fn clear_self_needs_paint(&self) {
        self.needs_paint.store(false, Relaxed)
    }

    pub fn needs_composite(&self) -> bool {
        self.needs_composite.load(Relaxed)
    }

    pub fn subtree_has_composite(&self) -> bool {
        self.subtree_has_composite.load(Relaxed)
    }

    pub(crate) fn clear_self_needs_composite(&self) {
        self.needs_composite.store(false, Relaxed)
    }

    pub(crate) fn clear_subtree_has_composite(&self) {
        self.subtree_has_composite.store(false, Relaxed)
    }
}
