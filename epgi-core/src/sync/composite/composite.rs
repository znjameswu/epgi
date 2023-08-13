use crate::{
    foundation::{Canvas, Protocol},
    tree::{Element, Render, RenderObject},
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
