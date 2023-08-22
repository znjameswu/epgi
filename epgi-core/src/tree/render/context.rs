use std::sync::atomic::{AtomicBool, Ordering::*};

use crate::{foundation::Asc, tree::ArcElementContextNode};

pub type AscRenderContextNode = Asc<RenderContextNode>;

pub struct RenderContextNode {
    pub(crate) parent: Option<AscRenderContextNode>,
    pub(crate) nearest_repaint_boundary: AscLayerScopeContextNode,
    pub(crate) is_repaint_boundary: bool,
    pub(crate) needs_layout: AtomicBool,
    pub(crate) needs_paint: AtomicBool,
    pub(crate) subtree_has_layout: AtomicBool,
    pub(crate) subtree_has_paint: AtomicBool,
    pub(crate) is_relayout_boundary: AtomicBool,
}

impl RenderContextNode {
    pub(crate) fn new(parent: AscRenderContextNode) -> Self {
        Self {
            nearest_repaint_boundary: parent.nearest_repaint_boundary.clone(),
            parent: Some(parent),
            is_repaint_boundary: false,
            needs_layout: AtomicBool::new(false),
            needs_paint: AtomicBool::new(false),
            subtree_has_layout: AtomicBool::new(false),
            subtree_has_paint: AtomicBool::new(false),
            is_relayout_boundary: AtomicBool::new(false),
        }
    }

    pub(crate) fn new_repaint_boundary(
        parent: AscRenderContextNode,
        layer_context: AscLayerScopeContextNode,
    ) -> Self {
        Self {
            parent: Some(parent),
            nearest_repaint_boundary: layer_context,
            is_repaint_boundary: false,
            needs_layout: AtomicBool::new(false),
            needs_paint: AtomicBool::new(false),
            subtree_has_layout: AtomicBool::new(false),
            subtree_has_paint: AtomicBool::new(false),
            is_relayout_boundary: AtomicBool::new(false),
        }
    }

    pub(crate) fn new_root() -> Self {
        Self {
            parent: None,
            nearest_repaint_boundary: todo!(),
            is_repaint_boundary: true,
            needs_layout: false.into(),
            needs_paint: false.into(),
            subtree_has_layout: false.into(),
            subtree_has_paint: false.into(),
            is_relayout_boundary: true.into(),
        }
    }

    pub fn is_relayout_boundary(&self) -> bool {
        self.is_relayout_boundary.load(Relaxed)
    }

    pub fn needs_layout(&self) -> bool {
        self.needs_layout.load(Relaxed)
    }

    pub fn subtree_has_layout(&self) -> bool {
        self.subtree_has_layout.load(Relaxed)
    }

    pub fn clear_self_needs_layout(&self) {
        self.needs_layout.store(false, Relaxed)
    }

    pub fn clear_subtree_has_layout(&self) {
        self.subtree_has_layout.store(false, Relaxed)
    }

    pub fn is_repaint_boundary(&self) -> bool {
        self.is_repaint_boundary
    }

    pub fn needs_paint(&self) -> bool {
        self.needs_paint.load(Relaxed)
    }

    pub fn subtree_has_paint(&self) -> bool {
        self.subtree_has_paint.load(Relaxed)
    }

    pub fn clear_self_needs_paint(&self) {
        self.needs_paint.store(false, Relaxed)
    }
}

impl RenderContextNode {
    pub(crate) fn mark_needs_layout(&self) {
        todo!()
    }

    pub(crate) fn mark_needs_paint(&self) {
        todo!()
    }
}

pub type AscLayerScopeContextNode = Asc<LayerScopeContextNode>;

pub struct LayerScopeContextNode {
    pub(crate) parent: Option<AscLayerScopeContextNode>,
    // pub(crate) needs_paint: AtomicBool,
    pub(crate) needs_composite: AtomicBool,
}
