use crate::{
    common::{Element, Render, RenderObject},
    foundation::{PaintingContext, Protocol},
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
        transformation: &<<R::Element as Element>::SelfProtocol as Protocol>::CanvasTransformation,
        paint_ctx: &mut impl PaintingContext<
            Canvas = <<R::Element as Element>::SelfProtocol as Protocol>::Canvas,
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
    pub trait ChildRenderObjectPaintExt<SP: Protocol> {
        fn paint(
            &self,
            transformation: &SP::CanvasTransformation,
            paint_ctx: &mut <SP::Canvas as Canvas>::DefaultPaintingContext,
        );

        fn paint_scan(
            &self,
            transformation: &SP::CanvasTransformation,
            paint_ctx: &mut <SP::Canvas as Canvas>::DefaultPaintingScanner,
        );
    }

    impl<R> ChildRenderObjectPaintExt<<R::Element as Element>::SelfProtocol> for RenderObject<R>
    where
        R: Render,
    {
        fn paint(
            &self,
            transformation: &<<R::Element as Element>::SelfProtocol as Protocol>::CanvasTransformation,
            paint_ctx: &mut <<<R::Element as Element>::SelfProtocol as Protocol>::Canvas as Canvas>::DefaultPaintingContext,
        ) {
            self.paint(transformation, paint_ctx)
        }

        fn paint_scan(
            &self,
            transformation: &<<R::Element as Element>::SelfProtocol as Protocol>::CanvasTransformation,
            paint_ctx: &mut <<<R::Element as Element>::SelfProtocol as Protocol>::Canvas as Canvas>::DefaultPaintingScanner,
        ) {
            self.paint(transformation, paint_ctx)
        }
    }
}
