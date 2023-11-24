use crate::{
    foundation::{Canvas, Protocol},
    tree::{HitTestResults, Render, RenderObject},
};

pub trait ChildRenderObjectHitTestExt<PP: Protocol> {
    fn hit_test(
        &self,
        results: &mut HitTestResults,
        coord: &<PP::Canvas as Canvas>::HitTestCoordinate,
    );
}

impl<R> ChildRenderObjectHitTestExt<R::ParentProtocol> for RenderObject<R>
where
    R: Render,
{
    fn hit_test(
        &self,
        results: &mut HitTestResults,
        coord: &<<R::ParentProtocol as Protocol>::Canvas as Canvas>::HitTestCoordinate,
    ) {
        let inner = self.inner.lock();
        R::hit_test(&inner.render, results, coord, &inner.children)
    }
}
