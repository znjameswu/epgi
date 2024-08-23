use std::any::Any;

use crate::{
    foundation::{Arc, Asc, Canvas, LayerProtocol, Protocol},
    tree::{
        CachedComposite, ChildLayerProducingIterator, Composite, CompositeResults,
        CompositionCache, ImplRenderObject, LayerCache, LayerCompositionConfig, LayerMark,
        LayerPaint, PaintResults, RecordedOrphanLayer, Render, RenderBase, RenderImpl,
        RenderObject,
    },
};

pub trait AnyLayerRenderObjectCompositeExt {
    fn recomposite_into_memo(&self) -> Asc<dyn Any + Send + Sync>;
}

impl<R> AnyLayerRenderObjectCompositeExt for RenderObject<R>
where
    R: Render,
    R::Impl: ImplComposite<R>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn recomposite_into_memo(&self) -> Asc<dyn Any + Send + Sync> {
        let no_relayout_token = self.mark.assert_not_needing_layout();
        let _no_detach_token = self.mark.assert_not_detached();
        let needs_composite = self.layer_mark.needs_composite();
        if let Err(no_recomposite_token) = needs_composite {
            let inner = self.inner.lock();
            let cached_composition = &inner
                .cache
                .layout_cache_ref(no_relayout_token.into())
                .expect("Layer should only be composited after they are laid out")
                .layer_cache
                .as_ref()
                .expect("Layer should only be composited after they are painted")
                .composite_results_ref(no_recomposite_token)
                .expect(
                    "Caching layers that are not marked as dirty should have a compositiong cache",
                )
                .cache;
            return Asc::new(R::Impl::get_composition_memo(cached_composition).clone());
        }

        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        let layer_cache = inner_reborrow
            .cache
            .layout_cache_mut(no_relayout_token.into())
            .expect("Layer should only be composited after they are laid out")
            .layer_cache
            .as_mut()
            .expect("Layer should only be composited after they are painted");

        let (composition_results, _orphan_layers) =
            R::Impl::regenerate_composite_cache(&inner_reborrow.render, &layer_cache.paint_results);
        let arc_memo = Asc::new(R::Impl::get_composition_memo(&composition_results.cache).clone());
        layer_cache.insert_composite_results(composition_results);
        self.layer_mark.clear_needs_composite();
        return arc_memo;
    }
}

pub trait ChildLayerRenderObjectCompositeExt<PC: Canvas> {
    fn composite_to(
        &self,
        encoding: &mut PC::Encoding,
        composition_config: &LayerCompositionConfig<PC>,
    ) -> Vec<RecordedOrphanLayer<PC>>;
}

impl<R> ChildLayerRenderObjectCompositeExt<<R::ParentProtocol as Protocol>::Canvas>
    for RenderObject<R>
where
    R: Render,
    R::Impl: ImplComposite<R>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn composite_to(
        &self,
        encoding: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> Vec<RecordedOrphanLayer<<R::ParentProtocol as Protocol>::Canvas>> {
        let no_relayout_token = self.mark.assert_not_needing_layout();
        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        let layer_cache = inner_reborrow
            .cache
            .layout_cache_mut(no_relayout_token.into())
            .expect("Layer should only be composited after they are laid out")
            .layer_cache
            .as_mut()
            .expect("Layer should only be composited after they are painted");

        let composite_results = self
            .layer_mark
            .needs_composite()
            .err()
            .and_then(|token| layer_cache.composite_results_ref(token));

        let orphan_layers = if let Some(composite_results) = composite_results {
            R::Impl::composite_with_cache(
                &inner_reborrow.render,
                encoding,
                composition_config,
                &layer_cache.paint_results,
                &composite_results.cache,
            )
        } else {
            let (composite_results, orphan_layers) = R::Impl::composite_without_cache(
                &inner_reborrow.render,
                encoding,
                composition_config,
                &layer_cache.paint_results,
            );
            layer_cache.insert_composite_results(composite_results);
            orphan_layers
        };
        // return composite_results.orphan_layers.clone();

        return orphan_layers
            .iter()
            .map(|unadopted_layer| RecordedOrphanLayer {
                config: R::transform_config(composition_config, &unadopted_layer.config),
                adopter_key: unadopted_layer.adopter_key.clone(),
                layer: unadopted_layer.layer.clone(),
            })
            .collect();
    }
}

pub trait ImplComposite<R: RenderBase>:
    ImplRenderObject<
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
    type CompositionMemo: Clone + Send + Sync;
    type CompositionCache: Clone + Send + Sync;

    fn get_composition_memo(cache: &Self::CompositionCache) -> &Self::CompositionMemo;

    fn regenerate_composite_cache(
        render: &R,
        paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> (
        CompositeResults<<R::ChildProtocol as Protocol>::Canvas, Self::CompositionCache>,
        Vec<RecordedOrphanLayer<<R::ChildProtocol as Protocol>::Canvas>>,
    );

    fn composite_without_cache(
        render: &R,
        encoding: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
        paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> (
        CompositeResults<<R::ChildProtocol as Protocol>::Canvas, Self::CompositionCache>,
        Vec<RecordedOrphanLayer<<R::ChildProtocol as Protocol>::Canvas>>,
    );

    fn composite_with_cache(
        render: &R,
        encoding: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
        paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
        cache: &Self::CompositionCache,
    ) -> Vec<RecordedOrphanLayer<<R::ChildProtocol as Protocol>::Canvas>>;
}

impl<
        R: RenderBase,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const ORPHAN_LAYER: bool,
        CC: Canvas,
        PC: Canvas,
    > ImplComposite<R> for RenderImpl<SIZED_BY_PARENT, LAYER_PAINT, false, ORPHAN_LAYER>
where
    R::ParentProtocol: Protocol<Canvas = PC>,
    R::ChildProtocol: Protocol<Canvas = CC>,
    Self: ImplRenderObject<
        R,
        LayerMark = LayerMark,
        LayerCache = LayerCache<<R::ChildProtocol as Protocol>::Canvas, ()>,
    >,
    R: Composite,
    R: LayerPaint,
    R::ChildProtocol: LayerProtocol,
    R::ParentProtocol: LayerProtocol,
{
    type CompositionMemo = ();
    type CompositionCache = ();

    fn get_composition_memo(cache: &Self::CompositionCache) -> &Self::CompositionMemo {
        cache
    }

    fn regenerate_composite_cache(
        _render: &R,
        _paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> (
        CompositeResults<<R::ChildProtocol as Protocol>::Canvas, Self::CompositionCache>,
        Vec<RecordedOrphanLayer<<R::ChildProtocol as Protocol>::Canvas>>,
    ) {
        panic!("Recomposite can only be called on layer render object with composition caches")
    }

    fn composite_without_cache(
        render: &R,
        encoding: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
        paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> (
        CompositeResults<<R::ChildProtocol as Protocol>::Canvas, Self::CompositionCache>,
        Vec<RecordedOrphanLayer<<R::ChildProtocol as Protocol>::Canvas>>,
    ) {
        let mut iter =
            ChildLayerProducingIterator::new(&paint_results, render.layer_key().map(Arc::as_ref));
        render.composite_to(encoding, &mut iter, composition_config);
        (
            CompositeResults {
                adopted_layers: iter.adopted_layers, //iter.orphan_layers,
                cache: (),
            },
            iter.orphan_layers,
        )
    }

    fn composite_with_cache(
        render: &R,
        encoding: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
        paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
        _cache: &Self::CompositionCache,
    ) -> Vec<RecordedOrphanLayer<<R::ChildProtocol as Protocol>::Canvas>> {
        Self::composite_without_cache(render, encoding, composition_config, paint_results).1
    }
}

impl<
        R: RenderBase,
        const SIZED_BY_PARENT: bool,
        const LAYER_PAINT: bool,
        const ORPHAN_LAYER: bool,
        CM,
    > ImplComposite<R> for RenderImpl<SIZED_BY_PARENT, LAYER_PAINT, true, ORPHAN_LAYER>
where
    Self: ImplRenderObject<
        R,
        LayerMark = LayerMark,
        LayerCache = LayerCache<
            <R::ChildProtocol as Protocol>::Canvas,
            CompositionCache<<R::ChildProtocol as Protocol>::Canvas, CM>,
        >,
    >,
    R: CachedComposite<CompositionMemo = CM>,
    R: LayerPaint,
    R::ChildProtocol: LayerProtocol,
    R::ParentProtocol: LayerProtocol,
    CM: Clone + Send + Sync,
{
    type CompositionMemo = CM;
    type CompositionCache = CompositionCache<<R::ChildProtocol as Protocol>::Canvas, CM>;

    fn get_composition_memo(cache: &Self::CompositionCache) -> &Self::CompositionMemo {
        &cache.memo
    }

    fn regenerate_composite_cache(
        render: &R,
        paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> (
        CompositeResults<<R::ChildProtocol as Protocol>::Canvas, Self::CompositionCache>,
        Vec<RecordedOrphanLayer<<R::ChildProtocol as Protocol>::Canvas>>,
    ) {
        let mut iter =
            ChildLayerProducingIterator::new(paint_results, render.layer_key().map(Arc::as_ref));
        let memo = render.composite_into_memo(&mut iter);
        (
            CompositeResults {
                adopted_layers: iter.adopted_layers,
                cache: CompositionCache {
                    orphan_layers: iter.orphan_layers.clone(),
                    memo,
                },
            },
            iter.orphan_layers,
        )
    }

    fn composite_without_cache(
        render: &R,
        encoding: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
        paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
    ) -> (
        CompositeResults<<R::ChildProtocol as Protocol>::Canvas, Self::CompositionCache>,
        Vec<RecordedOrphanLayer<<R::ChildProtocol as Protocol>::Canvas>>,
    ) {
        let results = Self::regenerate_composite_cache(render, paint_results);
        render.composite_from_cache_to(encoding, &results.0.cache.memo, composition_config);
        results
    }

    fn composite_with_cache(
        render: &R,
        encoding: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<<R::ParentProtocol as Protocol>::Canvas>,
        _paint_results: &PaintResults<<R::ChildProtocol as Protocol>::Canvas>,
        cache: &Self::CompositionCache,
    ) -> Vec<RecordedOrphanLayer<<R::ChildProtocol as Protocol>::Canvas>> {
        render.composite_from_cache_to(encoding, &cache.memo, composition_config);
        cache.orphan_layers.clone()
    }
}
