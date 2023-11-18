use crate::tree::{Layer, LayerNode};

impl<L> LayerNode<L>
where
    L: Layer,
{
    fn recomposite(&self) {
        // let mut result = <L::ParentCanvas as Canvas>::new_encoding();
        // let mut inner = self.inner.lock();
        // let inner_reborrow = &mut *inner;
        // if let Some(CachedCompositionFunctionTable {
        //     composite_to_cache,
        //     composite_from_cache_to,
        // }) = L::CACHED_COMPOSITION_FUNCTION_TABLE
        // {
        //     let cache = inner_reborrow
        //         .cache
        //         .as_mut()
        //         .expect("Composite can only happen after painting has finished");
        //     if let Some(composition_cache) = &cache.composite_results {
        //         composite_from_cache_to(
        //             &inner_reborrow.layer,
        //             &mut result,
        //             &composition_cache.cached_composition,
        //             &LayerCompositionConfig::new(),
        //         );
        //     } else {
        //         let mut it = CachingChildLayerProducingIterator {
        //             painting_results: &cache.paint_results,
        //             key: None,
        //             unadopted_layers: Default::default(),
        //         };
        //         let results = composite_to_cache(&inner_reborrow.layer, &mut it);
        //         cache.composite_results = Some(CompositionResults {
        //             unadopted_layers: it.unadopted_layers,
        //             cached_composition: results,
        //         });
        //     }
        // } else {
        //     todo!("panic")
        // }
        todo!()
    }
}

mod composite_private {
    use super::*;
    pub trait AnyLayerCompositeExt {
        fn recomposite(&self);
    }

    impl<L> AnyLayerCompositeExt for LayerNode<L>
    where
        L: Layer,
    {
        fn recomposite(&self) {
            self.recomposite()
        }
    }
}
