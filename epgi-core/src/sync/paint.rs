use hashbrown::HashSet;

use crate::{
    foundation::{Arc, AsIterator, Canvas, LayerProtocol, PaintContext, Protocol, PtrEq},
    sync::BuildScheduler,
    tree::{
        layer_render_function_table_of, AweakAnyLayerRenderObject, ComposableChildLayer,
        LayerCompositionConfig, LayerRender, LayerRenderFunctionTable, NotDetachedToken,
        PaintCache, Render, RenderObject,
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

impl<R> RenderObject<R>
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
        layout_cache.paint_cache = Some(PaintCache::new(paint_results, None));
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
        transform: &PP::Transform,
        paint_ctx: &mut <PP::Canvas as Canvas>::PaintContext<'_>,
    );

    /// Scan the render object to prepare for painting.
    ///
    /// When encountering a repaint boundary, whether clean or not, this method will note and bypass it.
    ///
    /// This operation will NOT unmark any needs_paint and subtree_has_paint flag.
    fn paint_scan(
        self: Arc<Self>,
        transform: &PP::Transform,
        paint_ctx: &mut <PP::Canvas as Canvas>::PaintScanner<'_>,
    );
}

impl<R> ChildRenderObjectPaintExt<R::ParentProtocol> for RenderObject<R>
where
    R: Render,
{
    fn paint(
        self: Arc<Self>,
        transform: &<R::ParentProtocol as Protocol>::Transform,
        paint_ctx: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::PaintContext<'_>,
    ) {
        self._paint(transform, paint_ctx);
    }

    fn paint_scan(
        self: Arc<Self>,
        transform: &<R::ParentProtocol as Protocol>::Transform,
        paint_ctx: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::PaintScanner<'_>,
    ) {
        self._paint(transform, paint_ctx);
    }
}

impl<R> RenderObject<R>
where
    R: Render,
{
    fn _paint(
        self: &Arc<Self>,
        transform: &<R::ParentProtocol as Protocol>::Transform,
        paint_ctx: &mut impl PaintContext<Canvas = <R::ParentProtocol as Protocol>::Canvas>,
    ) {
        let inner = self.inner.lock();
        let token = self.mark.assume_not_needing_layout();
        let Some(cache) = inner.cache.layout_cache_ref(token) else {
            panic!("Paint should only be called after layout has finished")
        };

        if let LayerRenderFunctionTable::LayerRender {
            into_arc_child_layer_render_object,
            get_canvas_transform_ref,
            ..
        } = layer_render_function_table_of::<R>()
        {
            paint_ctx.add_layer(|| ComposableChildLayer {
                config: LayerCompositionConfig {
                    transform: get_canvas_transform_ref(transform).clone(),
                },
                layer: into_arc_child_layer_render_object(self.clone()),
            })
        } else {
            inner.render.perform_paint(
                &cache.layout_results.size,
                transform,
                &cache.layout_results.memo,
                &inner.children,
                paint_ctx,
            );
        }
    }
}
