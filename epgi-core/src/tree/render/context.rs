use std::sync::atomic::{AtomicBool, Ordering::*};

use crate::{
    foundation::Asc,
    tree::{AscLayerContextNode, LayerContextNode},
};

pub type AscRenderContextNode = Asc<RenderContextNode>;

pub struct RenderContextNode {
    pub(crate) parent: Option<AscRenderContextNode>,
    pub(crate) nearest_repaint_boundary: AscLayerContextNode,
    pub(crate) is_repaint_boundary: bool,
    pub(crate) needs_layout: AtomicBool,
    // pub(crate) needs_paint: AtomicBool,
    pub(crate) subtree_has_layout: AtomicBool,
    // pub(crate) subtree_has_paint: AtomicBool,
    pub(crate) is_relayout_boundary: AtomicBool,
}

impl RenderContextNode {
    pub(crate) fn new_render(parent: AscRenderContextNode) -> Self {
        Self::new(Some(parent), None)
    }

    pub(crate) fn new_repaint_boundary(parent: AscRenderContextNode) -> Self {
        let layer_context = Asc::new(LayerContextNode::new(Some(
            parent.nearest_repaint_boundary.clone(),
        )));
        Self::new(Some(parent), Some(layer_context))
    }

    pub(crate) fn new_root() -> Self {
        let layer_context = Asc::new(LayerContextNode::new(None));
        Self::new(None, Some(layer_context))
    }

    #[inline(always)]
    fn new(
        parent: Option<AscRenderContextNode>,
        layer_context: Option<AscLayerContextNode>,
    ) -> Self {
        if let Some(parent) = parent {
            let (nearest_repaint_boundary, is_repaint_boundary) =
                if let Some(layer_context) = layer_context {
                    (layer_context, true)
                } else {
                    (parent.nearest_repaint_boundary.clone(), false)
                };
            Self {
                parent: Some(parent),
                nearest_repaint_boundary,
                is_repaint_boundary,
                needs_layout: AtomicBool::new(false),
                // needs_paint: AtomicBool::new(false),
                subtree_has_layout: AtomicBool::new(false),
                // subtree_has_paint: AtomicBool::new(false),
                is_relayout_boundary: AtomicBool::new(false),
            }
        } else {
            Self {
                parent: None,
                nearest_repaint_boundary: layer_context
                    .expect("A root render object must have a layer"),
                is_repaint_boundary: true,
                needs_layout: AtomicBool::new(false),
                // needs_paint: AtomicBool::new(false),
                subtree_has_layout: AtomicBool::new(false),
                // subtree_has_paint: AtomicBool::new(false),
                is_relayout_boundary: AtomicBool::new(true),
            }
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

    // pub fn needs_paint(&self) -> bool {
    //     self.needs_paint.load(Relaxed)
    // }

    // pub fn subtree_has_paint(&self) -> bool {
    //     self.subtree_has_paint.load(Relaxed)
    // }

    // pub fn clear_self_needs_paint(&self) {
    //     self.needs_paint.store(false, Relaxed)
    // }

    // pub fn clear_subtree_has_paint(&self) {
    //     self.subtree_has_paint.store(false, Relaxed)
    // }
}

impl RenderContextNode {
    pub(crate) fn mark_needs_layout(&self) {
        let mut cur = self;
        // Mark up to the nearest relayout boundary
        loop {
            let old_needs_relayout = cur.needs_layout.swap(true, Relaxed);
            cur.subtree_has_layout.store(true, Relaxed);
            cur.mark_needs_paint();
            if cur.is_relayout_boundary.load(Relaxed) {
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
            let old_subtree_contains_relayout = cur.subtree_has_layout.swap(true, Relaxed);
            if old_subtree_contains_relayout {
                break;
            }
        }
    }

    pub(crate) fn mark_needs_paint(&self) {
        todo!()
    }
}
