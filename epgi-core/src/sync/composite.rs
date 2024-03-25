use std::any::Any;

use crate::{
    foundation::{Arc, Asc, Canvas, ConstBool, LayerProtocol, Protocol},
    tree::{
        CachedComposite, CachedCompositionFunctionTable, CachingChildLayerProducingIterator,
        ComposableUnadoptedLayer, Composite, CompositeResults, HktLayerCache,
        LayerCompositionConfig, LayerMark, LayerPaint, LayerRender,
        NonCachingChildLayerProducingIterator, OrphanLayer, Paint, PaintResults, RenderNew,
        RenderObject, RenderObjectOld, SelectCachedComposite, SelectLayerPaint, SelectOrphanLayer,
        TreeNode,
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

impl<R, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool, const ORPHAN_LAYER: bool>
    AnyLayerRenderObjectCompositeExt
    for RenderObject<R, DRY_LAYOUT, true, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: RenderNew<RenderObject = Self>
        + SelectLayerPaint<true>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
    R: SelectCompositeImpl<CACHED_COMPOSITE, ORPHAN_LAYER>,
{
    fn recomposite_into_cache(&self) -> Asc<dyn Any + Send + Sync> {
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

        let composition_results = inner_reborrow
            .render
            .regenerate_composite_cache(&layer_cache.paint_results);
        let cached_composition = Asc::new(composition_results.cached_composition.clone());
        layer_cache.insert_composite_results(composition_results);
        self.layer_mark.clear_needs_composite();
        return cached_composition;
    }
}

impl<R> AnyLayerRenderObjectCompositeExt for RenderObjectOld<R>
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

impl<R, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool, const ORPHAN_LAYER: bool>
    ChildLayerRenderObjectCompositeExt<<R as SelectOrphanLayer<ORPHAN_LAYER>>::AdopterCanvas>
    for RenderObject<R, DRY_LAYOUT, true, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: RenderNew<RenderObject = Self>
        + SelectLayerPaint<true>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: LayerPaint,
    R: SelectCompositeImpl<CACHED_COMPOSITE, ORPHAN_LAYER>,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
    // R: SelectOrphanLayer<ORPHAN_LAYER>,
{
    fn composite_to(
        &self,
        encoding: &mut <<R as SelectOrphanLayer<ORPHAN_LAYER>>::AdopterCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<
            <R as SelectOrphanLayer<ORPHAN_LAYER>>::AdopterCanvas,
        >,
    ) -> Vec<ComposableUnadoptedLayer<<R as SelectOrphanLayer<ORPHAN_LAYER>>::AdopterCanvas>> {
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

        let composite_results = self
            .layer_mark
            .needs_composite()
            .err()
            .and_then(|token| layer_cache.composite_results_ref(token));

        let composite_results = if let Some(composite_results) = composite_results {
            inner_reborrow.render.composite_with_cache(
                encoding,
                composition_config,
                &layer_cache.paint_results,
                &composite_results.cached_composition,
            );
            composite_results
        } else {
            let composite_results = inner_reborrow.render.composite_without_cache(
                encoding,
                composition_config,
                &layer_cache.paint_results,
            );
            layer_cache.insert_composite_results(composite_results)
        };
        // return composite_results.unadopted_layers.clone();

        // return composite_results
        //     .unadopted_layers
        //     .iter()
        //     .map(|unadopted_layer| ComposableUnadoptedLayer {
        //         config: R::transform_config(composition_config, &unadopted_layer.config),
        //         adopter_key: unadopted_layer.adopter_key.clone(),
        //         layer: unadopted_layer.layer.clone(),
        //     })
        //     .collect();

        todo!()
    }
}

pub trait SelectCompositeImpl<const CACHED_COMPOSITE: bool, const ORPHAN_LAYER: bool>:
    TreeNode + SelectOrphanLayer<ORPHAN_LAYER> + SelectCachedComposite<CACHED_COMPOSITE>
{
    fn regenerate_composite_cache(
        &self,
        paint_results: &PaintResults<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> CompositeResults<<Self::ChildProtocol as Protocol>::Canvas, Self::CompositionCache>;

    fn composite_without_cache(
        &self,
        encoding: &mut <Self::AdopterCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::AdopterCanvas>,
        paint_results: &PaintResults<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> CompositeResults<<Self::ChildProtocol as Protocol>::Canvas, Self::CompositionCache>;

    fn composite_with_cache(
        &self,
        encoding: &mut <Self::AdopterCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::AdopterCanvas>,
        paint_results: &PaintResults<<Self::ChildProtocol as Protocol>::Canvas>,
        cache: &Self::CompositionCache,
    );
}

impl<R> SelectCompositeImpl<false, false> for R
where
    R: Composite<false>,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn regenerate_composite_cache(
        &self,
        paint_results: &PaintResults<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> CompositeResults<<Self::ChildProtocol as Protocol>::Canvas, Self::CompositionCache> {
        panic!("Recomposite can only be called on layer render object with composition caches")
    }

    fn composite_without_cache(
        &self,
        encoding: &mut <Self::AdopterCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::AdopterCanvas>,
        paint_results: &PaintResults<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> CompositeResults<<Self::ChildProtocol as Protocol>::Canvas, Self::CompositionCache> {
        let mut iter = NonCachingChildLayerProducingIterator {
            paint_results: &paint_results,
            key: self.layer_key().map(Arc::as_ref),
            unadopted_layers: Vec::new(),
            composition_config,
            transform_config: R::transform_config,
        };
        self.composite_to(encoding, &mut iter, composition_config);
        CompositeResults {
            unadopted_layers: todo!(), //iter.unadopted_layers,
            cached_composition: (),
        }
    }

    fn composite_with_cache(
        &self,
        encoding: &mut <Self::AdopterCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::AdopterCanvas>,
        paint_results: &PaintResults<<Self::ChildProtocol as Protocol>::Canvas>,
        _cache: &Self::CompositionCache,
    ) {
        self.composite_without_cache(encoding, composition_config, paint_results);
    }
}

impl<R> SelectCompositeImpl<false, true> for R
where
    R: LayerPaint,
    R: OrphanLayer,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn regenerate_composite_cache(
        &self,
        paint_results: &PaintResults<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> CompositeResults<<Self::ChildProtocol as Protocol>::Canvas, Self::CompositionCache> {
        panic!("Recomposite can only be called on layer render object with composition caches")
    }

    fn composite_without_cache(
        &self,
        encoding: &mut <Self::AdopterCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::AdopterCanvas>,
        paint_results: &PaintResults<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> CompositeResults<<Self::ChildProtocol as Protocol>::Canvas, Self::CompositionCache> {
        todo!()
    }

    fn composite_with_cache(
        &self,
        encoding: &mut <Self::AdopterCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::AdopterCanvas>,
        paint_results: &PaintResults<<Self::ChildProtocol as Protocol>::Canvas>,
        cache: &Self::CompositionCache,
    ) {
        self.composite_without_cache(encoding, composition_config, paint_results);
    }
}

impl<R> SelectCompositeImpl<true, false> for R
where
    R: LayerPaint,
    R: CachedComposite,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn regenerate_composite_cache(
        &self,
        paint_results: &PaintResults<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> CompositeResults<<Self::ChildProtocol as Protocol>::Canvas, Self::CompositionCache> {
        let mut iter = CachingChildLayerProducingIterator {
            paint_results,
            key: self.layer_key().map(Arc::as_ref),
            // key: inner_reborrow.layer.key().map(Arc::as_ref),
            unadopted_layers: Vec::new(),
        };
        let cached_composition = self.composite_into_cache(&mut iter);
        CompositeResults {
            unadopted_layers: iter.unadopted_layers,
            cached_composition,
        }
    }

    fn composite_without_cache(
        &self,
        encoding: &mut <Self::AdopterCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::AdopterCanvas>,
        paint_results: &PaintResults<<Self::ChildProtocol as Protocol>::Canvas>,
    ) -> CompositeResults<<Self::ChildProtocol as Protocol>::Canvas, Self::CompositionCache> {
        let results = self.regenerate_composite_cache(paint_results);
        self.composite_from_cache_to(encoding, &results.cached_composition, composition_config);
        results
    }

    fn composite_with_cache(
        &self,
        encoding: &mut <Self::AdopterCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::AdopterCanvas>,
        _paint_results: &PaintResults<<Self::ChildProtocol as Protocol>::Canvas>,
        cache: &Self::CompositionCache,
    ) {
        self.composite_from_cache_to(encoding, &cache, composition_config);
    }
}

impl<R> ChildLayerRenderObjectCompositeExt<<R::ParentProtocol as Protocol>::Canvas>
    for RenderObjectOld<R>
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
