use crate::{
    common::{Element, Render, RenderObject},
    foundation::Protocol,
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
        &mut self,
        transformation: &<<R::Element as Element>::SelfProtocol as Protocol>::CanvasTransformation,
        canvas: &mut <<R::Element as Element>::SelfProtocol as Protocol>::Canvas,
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
            canvas,
        )
    }
}

pub(crate) mod paint_private {
    use super::*;
    pub trait ChildRenderObjectPaintExt<SP: Protocol> {
        fn paint(&mut self, transformation: &SP::CanvasTransformation, canvas: &mut SP::Canvas);
    }

    impl<R> ChildRenderObjectPaintExt<<R::Element as Element>::SelfProtocol> for RenderObject<R>
    where
        R: Render,
    {
        fn paint(
            &mut self,
            transformation: &<<R::Element as Element>::SelfProtocol as Protocol>::CanvasTransformation,
            canvas: &mut <<R::Element as Element>::SelfProtocol as Protocol>::Canvas,
        ) {
            self.paint(transformation, canvas)
        }
    }
}
