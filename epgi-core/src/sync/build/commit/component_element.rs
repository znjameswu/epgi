use crate::{
    foundation::ArrayContainer,
    sync::{LaneScheduler, SubtreeRenderObjectChange},
    tree::{
        ArcChildElementNode, ArcElementContextNode, ChildRenderObjectsUpdateCallback, Element,
        ElementBase, ElementImpl, ElementNode, ImplElement,
    },
};

use super::ImplReconcileCommit;

impl<E, const PROVIDE_ELEMENT: bool> ImplReconcileCommit<E> for ElementImpl<false, PROVIDE_ELEMENT>
where
    E: Element<
        ChildProtocol = <E as ElementBase>::ParentProtocol,
        ChildContainer = ArrayContainer<1>,
        Impl = Self,
    >,
    Self: ImplElement<E, OptionArcRenderObject = ()>,
{
    fn visit_commit(
        _element_node: &ElementNode<E>,
        _render_object: (),
        [render_object_change]: [SubtreeRenderObjectChange<E::ChildProtocol>; 1],
        _lane_scheduler: &LaneScheduler,
        _scope: &rayon::Scope<'_>,
        _self_rebuild_suspended: bool,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        render_object_change
    }

    fn rebuild_success_commit(
        _element: &E,
        _widget: &E::ArcWidget,
        _shuffle: Option<ChildRenderObjectsUpdateCallback<E::ChildContainer, E::ChildProtocol>>,
        [_child]: &mut [ArcChildElementNode<E::ChildProtocol>; 1],
        _render_object: &mut (),
        [render_object_change]: [SubtreeRenderObjectChange<E::ChildProtocol>; 1],
        _element_context: &ArcElementContextNode,
        _lane_scheduler: &LaneScheduler,
        _scope: &rayon::Scope<'_>,
        _is_new_widget: bool,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        render_object_change
    }

    fn rebuild_suspend_commit(_render_object: ()) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        SubtreeRenderObjectChange::Suspend
    }

    fn inflate_success_commit(
        _element: &E,
        _widget: &E::ArcWidget,
        [_child]: &mut [ArcChildElementNode<E::ChildProtocol>; 1],
        [render_object_change]: [SubtreeRenderObjectChange<E::ChildProtocol>; 1],
        _element_context: &ArcElementContextNode,
        _lane_scheduler: &LaneScheduler,
    ) -> ((), SubtreeRenderObjectChange<E::ParentProtocol>) {
        ((), render_object_change)
    }
}
