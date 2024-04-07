use hashbrown::HashSet;

use crate::{
    foundation::{Arc, Canvas, HktContainer, LayerProtocol, PaintContext, Protocol, PtrEq},
    sync::BuildScheduler,
    tree::{
        ArcChildRenderObject, AweakAnyLayerRenderObject, HasLayerPaintImpl, HasOrphanLayerImpl,
        HasPaintImpl, LayerCache, Render, RenderImpl, RenderObject,
    },
};

use super::{ImplAdopterLayer, ImplComposite};

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

impl<R> AnyLayerRenderObjectPaintExt for RenderObject<R>
where
    R: Render,
    R::RenderImpl: ImplComposite<R>,
    R::RenderImpl: HasLayerPaintImpl<R>,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn repaint_if_attached(&self) {
        let Err(_token) = self.mark.is_detached() else {
            return;
        };
        let no_relayout_token = self.mark.assume_not_needing_layout();
        let mut inner = self.inner.lock();

        let paint_results = R::RenderImpl::paint_layer(&inner.render, &inner.children);
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
    R::RenderImpl: ImplPaint<R>,
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
        R::RenderImpl::paint_into_context(
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

pub trait ImplPaint<R: Render> {
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

impl<R: Render, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool, const ORPHAN_LAYER: bool>
    ImplPaint<R> for RenderImpl<R, DRY_LAYOUT, false, CACHED_COMPOSITE, ORPHAN_LAYER>
where
    R::RenderImpl: HasPaintImpl<R>,
{
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
    ) {
        R::RenderImpl::perform_paint(render, size, offset, memo, children, paint_ctx)
    }
}

impl<R: Render, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool> ImplPaint<R>
    for RenderImpl<R, DRY_LAYOUT, true, CACHED_COMPOSITE, false>
where
    // Will this cause inductive cycles? We'll see
    R::RenderImpl: ImplAdopterLayer<R, AdopterCanvas = <R::ParentProtocol as Protocol>::Canvas>
        + ImplComposite<R>,
    R::RenderImpl: HasLayerPaintImpl<R>,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
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
    ) {
        paint_ctx.add_layer(render_object.clone(), |transform| {
            <R::ParentProtocol as LayerProtocol>::compute_layer_transform(&offset, transform)
        });
    }
}

impl<R: Render, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool> ImplPaint<R>
    for RenderImpl<R, DRY_LAYOUT, true, CACHED_COMPOSITE, true>
where
    R::RenderImpl: ImplComposite<R>,
    R::RenderImpl: HasLayerPaintImpl<R>,
    R::RenderImpl: HasOrphanLayerImpl<R>,
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
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
    ) {
        paint_ctx.add_orphan_layer(
            render_object.clone(),
            R::RenderImpl::adopter_key(render).clone(),
            |transform| {
                <R::ParentProtocol as LayerProtocol>::compute_layer_transform(&offset, transform)
            },
        );
    }
}
