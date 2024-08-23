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
        element: &mut E,
        widget: &E::ArcWidget,
        _shuffle: Option<ChildRenderObjectsUpdateCallback<E::ChildContainer, E::ChildProtocol>>,
        [child]: &mut [ArcChildElementNode<E::ChildProtocol>; 1],
        render_object: Option<()>,
        [mut render_object_change]: [RenderObjectCommitResult<E::ChildProtocol>; 1],
        _element_context: &ArcElementContextNode,
        _lane_scheduler: &'batch LaneScheduler,
        _scope: &rayon::Scope<'batch>,
        _is_new_widget: bool,
    ) -> ((), RenderObjectCommitResult<E::ParentProtocol>) {
        // If the element has parent data
        if let Some((parent_data, render_action)) = element.generate_parent_data(widget) {
            use RenderObjectCommitResult::*;
            if let New(child_render_object) = &render_object_change {
                child_render_object.set_parent_data(parent_data);
                return ((), render_object_change);
            }

            let child_render_object = child.get_current_subtree_render_object();
            if let Some(child_render_object) = child_render_object {
                child_render_object.set_parent_data(parent_data);
                if render_object.is_none() {
                    if let Keep { .. } = render_object_change {
                        return ((), New(child_render_object));
                    }
                }
            }
            if let Keep {
                propagated_render_action,
                ..
            } = &mut render_object_change
            {
                *propagated_render_action = std::cmp::max(*propagated_render_action, render_action);
            }
            return ((), render_object_change);
        }

        // Normal path (no parent data)
        if render_object.is_some() {
            // Previously this node is not suspended, so we just report what our child report
            ((), render_object_change)
        } else {
            // Previously this node is suspended and now it resumed, we need to query our child and report ourself as having a new render object
            use RenderObjectCommitResult::*;
            let render_object_change = match render_object_change {
                Keep { .. } => child.get_current_subtree_render_object().map_or_else(
                    || render_object_change,
                    |child_render_object| New(child_render_object),
                ),
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
        element: &mut E,
        widget: &E::ArcWidget,
        [_child]: &mut [ArcChildElementNode<E::ChildProtocol>; 1],
        [mut render_object_change]: [RenderObjectCommitResult<E::ChildProtocol>; 1],
        _element_context: &ArcElementContextNode,
        _lane_scheduler: &LaneScheduler,
    ) -> ((), RenderObjectCommitResult<E::ParentProtocol>) {
        if let Some((parent_data, render_action)) = element.generate_parent_data(widget) {
            use RenderObjectCommitResult::*;
            if let New(child_render_object) = &render_object_change {
                child_render_object.set_parent_data(parent_data);
                return ((), render_object_change);
            }
            if let Keep {
                propagated_render_action,
                ..
            } = &mut render_object_change
            {
                debug_assert!(false, "Logic bug in epgi-core: Element being inflated should not have an unchanged child. Please file issue report");
                *propagated_render_action = std::cmp::max(*propagated_render_action, render_action);
            }
            return ((), render_object_change);
        }
        ((), render_object_change)
    }

    fn detach_render_object(_render_object: &()) {}
}
