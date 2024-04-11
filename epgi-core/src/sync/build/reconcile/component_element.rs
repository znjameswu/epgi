use crate::{
    foundation::ArrayContainer,
    scheduler::BuildScheduler,
    sync::SubtreeRenderObjectChange,
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
        _self_rebuild_suspended: bool,
        _scope: &rayon::Scope<'_>,
        _build_scheduler: &BuildScheduler,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        render_object_change
    }

    fn rebuild_success_commit(
        _element: &E,
        _widget: &E::ArcWidget,
        _shuffle: Option<ChildRenderObjectsUpdateCallback<E::ChildContainer, E::ChildProtocol>>,
        [_child]: &[ArcChildElementNode<E::ChildProtocol>; 1],
        _render_object: (),
        [render_object_change]: [SubtreeRenderObjectChange<E::ChildProtocol>; 1],
        _element_context: &ArcElementContextNode,
        _is_new_widget: bool,
    ) -> ((), SubtreeRenderObjectChange<E::ParentProtocol>) {
        ((), render_object_change)
    }

    fn rebuild_suspend_commit(_render_object: ()) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        SubtreeRenderObjectChange::Suspend
    }

    fn inflate_success_commit(
        _element: &E,
        _widget: &E::ArcWidget,
        _element_context: &ArcElementContextNode,
        [render_object_change]: [SubtreeRenderObjectChange<E::ChildProtocol>; 1],
    ) -> ((), SubtreeRenderObjectChange<E::ParentProtocol>) {
        ((), render_object_change)
    }
}
