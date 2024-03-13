use std::any::Any;

use crate::{
    foundation::{Arc, Asc, Canvas, LayerProtocol, Protocol},
    tree::{
        CachedCompositionFunctionTable, CachingChildLayerProducingIterator,
        ComposableUnadoptedLayer, CompositeResults, LayerCompositionConfig, LayerRender,
        NonCachingChildLayerProducingIterator, RenderObject,
    },
};

use super::BuildScheduler;

impl BuildScheduler {
    pub(crate) fn perform_composite(&self) -> Asc<dyn Any + Send + Sync> {
        self.root_render_object.recomposite_into_cache()
    }
}

pub trait AnyLayerRenderObjectCompositeExt {
    fn recomposite_into_cache(&self) -> Asc<dyn Any + Send + Sync>;
}

impl<R> AnyLayerRenderObjectCompositeExt for RenderObject<R>
where
    R: LayerRender,
    R::ChildProtocol: LayerProtocol,
    R::ParentProtocol: LayerProtocol,
{
    fn recomposite_into_cache(&self) -> Asc<dyn Any + Send + Sync> {
        let Some(CachedCompositionFunctionTable {
            composite_into_cache,
            ..
        }) = R::CACHED_COMPOSITION_FUNCTION_TABLE
        else {
            panic!("Recomposite can only be called on layer render object with composition caches")
        };
        let no_relayout_token = self.mark.assume_not_needing_layout();
        let _no_detach_token = self.mark.assume_not_detached();
        let needs_composite = self.layer_mark.needs_composite();
        if let Err(no_recomposite_token) = needs_composite {
            let cached_composition = self
                .inner
                .lock()
                .cache
                .layout_cache_mut(no_relayout_token)
                .expect("Layer should only be composited after they are laid out")
                .layer_cache
                .as_mut()
                .expect("Layer should only be composited after they are painted")
                .composite_results_ref(no_recomposite_token)
                .expect(
                    "Caching layers that are not marked as dirty should have a compositiong cache",
                )
                .cached_composition
                .clone();
            return Asc::new(cached_composition);
        }

        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        let layer_cache = inner_reborrow
            .cache
            .layout_cache_mut(no_relayout_token)
            .expect("Layer should only be composited after they are laid out")
            .layer_cache
            .as_mut()
            .expect("Layer should only be composited after they are painted");
        let mut iter = CachingChildLayerProducingIterator {
            paint_results: &layer_cache.paint_results,
            key: inner_reborrow.render.key().map(Arc::as_ref),
            // key: inner_reborrow.layer.key().map(Arc::as_ref),
            unadopted_layers: Vec::new(),
        };
        let cached_composition = composite_into_cache(&mut iter);
        let result = Asc::new(cached_composition.clone());
        layer_cache.insert_composite_results(CompositeResults {
            unadopted_layers: iter.unadopted_layers,
            cached_composition,
        });
        self.layer_mark.clear_needs_composite();
        return result;
    }
}

pub trait ChildLayerRenderObjectCompositeExt<PC: Canvas> {
    fn composite_to(
        &self,
        encoding: &mut PC::Encoding,
        composition_config: &LayerCompositionConfig<PC>,
    ) -> Vec<ComposableUnadoptedLayer<PC>>;
}

impl<R> ChildLayerRenderObjectCompositeExt<<R::ParentProtocol as Protocol>::Canvas>
    for RenderObject<R>
where
    R: LayerRender,
    R::ChildProtocol: LayerProtocol,
    R::ParentProtocol: LayerProtocol,
{
    fn composite_to(
        &self,
        encoding: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> Vec<ComposableUnadoptedLayer<<R::ParentProtocol as Protocol>::Canvas>> {
        let no_relayout_token = self.mark.assume_not_needing_layout();
        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        let layer_cache = inner_reborrow
            .cache
            .layout_cache_mut(no_relayout_token)
            .expect("Layer should only be composited after they are laid out")
            .layer_cache
            .as_mut()
            .expect("Layer should only be composited after they are painted");
        if let Some(CachedCompositionFunctionTable {
            composite_into_cache,
            composite_from_cache_to,
        }) = R::CACHED_COMPOSITION_FUNCTION_TABLE
        {
            if let Err(no_recomposite_token) = self.layer_mark.needs_composite() {
                layer_cache.composite_results_ref(no_recomposite_token)
            } else {
                None
            };

            let composite_results = self
                .layer_mark
                .needs_composite()
                .err()
                .and_then(|token| layer_cache.composite_results_ref(token));

            let composite_results = if let Some(composite_results) = composite_results {
                composite_from_cache_to(
                    encoding,
                    &composite_results.cached_composition,
                    composition_config,
                );
                composite_results
            } else {
                let mut iter = CachingChildLayerProducingIterator {
                    paint_results: &layer_cache.paint_results,
                    key: inner_reborrow.render.key().map(Arc::as_ref),
                    // key: inner_reborrow.layer.key().map(Arc::as_ref),
                    unadopted_layers: Vec::new(),
                };
                let results = composite_into_cache(&mut iter);
                composite_from_cache_to(encoding, &results, composition_config);
                layer_cache.insert_composite_results(CompositeResults {
                    unadopted_layers: iter.unadopted_layers,
                    cached_composition: results,
                })
            };

            self.layer_mark.clear_needs_composite();
            return composite_results
                .unadopted_layers
                .iter()
                .map(|unadopted_layer| ComposableUnadoptedLayer {
                    config: R::transform_config(composition_config, &unadopted_layer.config),
                    adopter_key: unadopted_layer.adopter_key.clone(),
                    layer: unadopted_layer.layer.clone(),
                })
                .collect();
        } else {
            let mut iter = NonCachingChildLayerProducingIterator {
                paint_results: &layer_cache.paint_results,
                key: inner_reborrow.render.key().map(Arc::as_ref),
                unadopted_layers: Vec::new(),
                composition_config,
                transform_config: R::transform_config,
            };
            <R as LayerRender>::composite_to(encoding, &mut iter, composition_config);
            self.layer_mark.clear_needs_composite();
            return iter.unadopted_layers;
        }
    }
}
