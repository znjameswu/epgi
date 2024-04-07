use crate::{
    foundation::{Canvas, Protocol},
    tree::{ArcChildRenderObject, ContainerOf, HitTestResults, Render},
};

/// Orphan layers can skip this implementation
pub trait HasHitTestImpl<R: Render> {
    fn hit_test_children(
        render:&R,
        size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        memo: &R::LayoutMemo,
        children: &ContainerOf<R, ArcChildRenderObject<R::ChildProtocol>>,
        results: &mut HitTestResults<<R::ParentProtocol as Protocol>::Canvas>,
    ) -> bool;

    fn hit_test_self(
        render:&R,
        position: &<<R::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
        size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        memo: &R::LayoutMemo,
    ) -> Option<HitTestBehavior>;
}

pub enum HitTestBehavior {
    Transparent,
    DeferToChild,
    Opaque,
}
