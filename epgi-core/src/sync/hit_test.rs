use crate::{
    foundation::{Arc, Canvas, ContainerOf, Protocol},
    tree::{
        ArcChildRenderObject, CachedComposite, CompositionCache, FullRender, HitTest,
        HitTestContext, HitTestResult, ImplRender, ImplRenderObject, LayerCache, LayerMark,
        RecordedChildLayer, Render, RenderBase, RenderImpl, RenderObject,
    },
};

pub trait ChildRenderObjectHitTestExt<PC: Canvas> {
    fn hit_test_with(self: Arc<Self>, ctx: &mut HitTestContext<PC>) -> bool;

    fn hit_test_from_adopter_with(self: Arc<Self>, ctx: &mut HitTestContext<PC>) -> bool;
}

impl<R> ChildRenderObjectHitTestExt<<R::ParentProtocol as Protocol>::Canvas> for RenderObject<R>
where
    R: FullRender,
{
    fn hit_test_with(
        self: Arc<Self>,
        ctx: &mut HitTestContext<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        if <R as FullRender>::Impl::ORPHAN_LAYER {
            return false;
        }
        self.really_hit_test_with(ctx)
    }

    fn hit_test_from_adopter_with(
        self: Arc<Self>,
        ctx: &mut HitTestContext<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        self.really_hit_test_with(ctx)
    }
}

impl<R: FullRender> RenderObject<R> {
    fn really_hit_test_with(
        self: Arc<Self>,
        ctx: &mut HitTestContext<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        let inner = self.inner.lock();
        let no_relayout_token = self.mark.assert_not_needing_layout(); // TODO: Do we really need to check this
        let layout_cache = inner
            .cache
            .layout_cache_ref(no_relayout_token)
            .expect("Hit test should not occur before layout");
        // let composition_cache = layout_cache.
        let offset = layout_cache
            .paint_offset
            .as_ref()
            .expect("Hit test should not occur before paint");

        let result = <R as FullRender>::Impl::hit_test(
            &inner.render,
            ctx,
            &layout_cache.layout_results.size,
            offset,
            &layout_cache.layout_results.memo,
            &inner.children,
            layout_cache
                .layer_cache
                .as_ref()
                .and_then(|layer_cache| {
                    <<R as FullRender>::Impl as ImplHitTest<R>>::get_adopted_children(
                        layer_cache,
                        &self.layer_mark,
                    )
                })
                .unwrap_or(&[]),
        );
        drop(inner);
        use HitTestResult::*;
        if result == NotHit {
            return false;
        }
        let self_has_interface = ctx.interface_exist_on::<R>();
        if self_has_interface {
            ctx.push(self as _);
        }
        return result != HitThroughSelf;
    }
}

pub trait ChildLayerRenderObjectHitTestExt<C: Canvas> {
    // fn hit_test_layer(self: Arc<Self>, results: &mut HitTestResults<C>) -> bool;
}

pub trait ImplHitTest<R: Render<Impl = Self>>: ImplRender<R> {
    const ORPHAN_LAYER: bool;

    fn get_adopted_children<'a>(
        layer_cache: &'a <R::Impl as ImplRenderObject<R>>::LayerCache,
        layer_mark: &'a <R::Impl as ImplRenderObject<R>>::LayerMark,
    ) -> Option<&'a [RecordedChildLayer<<<R as RenderBase>::ChildProtocol as Protocol>::Canvas>]>;

    fn hit_test(
        render: &R,
        ctx: &mut HitTestContext<<R::ParentProtocol as Protocol>::Canvas>,
        size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        memo: &R::LayoutMemo,
        children: &ContainerOf<R::ChildContainer, ArcChildRenderObject<R::ChildProtocol>>,
        adopted_children: &[RecordedChildLayer<<R::ChildProtocol as Protocol>::Canvas>],
    ) -> HitTestResult;
}

impl<
        R: Render<Impl = Self>,
        const DRY_LAYOUT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ImplHitTest<R> for RenderImpl<DRY_LAYOUT, false, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    Self: ImplRender<R>,
    R: HitTest,
{
    const ORPHAN_LAYER: bool = ORPHAN_LAYER;

    fn get_adopted_children<'a>(
        _layer_cache: &'a <<R as Render>::Impl as ImplRenderObject<R>>::LayerCache,
        _layer_mark: &'a <R::Impl as ImplRenderObject<R>>::LayerMark,
    ) -> Option<&'a [RecordedChildLayer<<<R as RenderBase>::ChildProtocol as Protocol>::Canvas>]>
    {
        None
    }

    fn hit_test(
        render: &R,
        ctx: &mut HitTestContext<<<R>::ParentProtocol as Protocol>::Canvas>,
        size: &<<R>::ParentProtocol as Protocol>::Size,
        offset: &<<R>::ParentProtocol as Protocol>::Offset,
        memo: &<R>::LayoutMemo,
        children: &ContainerOf<<R>::ChildContainer, ArcChildRenderObject<<R>::ChildProtocol>>,
        adopted_children: &[RecordedChildLayer<<<R>::ChildProtocol as Protocol>::Canvas>],
    ) -> HitTestResult {
        R::hit_test(render, ctx, size, offset, memo, children, adopted_children)
    }
}

impl<R: Render<Impl = Self>, const DRY_LAYOUT: bool, const ORPHAN_LAYER: bool> ImplHitTest<R>
    for RenderImpl<DRY_LAYOUT, true, false, ORPHAN_LAYER>
where
    Self: ImplRender<R>,
    R: HitTest,
    Self: ImplRenderObject<
        R,
        LayerMark = LayerMark,
        LayerCache = LayerCache<<R::ChildProtocol as Protocol>::Canvas, ()>,
    >,
{
    const ORPHAN_LAYER: bool = ORPHAN_LAYER;

    fn get_adopted_children<'a>(
        layer_cache: &'a <<R as Render>::Impl as ImplRenderObject<R>>::LayerCache,
        layer_mark: &'a <R::Impl as ImplRenderObject<R>>::LayerMark,
    ) -> Option<&'a [RecordedChildLayer<<<R as RenderBase>::ChildProtocol as Protocol>::Canvas>]>
    {
        layer_cache
            .composite_results_ref(layer_mark.assert_not_needing_composite())
            .map(|composition_results| composition_results.adopted_layers.as_ref())
    }

    fn hit_test(
        render: &R,
        ctx: &mut HitTestContext<<<R>::ParentProtocol as Protocol>::Canvas>,
        size: &<<R>::ParentProtocol as Protocol>::Size,
        offset: &<<R>::ParentProtocol as Protocol>::Offset,
        memo: &<R>::LayoutMemo,
        children: &ContainerOf<<R>::ChildContainer, ArcChildRenderObject<<R>::ChildProtocol>>,
        adopted_children: &[RecordedChildLayer<<<R>::ChildProtocol as Protocol>::Canvas>],
    ) -> HitTestResult {
        R::hit_test(render, ctx, size, offset, memo, children, adopted_children)
    }
}

impl<R: Render<Impl = Self>, const DRY_LAYOUT: bool, const ORPHAN_LAYER: bool> ImplHitTest<R>
    for RenderImpl<DRY_LAYOUT, true, true, ORPHAN_LAYER>
where
    Self: ImplRender<R>,
    R: HitTest,
    R: CachedComposite,
    Self: ImplRenderObject<
        R,
        LayerMark = LayerMark,
        LayerCache = LayerCache<
            <R::ChildProtocol as Protocol>::Canvas,
            CompositionCache<<R::ChildProtocol as Protocol>::Canvas, R::CompositionMemo>,
        >,
    >,
{
    const ORPHAN_LAYER: bool = ORPHAN_LAYER;

    fn get_adopted_children<'a>(
        layer_cache: &'a <<R as Render>::Impl as ImplRenderObject<R>>::LayerCache,
        layer_mark: &'a <R::Impl as ImplRenderObject<R>>::LayerMark,
    ) -> Option<&'a [RecordedChildLayer<<<R as RenderBase>::ChildProtocol as Protocol>::Canvas>]>
    {
        layer_cache
            .composite_results_ref(layer_mark.assert_not_needing_composite())
            .map(|composition_results| composition_results.adopted_layers.as_ref())
    }

    fn hit_test(
        render: &R,
        ctx: &mut HitTestContext<<<R>::ParentProtocol as Protocol>::Canvas>,
        size: &<<R>::ParentProtocol as Protocol>::Size,
        offset: &<<R>::ParentProtocol as Protocol>::Offset,
        memo: &<R>::LayoutMemo,
        children: &ContainerOf<<R>::ChildContainer, ArcChildRenderObject<<R>::ChildProtocol>>,
        adopted_children: &[RecordedChildLayer<<<R>::ChildProtocol as Protocol>::Canvas>],
    ) -> HitTestResult {
        R::hit_test(render, ctx, size, offset, memo, children, adopted_children)
    }
}

// pub trait ImplGetAdoptedChildren<R: RenderBase>: ImplRenderObject<R> {

// }
