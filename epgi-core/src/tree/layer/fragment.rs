use crate::foundation::{Canvas, SyncMutex};

/// Fragments are ephemeral. Scopes are persistent.
pub struct LayerFragment<C: Canvas> {
    inner: SyncMutex<LayerFragmentInner<C>>,
}

struct LayerFragmentInner<C: Canvas> {
    encoding: C::Encoding,
}
