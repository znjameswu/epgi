use std::any::Any;

use crate::{
    foundation::{Arc, Asc, Canvas, LayerProtocol, Protocol},
    tree::{
        CachedComposite, CachingChildLayerProducingIterator, ComposableUnadoptedLayer, Composite,
        CompositeResults, ImplRenderObject, LayerCache, LayerCompositionConfig, LayerMark,
        LayerPaint, NonCachingChildLayerProducingIterator, PaintResults, Render, RenderBase,
        RenderImpl, RenderObject,
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
    R: Render,
    R::RenderImpl: ImplComposite<R>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
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

        let composition_results = R::RenderImpl::regenerate_composite_cache(
            &inner_reborrow.render,
            &layer_cache.paint_results,
        );
        let cached_composition = Asc::new(composition_results.cached_composition.clone());
        layer_cache.insert_composite_results(composition_results);
        self.layer_mark.clear_needs_composite();
        return cached_composition;
    }
}

pub trait ChildLayerRenderObjectCompositeExt<PC: Canvas> {
    fn composite_to(
        &self,
        encoding: &mut PC::Encoding,
        composition_config: &LayerCompositionConfig<PC>,
    ) -> Vec<ComposableUnadoptedLayer<PC>>;
}

impl<R> ChildLayerRenderObjectCompositeExt<<R::RenderImpl as ImplAdopterLayer<R>>::AdopterCanvas>
    for RenderObject<R>
where
    R: Render,
    R::RenderImpl: ImplComposite<R>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn composite_to(
        &self,
        encoding: &mut <<R::RenderImpl as ImplAdopterLayer<R>>::AdopterCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<
            <R::RenderImpl as ImplAdopterLayer<R>>::AdopterCanvas,
        >,
    ) -> Vec<ComposableUnadoptedLayer<<R::RenderImpl as ImplAdopterLayer<R>>::AdopterCanvas>> {
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
            R::RenderImpl::composite_with_cache(
                &inner_reborrow.render,
                encoding,
                composition_config,
                &layer_cache.paint_results,
                &composite_results.cached_composition,
            );
            composite_results
        } else {
            let composite_results = R::RenderImpl::composite_without_cache(
                &inner_reborrow.render,
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

pub trait ImplAdopterLayer<R: RenderBase> {
    type AdopterCanvas: Canvas;
}

impl<
        R: RenderBase,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
    > ImplAdopterLayer<R> for RenderImpl<DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, false>
{
    type AdopterCanvas = <R::ParentProtocol as Protocol>::Canvas;
}

impl<
        R: RenderBase,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
    > ImplAdopterLayer<R> for RenderImpl<DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, true>
{
    type AdopterCanvas = <R::ChildProtocol as Protocol>::Canvas;
}

pub trait ImplComposite<R: Render>:
    ImplAdopterLayer<R>
    + ImplRenderObject<
        R,
        LayerMark = LayerMark,
        LayerCache = LayerCache<<R::ChildProtocol as Protocol>::Canvas, Self::CompositionCache>,
    >
// The following extra where clause is added to follow type bounds in AnyLayerRenderObjectPaintExt::repaint_if_attached implementation
// Removal is okay, but will cause great confusion when implementing other traits based on AnyLayerRenderObjectPaintExt
// This extra where clause enforces a consitent bound set on all related trait impl,
// and also possibly generates a more coherent error message for library users, by bail out early at ImplComposite checks,
// rather than later at a complex private intermediary trait way higher up.
where
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    type CompositionCache: Clone + Send + Sync;
    fn regenerate_composite_cache(
        render: &R,
        paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> CompositeResults<<R::ChildProtocol as Protocol>::Canvas, Self::CompositionCache>;

    fn composite_without_cache(
        render: &R,
        encoding: &mut <Self::AdopterCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::AdopterCanvas>,
        paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> CompositeResults<<R::ChildProtocol as Protocol>::Canvas, Self::CompositionCache>;

    fn composite_with_cache(
        render: &R,
        encoding: &mut <Self::AdopterCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::AdopterCanvas>,
        paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
        cache: &Self::CompositionCache,
    );
}

impl<R: Render, const DRY_LAYOUT: bool, const LAYER_PAINT: bool, const ORPHAN_LAYER: bool>
    ImplComposite<R> for RenderImpl<DRY_LAYOUT, LAYER_PAINT, false, ORPHAN_LAYER>
where
    Self: ImplAdopterLayer<R>
        + ImplRenderObject<
            R,
            LayerMark = LayerMark,
            LayerCache = LayerCache<<R::ChildProtocol as Protocol>::Canvas, ()>,
        >,
    R: Composite<Self::AdopterCanvas>,
    R: LayerPaint,
    R::ChildProtocol: LayerProtocol,
    R::ParentProtocol: LayerProtocol,
{
    type CompositionCache = ();

    fn regenerate_composite_cache(
        render: &R,
        paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> CompositeResults<<R::ChildProtocol as Protocol>::Canvas, Self::CompositionCache> {
        panic!("Recomposite can only be called on layer render object with composition caches")
    }

    fn composite_without_cache(
        render: &R,
        encoding: &mut <Self::AdopterCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::AdopterCanvas>,
        paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> CompositeResults<<R::ChildProtocol as Protocol>::Canvas, Self::CompositionCache> {
        let mut iter = NonCachingChildLayerProducingIterator {
            paint_results: &paint_results,
            key: render.layer_key().map(Arc::as_ref),
            unadopted_layers: Vec::new(),
            composition_config,
            transform_config: R::transform_config,
        };
        render.composite_to(encoding, &mut iter, composition_config);
        CompositeResults {
            unadopted_layers: todo!(), //iter.unadopted_layers,
            cached_composition: (),
        }
    }

    fn composite_with_cache(
        render: &R,
        encoding: &mut <Self::AdopterCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::AdopterCanvas>,
        paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
        cache: &Self::CompositionCache,
    ) {
        Self::composite_without_cache(render, encoding, composition_config, paint_results);
    }
}

impl<R: Render, const DRY_LAYOUT: bool, const LAYER_PAINT: bool, const ORPHAN_LAYER: bool, CC>
    ImplComposite<R> for RenderImpl<DRY_LAYOUT, LAYER_PAINT, true, ORPHAN_LAYER>
where
    Self: ImplAdopterLayer<R>
        + ImplRenderObject<
            R,
            LayerMark = LayerMark,
            LayerCache = LayerCache<<R::ChildProtocol as Protocol>::Canvas, CC>,
        >,
    R: CachedComposite<Self::AdopterCanvas, CompositionCache = CC>,
    R: LayerPaint,
    R::ChildProtocol: LayerProtocol,
    R::ParentProtocol: LayerProtocol,
    CC: Clone + Send + Sync,
{
    type CompositionCache = CC;

    fn regenerate_composite_cache(
        render: &R,
        paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> CompositeResults<<R::ChildProtocol as Protocol>::Canvas, Self::CompositionCache> {
        let mut iter = CachingChildLayerProducingIterator {
            paint_results,
            key: render.layer_key().map(Arc::as_ref),
            // key: inner_reborrow.layer.key().map(Arc::as_ref),
            unadopted_layers: Vec::new(),
        };
        let cached_composition = render.composite_into_cache(&mut iter);
        CompositeResults {
            unadopted_layers: iter.unadopted_layers,
            cached_composition,
        }
    }

    fn composite_without_cache(
        render: &R,
        encoding: &mut <Self::AdopterCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::AdopterCanvas>,
        paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> CompositeResults<<R::ChildProtocol as Protocol>::Canvas, Self::CompositionCache> {
        let results = Self::regenerate_composite_cache(render, paint_results);
        render.composite_from_cache_to(encoding, &results.cached_composition, composition_config);
        results
    }

    fn composite_with_cache(
        render: &R,
        encoding: &mut <Self::AdopterCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::AdopterCanvas>,
        paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
        cache: &Self::CompositionCache,
    ) {
        render.composite_from_cache_to(encoding, &cache, composition_config);
    }
}
