use crate::{
    foundation::ArrayContainer,
    scheduler::BuildScheduler,
    sync::SubtreeRenderObjectChange,
    tree::{
        ArcChildElementNode, ArcElementContextNode, ChildRenderObjectsUpdateCallback, Element,
        ElementNode, SelectArcRenderObject,
    },
};

use super::SelectReconcileImpl;

impl<E, const PROVIDE_ELEMENT: bool> SelectReconcileImpl<false, PROVIDE_ELEMENT> for E
where
    E: Element<
            ElementNode = ElementNode<E, false, PROVIDE_ELEMENT>,
            ChildProtocol = Self::ParentProtocol,
            ChildContainer = ArrayContainer<1>,
        > + SelectArcRenderObject<false, OptionArcRenderObject = ()>,
{
    fn visit_commit(
        _element_node: &Self::ElementNode,
        _render_object: (),
        [render_object_change]: [SubtreeRenderObjectChange<Self::ChildProtocol>; 1],
        _self_rebuild_suspended: bool,
        _scope: &rayon::Scope<'_>,
        _build_scheduler: &BuildScheduler,
    ) -> SubtreeRenderObjectChange<Self::ParentProtocol> {
        render_object_change
    }

    fn rebuild_success_commit(
        _element: &Self,
        _widget: &Self::ArcWidget,
        _shuffle: Option<ChildRenderObjectsUpdateCallback<Self>>,
        [_child]: &[ArcChildElementNode<Self::ChildProtocol>; 1],
        _render_object: (),
        [render_object_change]: [SubtreeRenderObjectChange<Self::ChildProtocol>; 1],
        _element_context: &ArcElementContextNode,
        _is_new_widget: bool,
    ) -> ((), SubtreeRenderObjectChange<Self::ParentProtocol>) {
        ((), render_object_change)
    }

    fn rebuild_suspend_commit(
        _render_object: (),
    ) -> SubtreeRenderObjectChange<Self::ParentProtocol> {
        SubtreeRenderObjectChange::Suspend
    }

    fn inflate_success_commit(
        _element: &Self,
        _widget: &Self::ArcWidget,
        _element_context: &ArcElementContextNode,
        [render_object_change]: [SubtreeRenderObjectChange<Self::ChildProtocol>; 1],
    ) -> ((), SubtreeRenderObjectChange<Self::ParentProtocol>) {
        ((), render_object_change)
    }
}
