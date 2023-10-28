use crate::{
    foundation::{Canvas, Protocol},
    tree::{Render, RenderObject},
};

impl<R> RenderObject<R>
where
    R: Render,
{
    fn composite(&self) -> <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Encoding {
        todo!()
    }
}
