use hashbrown::HashSet;

use crate::{
    foundation::{
        Arc, AsIterator, Canvas, ConstBool, False, LayerProtocol, PaintContext, Protocol, PtrEq,
        True,
    },
    sync::BuildScheduler,
    tree::{
        layer_render_function_table_of, AweakAnyLayerRenderObject, HktLayerCache, LayerCache,
        LayerPaint, LayerRender, LayerRenderFunctionTable, NotDetachedToken, Paint, Render,
        RenderNew, RenderObject, RenderObjectOld, SelectPaintImpl,
    },
};

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
    R: RenderNew<LayerPaint = True>,
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

impl<R> ChildRenderObjectPaintExt<R::ParentProtocol> for RenderObject<R>
where
    R: RenderNew,
{
    fn paint(
        self: Arc<Self>,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        paint_ctx: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::PaintContext<'_>,
    ) {
        paint(self, offset.clone(), paint_ctx)
    }

    fn paint_scan(
        self: Arc<Self>,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        paint_ctx: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::PaintScanner<'_>,
    ) {
        paint(self, offset.clone(), paint_ctx)
    }
}

fn paint<R: RenderNew>(
    render_object: Arc<RenderObject<R>>,
    offset: <R::ParentProtocol as Protocol>::Offset,
    paint_ctx: &mut impl PaintContext<Canvas = <R::ParentProtocol as Protocol>::Canvas>,
) {
    let mut inner = render_object.inner.lock();
    let inner_reborrow = &mut *inner;
    let token = render_object.mark.assume_not_needing_layout();
    let Some(cache) = inner_reborrow.cache.layout_cache_mut(token) else {
        panic!("Paint should only be called after layout has finished")
    };
    inner_reborrow.render.option_perform_paint(
        &cache.layout_results.size,
        &offset,
        &cache.layout_results.memo,
        &inner_reborrow.children,
        paint_ctx,
    );
    // We need size and memo to determine clip, therefore we have to clone either the Arc or the size or memo.
    R::option_paint_self_as_child_layer(
        &render_object,
        &cache.layout_results.size,
        &offset,
        &cache.layout_results.memo,
        paint_ctx,
    );
    cache.paint_offset = Some(offset);
}

impl<R> ChildRenderObjectPaintExt<R::ParentProtocol> for RenderObjectOld<R>
where
    R: Render,
{
    fn paint(
        self: Arc<Self>,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        paint_ctx: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::PaintContext<'_>,
    ) {
        self._paint(offset, paint_ctx);
    }

    fn paint_scan(
        self: Arc<Self>,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        paint_ctx: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::PaintScanner<'_>,
    ) {
        self._paint(offset, paint_ctx);
    }
}

impl<R> RenderObjectOld<R>
where
    R: Render,
{
    fn _paint(
        self: Arc<Self>,
        offset: &<R::ParentProtocol as Protocol>::Offset,
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
                compute_canvas_transform(offset, transform)
            })
        } else {
            inner_reborrow.render.perform_paint(
                &cache.layout_results.size,
                offset,
                &cache.layout_results.memo,
                &inner_reborrow.children,
                paint_ctx,
            );
        }
    }
}
