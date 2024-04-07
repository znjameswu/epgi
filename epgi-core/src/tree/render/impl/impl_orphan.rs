use crate::{
    foundation::{Asc, Key, LayerProtocol},
    tree::Render,
};

pub trait HasOrphanLayerImpl<R: Render>
where
    R::ParentProtocol: LayerProtocol,
    R::ChildProtocol: LayerProtocol,
{
    fn adopter_key(render: &R) -> &Asc<dyn Key>;
}
