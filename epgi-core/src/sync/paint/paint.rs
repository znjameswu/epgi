use crate::{
    common::{Element, PerformLayerPaint, Render, RenderObject},
    foundation::{Canvas, Identity, PaintContext, Protocol},
    sync::TreeScheduler,
};

impl TreeScheduler {
    pub(crate) fn perform_paint(&self) {
        todo!()
    }
}

impl<R> RenderObject<R>
where
    R: Render,
{
    fn paint(
        &self,
        transform: &<<R::Element as Element>::ParentProtocol as Protocol>::Transform,
        mut paint_ctx: impl PaintContext<
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
            get_layer,
            update_layer,
            child,
        }) = R::PERFORM_LAYER_PAINT
        {
            paint_ctx.with_layer(|transform_parent| {
                if self.element_context.needs_repaint() {
                    let layer = get_layer(
                        &mut inner_reborrow.render,
                        &layout_results.size,
                        transform,
                        &layout_results.memo,
                        &self.element_context,
                        transform_parent,
                    )
                    .clone();

                    layer.clear();
                    let child = child(&inner_reborrow.render);
                    <<<R::Element as Element>::ChildProtocol as Protocol>::Canvas as Canvas>::paint_layer(
                        layer.as_parent_layer_arc(),
                        |paint_scan| {
                            child.paint_scan(&Identity::IDENTITY, paint_scan);
                        },
                        |paint_ctx| {
                            child.paint(&Identity::IDENTITY, paint_ctx);
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
}

pub(crate) mod paint_private {
    use crate::foundation::Canvas;

    use super::*;
    pub trait ChildRenderObjectPaintExt<PP: Protocol> {
        fn paint(
            &self,
            transform: &PP::Transform,
            paint_ctx: <PP::Canvas as Canvas>::PaintContext<'_>,
        );

        fn paint_scan(
            &self,
            transform: &PP::Transform,
            paint_ctx: <PP::Canvas as Canvas>::PaintScanner<'_>,
        );
    }

    impl<R> ChildRenderObjectPaintExt<<R::Element as Element>::ParentProtocol> for RenderObject<R>
    where
        R: Render,
    {
        fn paint(
            &self,
            transform: &<<R::Element as Element>::ParentProtocol as Protocol>::Transform,
            paint_ctx: <<<R::Element as Element>::ParentProtocol as Protocol>::Canvas as Canvas>::PaintContext<'_>,
        ) {
            self.paint(transform, paint_ctx)
        }

        fn paint_scan(
            &self,
            transform: &<<R::Element as Element>::ParentProtocol as Protocol>::Transform,
            paint_ctx: <<<R::Element as Element>::ParentProtocol as Protocol>::Canvas as Canvas>::PaintScanner<'_>,
        ) {
            self.paint(transform, paint_ctx)
        }
    }
}
