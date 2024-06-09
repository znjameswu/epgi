use crate::{
    foundation::{Arc, Canvas, HktContainer, LayerProtocol, PaintContext, Protocol},
    tree::{
        ArcChildRenderObject, ImplMaybeLayer, ImplRender, LayerCache, LayerPaint, OrphanLayer,
        Paint, Render, RenderImpl, RenderObject,
    },
};

use super::{ImplComposite, ImplHitTest};

pub trait AnyLayerRenderObjectPaintExt {
    fn repaint_if_attached(&self);
}

impl<R> AnyLayerRenderObjectPaintExt for RenderObject<R>
where
    R: Render,
    R::Impl: ImplComposite<R>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn repaint_if_attached(&self) {
        let Err(_token) = self.mark.is_detached() else {
            return;
        };
        let no_relayout_token = self.mark.assert_not_needing_layout();
        let mut inner = self.inner.lock();

        let paint_results = inner.render.paint_layer(&inner.children);
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

impl<R> ChildRenderObjectPaintExtImpl<R::ParentProtocol> for RenderObject<R>
where
    R: Render,
    R::Impl: ImplPaint<R>,
{
    fn paint_impl(
        self: Arc<Self>,
        offset: <R::ParentProtocol as Protocol>::Offset,
        paint_ctx: &mut impl PaintContext<Canvas = <R::ParentProtocol as Protocol>::Canvas>,
    ) {
        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        let token = self.mark.assert_not_needing_layout();
        let Some(cache) = inner_reborrow.cache.layout_cache_mut(token) else {
            panic!("Paint should only be called after layout has finished")
        };
        R::Impl::paint_into_context(
            &mut inner_reborrow.render,
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

pub trait ImplPaint<R: Render<Impl = Self>> {
    fn paint_into_context(
        render: &R,
        render_object: &Arc<RenderObject<R>>,
        size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        memo: &R::LayoutMemo,
        children: &<R::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<R::ChildProtocol>,
        >,
        paint_ctx: &mut impl PaintContext<Canvas = <R::ParentProtocol as Protocol>::Canvas>,
    );
}

impl<
        R: Render<Impl = Self>,
        const SIZED_BY_PARENT: bool,
        const CACHED_COMPOSITE: bool,
        const ORPHAN_LAYER: bool,
    > ImplPaint<R> for RenderImpl<SIZED_BY_PARENT, false, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R: Paint,
{
    fn paint_into_context(
        render: &R,
        _render_object: &Arc<RenderObject<R>>,
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

impl<R: Render<Impl = Self>, const SIZED_BY_PARENT: bool, const CACHED_COMPOSITE: bool> ImplPaint<R>
    for RenderImpl<SIZED_BY_PARENT, true, CACHED_COMPOSITE, false>
where
    Self: ImplRender<R>,
    Self: ImplMaybeLayer<R>,
    Self: ImplHitTest<R>,
    Self: ImplComposite<R>,
    R: LayerPaint,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn paint_into_context(
        _render: &R,
        render_object: &Arc<RenderObject<R>>,
        _size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        _memo: &R::LayoutMemo,
        _children: &<R::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<R::ChildProtocol>,
        >,
        paint_ctx: &mut impl PaintContext<Canvas = <R::ParentProtocol as Protocol>::Canvas>,
    ) {
        paint_ctx.add_layer::<R::ParentProtocol>(render_object.clone(), offset);
    }
}

impl<R: Render<Impl = Self>, const SIZED_BY_PARENT: bool, const CACHED_COMPOSITE: bool> ImplPaint<R>
    for RenderImpl<SIZED_BY_PARENT, true, CACHED_COMPOSITE, true>
where
    Self: ImplRender<R>,
    Self: ImplMaybeLayer<R>,
    Self: ImplHitTest<R>,
    Self: ImplComposite<R>,
    R: LayerPaint,
    R: OrphanLayer,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn paint_into_context(
        render: &R,
        render_object: &Arc<RenderObject<R>>,
        _size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        _memo: &R::LayoutMemo,
        _children: &<R::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<R::ChildProtocol>,
        >,
        paint_ctx: &mut impl PaintContext<Canvas = <R::ParentProtocol as Protocol>::Canvas>,
    ) {
        paint_ctx.add_orphan_layer::<R::ParentProtocol>(
            render_object.clone(),
            R::adopter_key(render).clone(),
            offset,
        );
    }
}
