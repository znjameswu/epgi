use crate::{
    foundation::{Canvas, Identity, PaintContext, Parallel, Protocol},
    scheduler::get_current_scheduler,
    sync::TreeScheduler,
    tree::{
        ArcAnyLayer, AscRenderContextNode, Element, PerformLayerPaint, Render, RenderContextNode,
        RenderObject, RenderObjectInner,
    },
};

impl TreeScheduler {
    pub(crate) fn perform_paint(&self) -> ArcAnyLayer {
        todo!()
        // self.root_render_object.repaint()
    }
}

impl<R> RenderObject<R>
where
    R: Render,
{
    // fn visit_and_paint(&self) {
    //     let needs_paint = self.context.needs_paint();
    //     let subtree_has_paint = self.context.subtree_has_paint();
    //     let is_repaint_boundary = self.context.is_repaint_boundary();
    //     debug_assert_eq!(
    //         is_repaint_boundary,
    //         R::PERFORM_LAYER_PAINT.is_some(),
    //         "A repaint boundary should always be marked as so in its context node"
    //     );
    //     debug_assert!(
    //         is_repaint_boundary || !needs_paint,
    //         "A paint walk should not encounter a dirty non-boundary node.
    //         Such node should be already painted by an ancester paint sometime earlier in this walk."
    //     );
    //     debug_assert!(
    //         subtree_has_paint || !needs_paint,
    //         "A dirty node should always mark its subtree as dirty"
    //     );
    //     // Paint differs from layout
    //     //
    //     // Layout has side effects and thus the invocation order specified by user must be honored,
    //     // which prohibits us to perform tree walk while perform_layout
    //     //
    //     // Paint is pure (in terms of Render state). Therefore we can perform tree walk inside perform_paint
    //     if subtree_has_paint {
    //         let mut inner = self
    //             .inner
    //             .try_lock()
    //             .expect("Paint phase work units should have exclusive access to each RenderObject");

    //         if needs_paint {
    //             inner.repaint_inner();
    //             self.context.clear_self_needs_paint();
    //         } else {
    //             inner
    //                 .render
    //                 .children()
    //                 .par_for_each(&get_current_scheduler().sync_threadpool, |child| {
    //                     child.visit_and_paint()
    //                 })
    //         }
    //         self.context.clear_subtree_has_paint()
    //     }
    // }
}

impl<R> RenderObjectInner<R>
where
    R: Render,
{
    fn paint_inner(
        &mut self,
        context: &AscRenderContextNode,
        transform: &<<R::Element as Element>::ParentProtocol as Protocol>::Transform,
        paint_ctx: &mut impl PaintContext<
            Canvas = <<R::Element as Element>::ParentProtocol as Protocol>::Canvas,
        >,
    ) {
        let Some(layout_results) = self
            .cache
            .as_ref()
            .and_then(|x| x.layout_results(context))
        else {
            panic!("Paint should only be called after layout has finished")
        };

        if let Some(PerformLayerPaint {
            get_layer_or_insert,
            get_layer: _,
        }) = R::PERFORM_LAYER_PAINT
        {
            todo!()
            // paint_ctx.paint_layered_child(|transform_parent| {
            //     let layer = get_layer_or_insert(
            //         &mut self.render,
            //         &layout_results.size,
            //         transform,
            //         &layout_results.memo,
            //         context,
            //         transform_parent,
            //     );
            //     if context.needs_paint() {

            //         layer.clear();
            //         // let child = child(&inner_reborrow.render);
            //         <<<R::Element as Element>::ChildProtocol as Protocol>::Canvas as Canvas>::paint_layer(
            //             layer.clone().as_arc_parent_layer(),
            //             |paint_scan| {
            //                 paint_scan.paint_children(self.render.children(),&Identity::IDENTITY);
            //             },
            //             |paint_ctx| {
            //                 paint_ctx.paint_children(self.render.children(),&Identity::IDENTITY);
            //             },
            //         );
            //     } else if context.subtree_has_paint() {
            //         self.render.children().par_for_each(&get_current_scheduler().sync_threadpool, |child| child.visit_and_paint());
            //     }
            //     layer.as_arc_child_layer()
            // })
        } else {
            self.render.perform_paint(
                &layout_results.size,
                transform,
                &layout_results.memo,
                paint_ctx,
            );
        }
    }

    // fn repaint_inner(&mut self) {
    //     let Some(PerformLayerPaint {
    //         get_layer_or_insert: _,
    //         get_layer,
    //     }) = R::PERFORM_LAYER_PAINT else {
    //         panic!("Non-RepaintBoundary nodes should not be repainted")
    //     };

    //     let layer = get_layer(&mut self.render)
    //         .expect("Repaint can only be called on nodes with an attached layer")
    //         .clone();

    //     layer.clear();
    //     <<<R::Element as Element>::ChildProtocol as Protocol>::Canvas as Canvas>::paint_layer(
    //         layer.clone().as_arc_parent_layer(),
    //         |paint_scan| {
    //             paint_scan.paint_children(self.render.children(), &Identity::IDENTITY);
    //         },
    //         |paint_ctx| {
    //             paint_ctx.paint_children(self.render.children(), &Identity::IDENTITY);
    //         },
    //     );
    //     // layer.as_any_layer_arc()
    // }
}

pub(crate) mod paint_private {
    use crate::{foundation::Canvas, tree::ArcAnyLayer};

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

    impl<R> ChildRenderObjectPaintExt<<R::Element as Element>::ParentProtocol> for RenderObject<R>
    where
        R: Render,
    {
        fn paint(
            &self,
            transform: &<<R::Element as Element>::ParentProtocol as Protocol>::Transform,
            paint_ctx: &mut <<<R::Element as Element>::ParentProtocol as Protocol>::Canvas as Canvas>::PaintContext<'_>,
        ) {
            let mut inner = self.inner.lock();
            inner.paint_inner(&self.context, transform, paint_ctx);
            self.context.clear_self_needs_paint();
            self.context.clear_subtree_has_paint();
        }

        fn paint_scan(
            &self,
            transform: &<<R::Element as Element>::ParentProtocol as Protocol>::Transform,
            paint_ctx: &mut <<<R::Element as Element>::ParentProtocol as Protocol>::Canvas as Canvas>::PaintScanner<'_>,
        ) {
            let mut inner = self.inner.lock();
            inner.paint_inner(&self.context, transform, paint_ctx);
        }

        // fn visit_and_paint(&self) {
        //     self.visit_and_paint()
        // }
    }

    pub trait AnyRenderObjectRepaintExt {
        // fn repaint(&self) -> ArcAnyLayer;
    }

    impl<R> AnyRenderObjectRepaintExt for RenderObject<R>
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
