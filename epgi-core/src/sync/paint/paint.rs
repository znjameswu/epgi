use crate::{
    foundation::{Canvas, Identity, PaintContext, Protocol},
    sync::TreeScheduler,
    tree::{ArcAnyLayer, Element, PerformLayerPaint, Render, RenderObject},
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
            .and_then(|x| x.layout_results(&self.element_context))
        else {
            panic!("Paint should only be called after layout has finished")
        };

        if let Some(PerformLayerPaint {
            get_layer_or_insert,
            get_layer: _,
        }) = R::PERFORM_LAYER_PAINT
        {
            paint_ctx.with_layer(|transform_parent| {
                if self.element_context.needs_repaint() {
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
        let Some(layout_results) = inner_reborrow
            .cache
            .as_ref()
            .and_then(|x| x.layout_results(&self.element_context))
        else {
            panic!("Paint should only be called after layout has finished")
        };
        if let Some(PerformLayerPaint {
            get_layer_or_insert: _,
            get_layer,
        }) = R::PERFORM_LAYER_PAINT
        {
            let layer = get_layer(&mut inner_reborrow.render).clone();

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
