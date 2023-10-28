use crate::{
    foundation::{PaintContext, Protocol},
    sync::TreeScheduler,
    tree::{AscRenderContextNode, Render, RenderObject},
};

impl TreeScheduler {
    pub(crate) fn perform_paint(&self) {}
}

impl<R> RenderObject<R>
where
    R: Render,
{
    fn paint(
        &self,
        context: &AscRenderContextNode,
        transform: &<R::ParentProtocol as Protocol>::Transform,
        paint_ctx: &mut impl PaintContext<Canvas = <R::ParentProtocol as Protocol>::Canvas>,
    ) {
        let mut inner = self.inner.lock();
        let Some(layout_results) = inner
            .cache
            .as_ref()
            .and_then(|x| x.layout_results(context))
        else {
            panic!("Paint should only be called after layout has finished")
        };

        // if let Some(PerformLayerPaint {
        //     get_layer,
        //     get_canvas_transform_ref,
        //     ..
        // }) = R::PERFORM_LAYER_PAINT
        // {
        //     paint_ctx.add_layer(|| ComposableChildLayer {
        //         config: LayerCompositionConfig {
        //             transform: get_canvas_transform_ref(transform).clone(),
        //         },
        //         layer: get_layer(&mut inner.render).as_arc_child_layer(),
        //     })
        // } else {
        //     inner.render.perform_paint(
        //         &layout_results.size,
        //         transform,
        //         &layout_results.memo,
        //         paint_ctx,
        //     );
        // }
    }
}

pub(crate) mod paint_private {
    use crate::foundation::Canvas;

    use super::*;
    pub trait ChildRenderObjectPaintExt<PP: Protocol> {
        /// *Really* paint the render object onto canvas.
        ///
        /// When encountering a clean repaint boundary, this method will call [ChildRenderObjectPaintExt::visit_and_paint] to continue the tree walk.
        ///
        /// This operation will unmark any needs_paint and subtree_has_paint flag.
        fn paint(
            &self,
            transform: &PP::Transform,
            paint_ctx: &mut <PP::Canvas as Canvas>::PaintContext<'_>,
        );

        /// Scan the render object to prepare for painting.
        ///
        /// When encountering a repaint boundary, whether clean or not, this method will note and bypass it.
        ///
        /// This operation will NOT unmark any needs_paint and subtree_has_paint flag.
        fn paint_scan(
            &self,
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
            &self,
            transform: &<R::ParentProtocol as Protocol>::Transform,
            paint_ctx: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::PaintContext<'_>,
        ) {
            self.paint(&self.context, transform, paint_ctx);
        }

        fn paint_scan(
            &self,
            transform: &<R::ParentProtocol as Protocol>::Transform,
            paint_ctx: &mut <<R::ParentProtocol as Protocol>::Canvas as Canvas>::PaintScanner<'_>,
        ) {
            self.paint(&self.context, transform, paint_ctx);
        }

        // fn visit_and_paint(&self) {
        //     self.visit_and_paint()
        // }
    }

    pub trait AnyLayerPaintExt {
        // fn repaint(&self) -> ArcAnyLayer;
    }

    impl<R> AnyLayerPaintExt for RenderObject<R>
    where
        R: Render,
    {
        // fn repaint(&self) -> ArcAnyLayer {
        //     let mut inner = self.inner.lock();
        //     inner.repaint_inner();
        //     todo!()
        // }
    }
}
