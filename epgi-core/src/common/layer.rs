use std::sync::atomic::AtomicBool;

use crate::foundation::{Arc, Asc, Aweak, Canvas, InlinableDwsizeVec, InlinableUsizeVec};

/// Fragments are ephemeral. Layers are persistent.
pub enum LayerNode<C: Canvas> {
    Fragment(Asc<LayerFragment<C>>),
    Layer(Arc<Layer<C>>),
}

pub struct Layer<C: Canvas> {
    root: Aweak<Layer<C>>,
    is_detached: bool,
    // needs_recompositing: AtomicBool,
    inner: LayerInner<C>,
}

pub struct LayerFragment<C: Canvas> {
    inner: LayerFragmentInner<C>,
}

struct LayerInner<C: Canvas> {
    transform_abs: C::Transform,
    structured_children: InlinableDwsizeVec<LayerNode<C>>,
    detached_children: InlinableUsizeVec<Arc<Layer<C>>>,
}

struct LayerFragmentInner<C: Canvas> {
    transform_abs: C::Transform,
    encoding: C::Encoding,
}
