use crate::{
    foundation::Protocol,
    tree::{ArcChildRenderObject, ContainerOf, Render},
};

pub trait HasLayoutImpl<R: Render> {
    fn perform_layout(
        render: &mut R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
        children: &ContainerOf<R, ArcChildRenderObject<R::ChildProtocol>>,
    ) -> (<R::ParentProtocol as Protocol>::Size, R::LayoutMemo);
}

pub trait HasDryLayoutImpl<R: Render> {
    fn compute_dry_layout(
        render: &R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
    ) -> <R::ParentProtocol as Protocol>::Size;

    fn compute_layout_memo(
        render: &mut R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
        size: &<R::ParentProtocol as Protocol>::Size,
        children: &ContainerOf<R, ArcChildRenderObject<R::ChildProtocol>>,
    ) -> R::LayoutMemo;
}
