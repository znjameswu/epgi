use crate::{
    foundation::{Canvas, LayerProtocol, Protocol},
    tree::{
        CachedCompositionFunctionTable, CachingChildLayerProducingIterator,
        ComposableUnadoptedLayer, CompositeResults, Layer, LayerCompositionConfig,
        NonCachingChildLayerProducingIterator, Render, RenderObject,
    },
};

impl<R, L> RenderObject<R>
where
    R: Render<LayerOrUnit = L>,
    R::ChildProtocol: LayerProtocol,
    R::ParentProtocol: LayerProtocol,
    L: Layer<
        ParentCanvas = <R::ParentProtocol as Protocol>::Canvas,
        ChildCanvas = <R::ChildProtocol as Protocol>::Canvas,
    >,
{
    fn composite_to(
        &self,
        encoding: &mut <L::ParentCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<L::ParentCanvas>,
    ) -> Vec<ComposableUnadoptedLayer<L::ParentCanvas>> {
        let no_relayout_token = self.mark.assume_not_needing_layout();
        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        let paint_cache = inner_reborrow
            .layout_cache_mut(no_relayout_token)
            .expect("Layer should only be composited after they are laid out")
            .paint_cache
            .as_mut()
            .expect("Layer should only be composited after they are painted");
        if let Some(CachedCompositionFunctionTable {
            composite_to_cache,
            composite_from_cache_to,
        }) = L::CACHED_COMPOSITION_FUNCTION_TABLE
        {
            if let Err(no_recomposite_token) = self.layer_mark.needs_composite() {
                paint_cache.composite_results_ref(no_recomposite_token)
            } else {
                None
            };

            let composite_results = self
                .layer_mark
                .needs_composite()
                .err()
                .and_then(|token| paint_cache.composite_results_ref(token));

            let composite_results = if let Some(composite_results) = composite_results {
                composite_from_cache_to(
                    encoding,
                    &composite_results.cached_composition,
                    composition_config,
                );
                composite_results
            } else {
                let mut iter = CachingChildLayerProducingIterator {
                    paint_results: &paint_cache.paint_results,
                    key: todo!(),
                    // key: inner_reborrow.layer.key().map(Arc::as_ref),
                    unadopted_layers: Vec::new(),
                };
                let results = composite_to_cache(&mut iter);
                composite_from_cache_to(encoding, &results, composition_config);
                paint_cache.insert_composite_results(CompositeResults {
                    unadopted_layers: iter.unadopted_layers,
                    cached_composition: results,
                })
            };

            return composite_results
                .unadopted_layers
                .iter()
                .map(|unadopted_layer| ComposableUnadoptedLayer {
                    config: L::transform_config(composition_config, &unadopted_layer.config),
                    adopter_key: unadopted_layer.adopter_key.clone(),
                    layer: unadopted_layer.layer.clone(),
                })
                .collect();
        } else {
            let mut iter = NonCachingChildLayerProducingIterator {
                paint_results: &paint_cache.paint_results,
                key: todo!(), //inner_reborrow.layer.key().map(Arc::as_ref),
                unadopted_layers: Vec::new(),
                composition_config,
                transform_config: L::transform_config,
            };
            <L as Layer>::composite_to(encoding, &mut iter, composition_config);
            return iter.unadopted_layers;
        }
    }
}

pub(crate) mod composite_private {

    use super::*;

    pub trait ChildLayerCompositeExt<PC: Canvas> {
        fn composite_to(
            &self,
            encoding: &mut PC::Encoding,
            composition_config: &LayerCompositionConfig<PC>,
        ) -> Vec<ComposableUnadoptedLayer<PC>>;
    }

    impl<R, L> ChildLayerCompositeExt<L::ParentCanvas> for RenderObject<R>
    where
        R: Render<LayerOrUnit = L>,
        R::ChildProtocol: LayerProtocol,
        R::ParentProtocol: LayerProtocol,
        L: Layer<
            ParentCanvas = <R::ParentProtocol as Protocol>::Canvas,
            ChildCanvas = <R::ChildProtocol as Protocol>::Canvas,
        >,
    {
        fn composite_to(
            &self,
            encoding: &mut <L::ParentCanvas as Canvas>::Encoding,
            composition_config: &LayerCompositionConfig<L::ParentCanvas>,
        ) -> Vec<ComposableUnadoptedLayer<L::ParentCanvas>> {
            self.composite_to(encoding, composition_config)
        }
    }

    pub trait AnyLayerCompositeExt {
        fn recomposite(&self);
    }
}
