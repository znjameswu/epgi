use crate::{
    foundation::{Canvas, Identity, PaintContext, Parallel, Protocol},
    scheduler::get_current_scheduler,
    sync::TreeScheduler,
    tree::{ArcAnyLayer, Element, PerformLayerPaint, Render, RenderObject, RenderObjectInner},
};

impl TreeScheduler {
    pub(crate) fn perform_paint(&self) -> ArcAnyLayer {
        self.root_render_object.repaint()
    }
}

impl<R> RenderObject<R>
where
    R: Render,
{
    fn paint(
        &self,
        transform: &<<R::Element as Element>::ParentProtocol as Protocol>::Transform,
        paint_ctx: &mut impl PaintContext<
            Canvas = <<R::Element as Element>::ParentProtocol as Protocol>::Canvas,
        >,
    ) {
        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        let Some(layout_results) = inner_reborrow
            .cache
            .as_ref()
            .and_then(|x| x.layout_results(&self.context))
        else {
            panic!("Paint should only be called after layout has finished")
        };

        if let Some(PerformLayerPaint {
            get_layer_or_insert,
            get_layer: _,
        }) = R::PERFORM_LAYER_PAINT
        {
            paint_ctx.with_layer(|transform_parent| {
                if self.context.needs_paint() {
                    let layer = get_layer_or_insert(
                        &mut inner_reborrow.render,
                        &layout_results.size,
                        transform,
                        &layout_results.memo,
                        &self.element_context,
                        transform_parent,
                    )
                    .clone();

                    layer.clear();
                    // let child = child(&inner_reborrow.render);
                    <<<R::Element as Element>::ChildProtocol as Protocol>::Canvas as Canvas>::paint_layer(
                        layer.as_parent_layer_arc(),
                        |mut paint_scan| {
                            paint_scan.paint_children(inner_reborrow.render.children(),&Identity::IDENTITY);
                        },
                        |mut paint_ctx| {
                            paint_ctx.paint_children(inner_reborrow.render.children(),&Identity::IDENTITY);
                        },
                    );
                    self.context.clear_self_needs_paint();
                } else if self.context.subtree_has_paint() {
                    todo!()
                }
            })
        } else {
            inner_reborrow.render.perform_paint(
                &layout_results.size,
                transform,
                &layout_results.memo,
                paint_ctx,
            );
        }
    }

    fn repaint(&self) -> ArcAnyLayer {
        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        // let Some(layout_results) = inner_reborrow
        //     .cache
        //     .as_ref()
        //     .and_then(|x| x.layout_results(&self.element_context))
        // else {
        //     panic!("Repaint should only be called after layout has finished")
        // };
        if let Some(PerformLayerPaint {
            get_layer_or_insert: _,
            get_layer,
        }) = R::PERFORM_LAYER_PAINT
        {
            let layer = get_layer(&mut inner_reborrow.render)
                .expect("Repaint can only be called on nodes with an attached layer")
                .clone();

            layer.clear();
            <<<R::Element as Element>::ChildProtocol as Protocol>::Canvas as Canvas>::paint_layer(
                layer.clone().as_parent_layer_arc(),
                |mut paint_scan| {
                    paint_scan
                        .paint_children(inner_reborrow.render.children(), &Identity::IDENTITY);
                },
                |mut paint_ctx| {
                    paint_ctx.paint_children(inner_reborrow.render.children(), &Identity::IDENTITY);
                },
            );
            layer.as_any_layer_arc()
        } else {
            panic!("Non-RepaintBoundary nodes should not be repainted")
        }
    }

    fn visit_and_paint(&self) {
        let needs_paint = self.context.needs_paint();
        let subtree_has_paint = self.context.subtree_has_paint();
        let is_repaint_boundary = self.context.is_repaint_boundary();
        debug_assert_eq!(
            is_repaint_boundary,
            R::PERFORM_LAYER_PAINT.is_some(),
            "A repaint boundary should always be marked as so in its context node"
        );
        debug_assert!(
            is_repaint_boundary || !needs_paint,
            "A paint walk should not encounter a dirty non-boundary node.
            Such node should be already painted by an ancester paint sometime earlier in this walk."
        );
        debug_assert!(
            subtree_has_paint || !needs_paint,
            "A dirty node should always mark its subtree as dirty"
        );
        // Paint differs from layout
        //
        // Layout has side effects and thus the invocation order specified by user must be honored,
        // which prohibits us to perform tree walk while perform_layout
        //
        // Paint is pure (in terms of Render state). Therefore we can perform tree walk inside perform_paint
        if needs_paint {
            self.repaint();
        } else if subtree_has_paint {
            self.inner
                .lock()
                .render
                .children()
                .par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                    child.visit_and_paint()
                })
        }
    }
}

impl<R> RenderObjectInner<R>
where
    R: Render,
{
    // #[inline(always)]
    // fn perform_repaint_inner(&self) {
    //     if let Some(PerformLayerPaint {
    //         get_layer_or_insert: _,
    //         get_layer,
    //     }) = R::PERFORM_LAYER_PAINT
    //     {
    //         let layer = get_layer(&mut self.render)
    //             .expect("Repaint can only be called on nodes with an attached layer")
    //             .clone();

    //         layer.clear();
    //         <<<R::Element as Element>::ChildProtocol as Protocol>::Canvas as Canvas>::paint_layer(
    //             layer.clone().as_parent_layer_arc(),
    //             |mut paint_scan| {
    //                 paint_scan.paint_children(self.render.children(), &Identity::IDENTITY);
    //             },
    //             |mut paint_ctx| {
    //                 paint_ctx.paint_children(self.render.children(), &Identity::IDENTITY);
    //             },
    //         );
    //         layer.as_any_layer_arc()
    //     } else {
    //         panic!("Non-RepaintBoundary nodes should not be repainted")
    //     }
    // }
}

pub(crate) mod paint_private {
    use crate::{foundation::Canvas, tree::ArcAnyLayer};

    use super::*;
    pub trait ChildRenderObjectPaintExt<PP: Protocol> {
        fn paint(
            &self,
            transform: &PP::Transform,
            paint_ctx: &mut <PP::Canvas as Canvas>::PaintContext<'_>,
        );

        fn paint_scan(
            &self,
            transform: &PP::Transform,
            paint_ctx: &mut <PP::Canvas as Canvas>::PaintScanner<'_>,
        );

        fn visit_and_paint(&self);
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
            self.paint(transform, paint_ctx)
        }

        fn paint_scan(
            &self,
            transform: &<<R::Element as Element>::ParentProtocol as Protocol>::Transform,
            paint_ctx: &mut <<<R::Element as Element>::ParentProtocol as Protocol>::Canvas as Canvas>::PaintScanner<'_>,
        ) {
            self.paint(transform, paint_ctx)
        }

        fn visit_and_paint(&self) {
            self.visit_and_paint()
        }
    }

    pub trait AnyRenderObjectRepaintExt {
        fn repaint(&self) -> ArcAnyLayer;
    }

    impl<R> AnyRenderObjectRepaintExt for RenderObject<R>
    where
        R: Render,
    {
        fn repaint(&self) -> ArcAnyLayer {
            self.repaint()
        }
    }
}
