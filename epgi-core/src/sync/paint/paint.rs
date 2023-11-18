use hashbrown::HashSet;

use crate::{
    foundation::{Arc, AsIterator, Canvas, LayerProtocol, PaintContext, Protocol, PtrEq},
    sync::TreeScheduler,
    tree::{
        layer_render_function_table_of, AweakAnyLayerRenderObject, ComposableChildLayer,
        LayerCompositionConfig, LayerRender, LayerRenderFunctionTable, NotDetachedToken,
        PaintCache, Render, RenderObject,
    },
};

impl TreeScheduler {
    pub(crate) fn perform_paint(
        &self,
        layer_render_objects: HashSet<PtrEq<AweakAnyLayerRenderObject>>,
    ) {
        rayon::scope(|scope| {
            for PtrEq(layer_render_object) in layer_render_objects {
                let Some(layer_render_objects) = layer_render_object.upgrade() else {
                    continue;
                };
                layer_render_objects.repaint_if_attached();
            }
            // layer_nodes
            //     .into_iter()
            //     .filter_map(|PtrEq(layer_node)| {
            //         layer_node
            //             .upgrade()
            //             .filter(|layer_node| !layer_node.mark().detached())
            //     })
            //     .for_each(|layer_node| {
            //         scope.spawn(move |_| {
            //             layer_node.repaint();
            //         })
            //     })
            todo!()
        })
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
                paint_ctx,
            );
        }
    }
}

pub(crate) mod paint_private {
    use crate::{foundation::Canvas, tree::LayerRender};

    use super::*;
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

        // /// Walk the render object tree and initiate painting for dirty repaint boundaries.
        // ///
        // /// The method initiate painting by first create a [PaintContext]
        // /// (Sometimes two with one for scanning prior to the real painting for parallelization)
        // /// and delegate the painting to the [PaintContext].
        // /// The [PaintContext] then calls descendants' [ChildRenderObjectPaintExt::paint] method to perform the painting,
        // /// which, when encountering a clean repaint boundary, will call [ChildRenderObjectPaintExt::visit_and_paint] to continue the tree walk.
        // ///
        // /// This operation will unmark any [RenderContextNode::needs_paint] and [RenderContextNode::subtree_has_paint] flag.
        // fn visit_and_paint(&self);
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

        // fn visit_and_paint(&self) {
        //     self.visit_and_paint()
        // }
    }

    pub trait AnyLayerPaintExt {
        fn repaint_if_attached(&self);
    }

    impl<R> AnyLayerPaintExt for RenderObject<R>
    where
        R: LayerRender,
        R::ChildProtocol: LayerProtocol,
        R::ParentProtocol: LayerProtocol,
    {
        fn repaint_if_attached(&self) {
            if let Err(token) = self.mark.is_detached() {
                self.repaint(token)
            }
        }
    }
}
