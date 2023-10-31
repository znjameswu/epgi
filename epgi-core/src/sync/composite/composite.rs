use crate::tree::{CachedCompositionFunctionTable, Layer, LayerNode};

impl<L> LayerNode<L>
where
    L: Layer,
{
    fn composite(&self) {
        let mut inner = self.inner.lock();
        if let Some(CachedCompositionFunctionTable {
            composite_to,
            composite_from_cache_to,
        }) = L::CACHED_COMPOSITION_FUNCTION_TABLE
        {
            // inner.cache.as_ref().
        } else {
        }
        todo!()
    }
}
