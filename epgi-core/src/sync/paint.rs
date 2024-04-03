use hashbrown::HashSet;

use crate::{
    foundation::{Arc, Canvas, HktContainer, LayerProtocol, PaintContext, Protocol, PtrEq},
    sync::BuildScheduler,
    tree::{
        ArcChildRenderObject, AweakAnyLayerRenderObject, HasLayoutMemo, Hkt, HktLayerCache,
        HktUnit, LayerCache, LayerMark, LayerPaint, Paint, Render, RenderImpl, RenderObject,
        SelectCachedComposite, SelectLayerPaint, TreeNode,
    },
};

use super::{SelectCompositeImpl, SelectLayoutImpl};

impl BuildScheduler {
    pub(crate) fn perform_paint(
        &self,
        layer_render_objects: HashSet<PtrEq<AweakAnyLayerRenderObject>>,
    ) {
        rayon::scope(|scope| {
            for PtrEq(layer_render_object) in layer_render_objects {
                let Some(layer_render_objects) = layer_render_object.upgrade() else {
                    continue;
                };
                scope.spawn(move |_| layer_render_objects.repaint_if_attached());
            }
        })
    }
}

pub trait AnyLayerRenderObjectPaintExt {
    fn repaint_if_attached(&self);
}

impl<R, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool, const ORPHAN_LAYER: bool>
    AnyLayerRenderObjectPaintExt
    for RenderObject<R, DRY_LAYOUT, true, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Render<RenderObject = Self>
        + SelectLayerPaint<true>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn repaint_if_attached(&self) {
        let Err(_token) = self.mark.is_detached() else {
            return;
        };
        let no_relayout_token = self.mark.assume_not_needing_layout();
        let mut inner = self.inner.lock();

        let paint_results = R::paint_layer(&inner.render, &inner.children);
        let layout_cache = inner
            .cache
            .layout_cache_mut(no_relayout_token)
            .expect("Repaint can only be performed after layout has finished");
        layout_cache.layer_cache = Some(LayerCache::new(paint_results, None));
    }
}

pub trait ChildRenderObjectPaintExt<PP: Protocol> {
    /// *Really* paint the render object onto canvas.
    ///
    /// When encountering a clean repaint boundary, this method will call [ChildRenderObjectPaintExt::visit_and_paint] to continue the tree walk.
    ///
    /// This operation will unmark any needs_paint and subtree_has_paint flag.
    fn paint(
        self: Arc<Self>,
        offset: &PP::Offset,
        paint_ctx: &mut <PP::Canvas as Canvas>::PaintContext<'_>,
    );

    /// Scan the render object to prepare for painting.
    ///
    /// When encountering a repaint boundary, whether clean or not, this method will note and bypass it.
    ///
    /// This operation will NOT unmark any needs_paint and subtree_has_paint flag.
    fn paint_scan(
        self: Arc<Self>,
        offset: &PP::Offset,
        paint_ctx: &mut <PP::Canvas as Canvas>::PaintScanner<'_>,
    );
}

trait ChildRenderObjectPaintExtImpl<PP: Protocol> {
    fn paint_impl(
        self: Arc<Self>,
        offset: PP::Offset,
        paint_ctx: &mut impl PaintContext<Canvas = PP::Canvas>,
    );
}

impl<PP, T> ChildRenderObjectPaintExt<PP> for T
where
    T: ChildRenderObjectPaintExtImpl<PP>,
    PP: Protocol,
{
    fn paint(
        self: Arc<Self>,
        offset: &<PP as Protocol>::Offset,
        paint_ctx: &mut <<PP as Protocol>::Canvas as Canvas>::PaintContext<'_>,
    ) {
        self.paint_impl(offset.clone(), paint_ctx)
    }

    fn paint_scan(
        self: Arc<Self>,
        offset: &<PP as Protocol>::Offset,
        paint_ctx: &mut <<PP as Protocol>::Canvas as Canvas>::PaintScanner<'_>,
    ) {
        self.paint_impl(offset.clone(), paint_ctx)
    }
}

impl<
        R,
        const DRY_LAYOUT: bool,
        const LAYER_PAINT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ChildRenderObjectPaintExtImpl<R::ParentProtocol>
    for RenderObject<R, DRY_LAYOUT, LAYER_PAINT, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Render<RenderObject = Self>
        + SelectLayerPaint<LAYER_PAINT>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: SelectPaintImpl<LAYER_PAINT, ORPHAN_LAYER>,
{
    fn paint_impl(
        self: Arc<Self>,
        offset: <R::ParentProtocol as Protocol>::Offset,
        paint_ctx: &mut impl PaintContext<Canvas = <R::ParentProtocol as Protocol>::Canvas>,
    ) {
        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        let token = self.mark.assume_not_needing_layout();
        let Some(cache) = inner_reborrow.cache.layout_cache_mut(token) else {
            panic!("Paint should only be called after layout has finished")
        };
        inner_reborrow.render.paint_into_context(
            &self,
            &cache.layout_results.size,
            &offset,
            &cache.layout_results.memo,
            &inner_reborrow.children,
            paint_ctx,
        );
        cache.paint_offset = Some(offset);
    }
}

pub trait SelectPaintImpl<const LAYER_PAINT: bool, const ORPHAN_LAYER: bool>:
    TreeNode + HasLayoutMemo
{
    fn paint_into_context(
        &self,
        render_object: &Arc<Self::RenderObject>,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) where
        Self: Render;
}

impl<R> SelectPaintImpl<false, false> for R
where
    R: Paint,
{
    fn paint_into_context(
        &self,
        render_object: &Arc<<Self as Render>::RenderObject>,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) where
        Self: Render,
    {
        self.perform_paint(size, offset, memo, children, paint_ctx)
    }
}

impl<R, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool> SelectPaintImpl<true, false> for R
where
    R: Render<RenderObject = RenderObject<R, DRY_LAYOUT, true, CACHED_COMPOSITE, false>>
        + SelectLayerPaint<true>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: SelectCompositeImpl<CACHED_COMPOSITE, false>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn paint_into_context(
        &self,
        render_object: &Arc<<Self as Render>::RenderObject>,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) where
        Self: Render,
    {
        paint_ctx.add_layer(render_object.clone(), |transform| {
            <R::ParentProtocol as LayerProtocol>::compute_layer_transform(&offset, transform)
        });
    }
}

impl<R, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool> SelectPaintImpl<true, true> for R
where
    R: Render<RenderObject = RenderObject<R, DRY_LAYOUT, true, CACHED_COMPOSITE, true>>
        + SelectLayerPaint<true>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: SelectCompositeImpl<CACHED_COMPOSITE, true>,
    R: SelectLayoutImpl<DRY_LAYOUT>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn paint_into_context(
        &self,
        render_object: &Arc<<Self as Render>::RenderObject>,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) where
        Self: Render,
    {
        paint_ctx.add_orphan_layer(render_object.clone(), |transform| {
            <R::ParentProtocol as LayerProtocol>::compute_layer_transform(&offset, transform)
        });
    }
}

pub trait ImplPaint<R: Render> {
    type LayerMark: Default + Send + Sync;
    type HktLayerCache: Hkt;
    fn paint_into_context(
        render: &R,
        render_object: &Arc<R::RenderObject>,
        size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        memo: &R::LayoutMemo,
        children: &<R::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<R::ChildProtocol>,
        >,
        paint_ctx: &mut impl PaintContext<Canvas = <R::ParentProtocol as Protocol>::Canvas>,
    );
}

impl<R: Render, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool, const ORPHAN_LAYER: bool>
    ImplPaint<R> for RenderImpl<R, DRY_LAYOUT, false, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Paint,
{
    type LayerMark = ();

    type HktLayerCache = HktUnit;

    fn paint_into_context(
        render: &R,
        render_object: &Arc<<R as Render>::RenderObject>,
        size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        memo: &R::LayoutMemo,
        children: &<R::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<R::ChildProtocol>,
        >,
        paint_ctx: &mut impl PaintContext<Canvas = <R::ParentProtocol as Protocol>::Canvas>,
    ) {
        render.perform_paint(size, offset, memo, children, paint_ctx)
    }
}

impl<R: Render, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool, const ORPHAN_LAYER: bool>
    ImplPaint<R> for RenderImpl<R, DRY_LAYOUT, true, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    type LayerMark = LayerMark;

    type HktLayerCache = HktLayerCache<<R::ChildProtocol as Protocol>::Canvas>;

    fn paint_into_context(
        render: &R,
        render_object: &Arc<<R as Render>::RenderObject>,
        size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        memo: &R::LayoutMemo,
        children: &<R::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<R::ChildProtocol>,
        >,
        paint_ctx: &mut impl PaintContext<Canvas = <R::ParentProtocol as Protocol>::Canvas>,
    ) {
        todo!()
    }
}
