use crate::foundation::{Canvas, SyncMutex};

/// Fragments are ephemeral. Scopes are persistent.
pub struct LayerFragment<C: Canvas> {
    inner: SyncMutex<LayerFragmentInner<C>>,
}

struct LayerFragmentInner<C: Canvas> {
    transform_abs: C::Transform,
    encoding: C::Encoding,
}
