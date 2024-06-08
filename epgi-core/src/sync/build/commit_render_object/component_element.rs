use crate::{
    foundation::ArrayContainer,
    sync::{LaneScheduler, RenderObjectCommitResult},
    tree::{
        ArcChildElementNode, ArcElementContextNode, ChildRenderObjectsUpdateCallback, Element,
        ElementBase, ElementImpl, ElementNode, ImplElement,
    },
};

use super::ImplCommitRenderObject;

impl<E, const PROVIDE_ELEMENT: bool> ImplCommitRenderObject<E>
    for ElementImpl<false, PROVIDE_ELEMENT>
where
    E: Element<
        ChildProtocol = <E as ElementBase>::ParentProtocol,
        ChildContainer = ArrayContainer<1>,
        Impl = Self,
    >,
    Self: ImplElement<E, OptionArcRenderObject = ()>,
{
    fn visit_commit_render_object<'batch>(
        _element_node: &ElementNode<E>,
        _render_object: (),
        [render_object_change]: [RenderObjectCommitResult<E::ChildProtocol>; 1],
        _lane_scheduler: &'batch LaneScheduler,
        _scope: &rayon::Scope<'batch>,
        _self_rebuild_suspended: bool,
    ) -> RenderObjectCommitResult<E::ParentProtocol> {
        render_object_change
    }

    fn rebuild_success_commit_render_object<'batch>(
        _element: &E,
        _widget: &E::ArcWidget,
        _shuffle: Option<ChildRenderObjectsUpdateCallback<E::ChildContainer, E::ChildProtocol>>,
        [child]: &mut [ArcChildElementNode<E::ChildProtocol>; 1],
        render_object: Option<()>,
        [render_object_change]: [RenderObjectCommitResult<E::ChildProtocol>; 1],
        _element_context: &ArcElementContextNode,
        _lane_scheduler: &'batch LaneScheduler,
        _scope: &rayon::Scope<'batch>,
        _is_new_widget: bool,
    ) -> ((), RenderObjectCommitResult<E::ParentProtocol>) {
        if render_object.is_some() {
            // Previously this node is not suspended, so we just report what our child report
            ((), render_object_change)
        } else {
            // Previously this node is suspended and now it resumed, we need to query our child and report ourself as having a new render object
            use RenderObjectCommitResult::*;
            let render_object_change = match render_object_change {
                Keep { .. } => child
                    .get_current_subtree_render_object()
                    .map_or_else(|| render_object_change, |render_object| New(render_object)),
                New(_) | Suspend => render_object_change,
            };
            ((), render_object_change)
        }
    }

    fn rebuild_suspend_commit_render_object(
        _render_object: Option<()>,
    ) -> RenderObjectCommitResult<E::ParentProtocol> {
        RenderObjectCommitResult::Suspend
    }

    fn inflate_success_commit_render_object(
        _element: &E,
        _widget: &E::ArcWidget,
        [_child]: &mut [ArcChildElementNode<E::ChildProtocol>; 1],
        [render_object_change]: [RenderObjectCommitResult<E::ChildProtocol>; 1],
        _element_context: &ArcElementContextNode,
        _lane_scheduler: &LaneScheduler,
    ) -> ((), RenderObjectCommitResult<E::ParentProtocol>) {
        ((), render_object_change)
    }

    fn detach_render_object(_render_object: &()) {}
}
