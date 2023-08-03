use crate::{
    common::{Element, Render, RenderObject},
    foundation::{Canvas, Protocol},
};

impl<R> RenderObject<R>
where
    R: Render,
{
    fn composite(
        &self,
    ) -> <<<R::Element as Element>::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding {
        todo!()
    }
}
