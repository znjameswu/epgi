use hashbrown::HashSet;

use crate::{
    foundation::{
        Arc, AsIterator, Canvas, ConstBool, False, HktContainer, LayerProtocol, PaintContext,
        Protocol, PtrEq, True,
    },
    sync::BuildScheduler,
    tree::{
        layer_render_function_table_of, ArcChildRenderObject, AweakAnyLayerRenderObject,
        HasLayoutMemo, HktLayerCache, LayerCache, LayerPaint, LayerRender,
        LayerRenderFunctionTable, NotDetachedToken, OrphanLayer, Paint, Render, RenderNew,
        RenderObject, RenderObjectOld, SelectCachedComposite, SelectLayerPaint, TreeNode,
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
    R: RenderNew<RenderObject = Self>
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

impl<R> AnyLayerRenderObjectPaintExt for RenderObjectOld<R>
where
    R: LayerRender,
    R::ChildProtocol: LayerProtocol,
    R::ParentProtocol: LayerProtocol,
{
    fn repaint_if_attached(&self) {
        let Err(token) = self.mark.is_detached() else {
            return;
        };
        self.repaint(token);
    }
}

impl<R> RenderObjectOld<R>
where
    R: LayerRender,
    R::ChildProtocol: LayerProtocol,
    R::ParentProtocol: LayerProtocol,
{
    fn repaint(&self, _not_detached_token: NotDetachedToken) {
        let no_relayout_token = self.mark.assume_not_needing_layout();
        let mut inner = self.inner.lock();
        let paint_results =
            <<R::ChildProtocol as Protocol>::Canvas as Canvas>::paint_render_objects(
                inner.children.as_iter().cloned(),
            );
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
        // const CACHED_COMPOSITE: bool,
        // const ORPHAN_LAYER: bool,
    > ChildRenderObjectPaintExtImpl<R::ParentProtocol>
    for RenderObject<R, DRY_LAYOUT, false, false, false>
where
    R: RenderNew<RenderObject = Self> + SelectLayerPaint<false> + SelectCachedComposite<false>,
    R: Paint,
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
        inner_reborrow.render.perform_paint(
            &cache.layout_results.size,
            &offset,
            &cache.layout_results.memo,
            &inner_reborrow.children,
            paint_ctx,
        );
        cache.paint_offset = Some(offset);
    }
}

impl<R, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool>
    ChildRenderObjectPaintExtImpl<R::ParentProtocol>
    for RenderObject<R, DRY_LAYOUT, true, CACHED_COMPOSITE, false>
where
    R: RenderNew<RenderObject = Self>
        + SelectLayerPaint<true>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
    R: SelectCompositeImpl<CACHED_COMPOSITE, false>,
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
        paint_ctx.add_layer(self.clone(), |transform| {
            <R::ParentProtocol as LayerProtocol>::compute_layer_transform(&offset, transform)
        });
        cache.paint_offset = Some(offset);
    }
}

impl<R, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool>
    ChildRenderObjectPaintExtImpl<R::ParentProtocol>
    for RenderObject<R, DRY_LAYOUT, true, CACHED_COMPOSITE, true>
where
    R: RenderNew<RenderObject = Self>
        + SelectLayerPaint<true>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: OrphanLayer,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
    R: SelectCompositeImpl<CACHED_COMPOSITE, true>,
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
        // paint_ctx.add_orphan_layer(self.clone(), |transform| {
        //     <R::ParentProtocol as LayerProtocol>::compute_layer_transform(&offset, transform)
        // });
        cache.paint_offset = Some(offset);
    }
}

impl<R> ChildRenderObjectPaintExtImpl<R::ParentProtocol> for RenderObjectOld<R>
where
    R: Render,
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

        cache.paint_offset = Some(offset.clone());

        if let LayerRenderFunctionTable::LayerRender {
            into_arc_child_layer_render_object,
            compute_canvas_transform,
            ..
        } = layer_render_function_table_of::<R>()
        {
            drop(inner);
            paint_ctx.add_layer(into_arc_child_layer_render_object(self), |transform| {
                compute_canvas_transform(&offset, transform)
            })
        } else {
            inner_reborrow.render.perform_paint(
                &cache.layout_results.size,
                &offset,
                &cache.layout_results.memo,
                &inner_reborrow.children,
                paint_ctx,
            );
        }
    }
}

pub trait SelectPaintImpl<const LAYER_PAINT: bool, const ORPHAN_LAYER: bool>: TreeNode + HasLayoutMemo {
    fn perform_paint(
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
        Self: RenderNew;
}

impl<R> SelectPaintImpl<false, false> for R
where
    R: Paint,
{
    fn perform_paint(
        &self,
        render_object: &Arc<<Self as RenderNew>::RenderObject>,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) where
        Self: RenderNew,
    {
        self.perform_paint(size, offset, memo, children, paint_ctx)
    }
}

// impl<R, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool, const ORPHAN_LAYER: bool>
//     SelectPaintImpl<true> for R
// where
//     R: RenderNew<RenderObject = RenderObject<R, DRY_LAYOUT, true, CACHED_COMPOSITE, ORPHAN_LAYER>>
//         + SelectLayerPaint<true>
//         + SelectCachedComposite<CACHED_COMPOSITE>,
//     R: SelectCompositeImpl<CACHED_COMPOSITE, ORPHAN_LAYER>,
//     R: Paint,
// {
//     fn perform_paint(
//         &self,
//         render_object: &Arc<<Self as RenderNew>::RenderObject>,
//         size: &<Self::ParentProtocol as Protocol>::Size,
//         offset: &<Self::ParentProtocol as Protocol>::Offset,
//         memo: &Self::LayoutMemo,
//         children: &<Self::ChildContainer as HktContainer>::Container<
//             ArcChildRenderObject<Self::ChildProtocol>,
//         >,
//         paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
//     ) where
//         Self: RenderNew,
//     {
//         todo!()
//     }
// }
impl<R, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool>
    SelectPaintImpl<true, false> for R
where
    R: RenderNew<RenderObject = RenderObject<R, DRY_LAYOUT, true, CACHED_COMPOSITE, false>>
        + SelectLayerPaint<true>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: SelectCompositeImpl<CACHED_COMPOSITE, false>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn perform_paint(
        &self,
        render_object: &Arc<<Self as RenderNew>::RenderObject>,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) where
        Self: RenderNew,
    {
        paint_ctx.add_layer(render_object.clone(), |transform| {
            <R::ParentProtocol as LayerProtocol>::compute_layer_transform(&offset, transform)
        });
    }
}

impl<R, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool>
    SelectPaintImpl<true, true> for R
where
    R: RenderNew<RenderObject = RenderObject<R, DRY_LAYOUT, true, CACHED_COMPOSITE, true>>
        + SelectLayerPaint<true>
        + SelectCachedComposite<CACHED_COMPOSITE>,
    R: SelectCompositeImpl<CACHED_COMPOSITE, true>,
    R: SelectLayoutImpl<DRY_LAYOUT>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn perform_paint(
        &self,
        render_object: &Arc<<Self as RenderNew>::RenderObject>,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        children: &<Self::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<Self::ChildProtocol>,
        >,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) where
        Self: RenderNew,
    {
        paint_ctx.add_orphan_layer(render_object.clone(), |transform| {
            <R::ParentProtocol as LayerProtocol>::compute_layer_transform(&offset, transform)
        });
    }
}