use crate::{
    common::{Element, Render, RenderObject},
    foundation::{PaintContext, Protocol},
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
        transformation: &<<R::Element as Element>::ParentProtocol as Protocol>::SelfTransform,
        paint_ctx: impl PaintContext<
            Canvas = <<R::Element as Element>::ParentProtocol as Protocol>::Canvas,
        >,
    ) {
        let mut inner = self.inner.lock();
        let inner_reborrow = &mut *inner;
        let Some(layout_results) = inner_reborrow.cache.layout_results_mut() else {
            panic!("Paint should only be called after layout has finished")
        };
        inner_reborrow.render.perform_paint(
            &layout_results.size,
            transformation,
            &layout_results.memo,
            paint_ctx,
        )
    }
}

pub(crate) mod paint_private {
    use crate::foundation::Canvas;

    use super::*;
    pub trait ChildRenderObjectPaintExt<PP: Protocol> {
        fn paint(
            &self,
            transformation: &PP::SelfTransform,
            paint_ctx: <PP::Canvas as Canvas>::PaintContext<'_>,
        );

        fn paint_scan(
            &self,
            transformation: &PP::SelfTransform,
            paint_ctx: <PP::Canvas as Canvas>::PaintScanner<'_>,
        );
    }

    impl<R> ChildRenderObjectPaintExt<<R::Element as Element>::ParentProtocol> for RenderObject<R>
    where
        R: Render,
    {
        fn paint(
            &self,
            transformation: &<<R::Element as Element>::ParentProtocol as Protocol>::SelfTransform,
            paint_ctx: <<<R::Element as Element>::ParentProtocol as Protocol>::Canvas as Canvas>::PaintContext<'_>,
        ) {
            self.paint(transformation, paint_ctx)
        }

        fn paint_scan(
            &self,
            transformation: &<<R::Element as Element>::ParentProtocol as Protocol>::SelfTransform,
            paint_ctx: <<<R::Element as Element>::ParentProtocol as Protocol>::Canvas as Canvas>::PaintScanner<'_>,
        ) {
            self.paint(transformation, paint_ctx)
        }
    }
}
