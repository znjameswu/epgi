use crate::{
    foundation::{Canvas, InlinableDwsizeVec, SyncMutex},
    tree::AscRenderContextNode,
};

use super::{ArcChildLayer, AweakParentLayer};

/// A transparent, unretained internal layer.
pub struct ComponentLayer<C: Canvas> {
    detached_parent: Option<AweakParentLayer<C>>,
    context: AscRenderContextNode,
    inner: SyncMutex<ComponentLayerInner<C>>,
}

struct ComponentLayerInner<C: Canvas> {
    transform_abs: C::Transform,
    structured_children: InlinableDwsizeVec<ArcChildLayer<C>>,
    detached_children: InlinableDwsizeVec<ArcChildLayer<C>>,
}


