use crate::{
    foundation::{Arc, AsIterator, Container, ContainerOf},
    scheduler::get_current_scheduler,
    sync::{LaneScheduler, RenderObjectCommitResult, RenderObjectCommitSummary},
    tree::{
        AnyRenderObject, ArcChildElementNode, ArcChildRenderObject, ArcElementContextNode,
        ChildRenderObjectsUpdateCallback, Element, ElementImpl, ElementNode, ImplElement,
        ImplElementNode, MainlineState, RenderAction, RenderBase, RenderElement, RenderObject,
        RenderObjectSlots,
    },
};

use super::ImplCommitRenderObject;

impl<E, const PROVIDE_ELEMENT: bool> ImplCommitRenderObject<E>
    for ElementImpl<true, PROVIDE_ELEMENT>
where
    E: RenderElement,
    E: Element<Impl = Self>,
    Self: ImplElement<E, OptionArcRenderObject = Option<Arc<RenderObject<E::Render>>>>,
{
    fn visit_commit_render_object<'batch>(
        element_node: &ElementNode<E>,
        render_object: Option<Arc<RenderObject<E::Render>>>,
        render_object_changes: ContainerOf<
            E::ChildContainer,
            RenderObjectCommitResult<E::ChildProtocol>,
        >,
        _lane_scheduler: &'batch LaneScheduler,
        _scope: &rayon::Scope<'batch>,
        self_rebuild_suspended: bool,
    ) -> RenderObjectCommitResult<E::ParentProtocol> {
        debug_assert!(
            render_object.is_none() || !self_rebuild_suspended,
            "Logic error in parameters: \
            This node cannot be in RebuildSuspended state if it has an attached render object"
        );
        let render_object_change_summary =
            RenderObjectCommitResult::summarize(render_object_changes.as_iter());
        if let Some(render_object) = render_object {
            visit_commit_attached(
                element_node,
                render_object,
                render_object_changes,
                render_object_change_summary,
            )
        } else {
            visit_commit_detached(
                element_node,
                render_object_changes,
                render_object_change_summary,
                self_rebuild_suspended,
            )
        }
    }

    fn rebuild_success_commit_render_object<'batch>(
        element: &E,
        widget: &E::ArcWidget,
        shuffle: Option<ChildRenderObjectsUpdateCallback<E::ChildContainer, E::ChildProtocol>>,
        children: &mut ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
        render_object: Option<Self::OptionArcRenderObject>,
        render_object_changes: ContainerOf<
            E::ChildContainer,
            RenderObjectCommitResult<E::ChildProtocol>,
        >,
        element_context: &ArcElementContextNode,
        _lane_scheduler: &'batch LaneScheduler,
        _scope: &rayon::Scope<'batch>,
        is_new_widget: bool,
    ) -> (
        Self::OptionArcRenderObject,
        RenderObjectCommitResult<E::ParentProtocol>,
    ) {
        let was_suspended = render_object.is_none();
        let (new_render_object, change) = if let Some(render_object) = render_object.flatten() {
            rebuild_success_process_attached(
                widget,
                shuffle,
                render_object,
                render_object_changes,
                is_new_widget,
            )
        } else {
            let render_object_change_summary =
                RenderObjectCommitResult::summarize(render_object_changes.as_iter());
            if render_object_change_summary == RenderObjectCommitSummary::HasNewNoSuspend
                || (was_suspended && render_object_change_summary.is_keep_all())
            {
                let render_object = try_create_render_object(
                    element,
                    widget,
                    element_context,
                    children,
                    render_object_changes,
                );

                if let Some(render_object) = render_object {
                    if let Some(layer_render_object) =
                        RenderObject::<E::Render>::try_as_aweak_any_layer_render_object(
                            &render_object,
                        )
                    {
                        get_current_scheduler()
                            .push_layer_render_objects_needing_paint(layer_render_object)
                    }
                    let change = RenderObjectCommitResult::New(render_object.clone());
                    return (Some(render_object), change);
                }
            }
            return (None, RenderObjectCommitResult::Suspend);
        };
        (new_render_object, change)
    }

    fn rebuild_suspend_commit_render_object(
        render_object: Option<Option<Arc<RenderObject<E::Render>>>>,
    ) -> RenderObjectCommitResult<E::ParentProtocol> {
        render_object
            .flatten()
            .map(|render_object| render_object.detach_render_object());
        RenderObjectCommitResult::Suspend
    }

    fn inflate_success_commit_render_object(
        element: &E,
        widget: &E::ArcWidget,
        _children: &mut ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
        render_object_changes: ContainerOf<
            E::ChildContainer,
            RenderObjectCommitResult<E::ChildProtocol>,
        >,
        element_context: &ArcElementContextNode,
        _lane_scheduler: &LaneScheduler,
    ) -> (
        Option<Arc<RenderObject<E::Render>>>,
        RenderObjectCommitResult<E::ParentProtocol>,
    ) {
        let render_object_change_summary =
            RenderObjectCommitResult::summarize(render_object_changes.as_iter());

        debug_assert!(
            !render_object_changes
                .as_iter()
                .any(RenderObjectCommitResult::is_keep_render_object),
            "Fatal logic bug in epgi-core reconcile logic. Please file issue report."
        );

        if render_object_change_summary.is_suspended() {
            return (None, RenderObjectCommitResult::Suspend);
        }

        use RenderObjectCommitResult::*;
        let child_render_objects = render_object_changes.map_collect(|change| match change {
            New(child) => child,
            Suspend | Keep { .. } => {
                panic!("Fatal logic bug in epgi-core reconcile logic. Please file issue report.")
            }
        });

        let new_render_object = Arc::new(RenderObject::<E::Render>::new(
            E::create_render(&element, &widget), //TODO: This could panic
            child_render_objects,
            element_context.clone(),
        ));

        if let Some(layer_render_object) =
            RenderObject::<E::Render>::try_as_aweak_any_layer_render_object(&new_render_object)
        {
            get_current_scheduler().push_layer_render_objects_needing_paint(layer_render_object)
        }

        let change = RenderObjectCommitResult::New(new_render_object.clone());

        (Some(new_render_object), change)
    }

    fn detach_render_object(render_object: &Option<Arc<RenderObject<E::Render>>>) {
        render_object
            .as_ref()
            .map(|render_object| render_object.detach_render_object());
    }
}

#[inline(always)]
fn visit_commit_attached<E, const PROVIDE_ELEMENT: bool>(
    element_node: &ElementNode<E>,
    render_object: Arc<RenderObject<E::Render>>,
    render_object_changes: ContainerOf<
        E::ChildContainer,
        RenderObjectCommitResult<E::ChildProtocol>,
    >,
    render_object_change_summary: RenderObjectCommitSummary,
) -> RenderObjectCommitResult<E::ParentProtocol>
where
    E: RenderElement,
    E: Element<Impl = ElementImpl<true, PROVIDE_ELEMENT>>,
    ElementImpl<true, PROVIDE_ELEMENT>:
        ImplElementNode<E, OptionArcRenderObject = Option<Arc<RenderObject<E::Render>>>>,
{
    use RenderObjectCommitSummary::*;
    match render_object_change_summary {
        KeepAll {
            propagated_render_action,
            descendant_has_action,
        } => {
            let render_action =
                render_object.mark_render_action(propagated_render_action, descendant_has_action);
            return RenderObjectCommitResult::Keep {
                // Absorb on boundaries.
                propagated_render_action: render_action,
                subtree_has_action: descendant_has_action,
            };
        }
        HasNewNoSuspend => {
            let render_action = render_object
                .mark_render_action(Some(RenderAction::Relayout), Some(RenderAction::Relayout));
            render_object.update(|_render, children| {
                update_children::<E::Render>(
                    children,
                    None,
                    render_object_changes,
                    render_object_change_summary,
                )
            });
            return RenderObjectCommitResult::Keep {
                propagated_render_action: render_action,
                subtree_has_action: Some(RenderAction::Relayout),
            };
        }
        HasSuspended => {
            // render_object.detach_render_object();
            let mut snapshot = element_node.snapshot.lock();
            let state = snapshot
                .inner
                .mainline_mut()
                .expect("An unmounted element node should not be reachable by a rebuild!")
                .state
                .as_mut()
                .expect(
                    "State corrupted. \
                        This node has been previously designated to visit by a sync batch. \
                        However, when the visit returns, \
                        it found the sync state has been occupied.",
                );
            use MainlineState::*;
            match state {
                Ready { render_object, .. } => {
                    render_object
                        .take() // We detach the render object
                        .map(|render_object| render_object.detach_render_object());
                }
                RebuildSuspended { .. } => panic!(
                    "State corrupted. \
                    This node has been previously visited and found to have attached render object. \
                    However, when the visit returns, \
                    it found the render object has been detached."
                ),
                InflateSuspended { .. } => panic!(
                    "State corrupted. \
                    This node has been previously designated to visit by a sync batch. \
                    However, when the visit returns, \
                    it found the node to be in an suspended inflated state."
                ),
            }
            return RenderObjectCommitResult::Suspend;
        }
    }
}

pub(crate) fn visit_commit_detached<E, const PROVIDE_ELEMENT: bool>(
    element_node: &ElementNode<E>,
    render_object_changes: ContainerOf<
        E::ChildContainer,
        RenderObjectCommitResult<E::ChildProtocol>,
    >,
    render_object_change_summary: RenderObjectCommitSummary,
    self_rebuild_suspended: bool,
) -> RenderObjectCommitResult<E::ParentProtocol>
where
    E: RenderElement,
    E: Element<Impl = ElementImpl<true, PROVIDE_ELEMENT>>,
    ElementImpl<true, PROVIDE_ELEMENT>:
        ImplElementNode<E, OptionArcRenderObject = Option<Arc<RenderObject<E::Render>>>>,
{
    if let RenderObjectCommitSummary::KeepAll { .. } | RenderObjectCommitSummary::HasSuspended =
        render_object_change_summary
    {
        return RenderObjectCommitResult::Suspend;
    };

    if self_rebuild_suspended {
        return RenderObjectCommitResult::Suspend;
    }

    let mut snapshot = element_node.snapshot.lock();
    let snapshot_reborrow = &mut *snapshot;
    let state = &mut snapshot_reborrow
        .inner
        .mainline_mut()
        .expect("An unmounted element node should not be reachable by a rebuild!")
        .state;

    // We perform a "take-modify-insert" operation on the state to avoid using `replace_with` on child render objects
    //
    // The key flaw of any hypothetical `replace_with` implementations would be that
    // the `E::create_render` call could panic inside, and that is an external method supplied by the library users.
    // Moreover, `E::create_render` is hard to make `UnwindSafe` and therefore can't be handled by `catch_unwind`.
    // Moreover, it would also be impossible to migrate `E::create_render` out of the `replace_with` critical region without a significant cost.
    //
    // All `replace_with` occurrence within this crate MUST avoid panic from external implementations.
    // The only exception would be external panics from `HktContainer` implementations,
    // since its implementation is considered advanced while it actually takes considerable stupidity to mis-implement that contract.

    let old_state = state.take().expect(
        "State corrupted. \
                    This node has been previously designated to visit by a sync batch. \
                    However, when the visit returns, \
                    it found the sync state has been occupied.",
    );

    use MainlineState::*;
    let mut new_attached_render_object = None;
    let new_state = match old_state {
        Ready {
            element,
            render_object: None,
            hooks,
            children,
        } => {
            let render_object = try_create_render_object(
                &element,
                &snapshot_reborrow.widget,
                &element_node.context,
                &children,
                render_object_changes,
            );
            new_attached_render_object = render_object.clone();
            MainlineState::Ready {
                element,
                hooks,
                children,
                render_object,
            }
        }
        old_state @ RebuildSuspended { .. } => {
            debug_assert!(
                false,
                "State corrupted. \
                This node has been previously found to have not been in RebuildSuspended state. \
                However, when the visit returns, \
                it found the node to have entered RebuildSuspended state."
            );
            old_state
        }
        Ready {
            render_object: Some(_),
            ..
        } => panic!(
            "State corrupted. \
            This node has been previously found to have been suspended by this visit. \
            However, when the visit returns, \
            it found the node to have resumed."
        ),
        InflateSuspended { .. } => panic!(
            "State corrupted. \
            This node has been previously designated to visit by a sync batch. \
            However, when the visit returns, \
            it found the node to be in an suspended inflated state."
        ),
    };
    *state = Some(new_state);
    drop(snapshot);

    if let Some(new_attached_render_object) = new_attached_render_object {
        if let Some(layer_render_object) =
            RenderObject::<E::Render>::try_as_aweak_any_layer_render_object(
                &new_attached_render_object,
            )
        {
            get_current_scheduler().push_layer_render_objects_needing_paint(layer_render_object)
        }
        return RenderObjectCommitResult::New(new_attached_render_object);
    } else {
        return RenderObjectCommitResult::Suspend;
    }
}

#[inline(always)]
fn rebuild_success_process_attached<E, const PROVIDE_ELEMENT: bool>(
    widget: &E::ArcWidget,
    shuffle: Option<ChildRenderObjectsUpdateCallback<E::ChildContainer, E::ChildProtocol>>,
    render_object: Arc<RenderObject<E::Render>>,
    render_object_changes: ContainerOf<
        E::ChildContainer,
        RenderObjectCommitResult<E::ChildProtocol>,
    >,
    is_new_widget: bool,
) -> (
    Option<Arc<RenderObject<E::Render>>>,
    RenderObjectCommitResult<E::ParentProtocol>,
)
where
    E: RenderElement,
    ElementImpl<true, PROVIDE_ELEMENT>:
        ImplElementNode<E, OptionArcRenderObject = Option<Arc<RenderObject<E::Render>>>>,
{
    let render_object_change_summary =
        RenderObjectCommitResult::summarize(render_object_changes.as_iter());

    use RenderObjectCommitSummary::*;

    if render_object_change_summary.is_suspended() {
        render_object.detach_render_object();
        return (None, RenderObjectCommitResult::Suspend);
    }

    let mut self_render_action = None;

    if shuffle.is_some()
        || !render_object_change_summary.is_keep_all()
        || (is_new_widget && !E::NOOP_UPDATE_RENDER_OBJECT)
    {
        render_object.update(|render, children| {
            if is_new_widget && !E::NOOP_UPDATE_RENDER_OBJECT {
                self_render_action = E::update_render(render, widget);
            }
            update_children::<E::Render>(
                children,
                shuffle,
                render_object_changes,
                render_object_change_summary,
            )
        });
    }

    let (propagated_render_action, descendant_has_action) = if let KeepAll {
        propagated_render_action,
        descendant_has_action,
    } = render_object_change_summary
    {
        (propagated_render_action, descendant_has_action)
    } else {
        (Some(RenderAction::Relayout), Some(RenderAction::Relayout))
    };

    let self_render_action = std::cmp::max(self_render_action, propagated_render_action);
    let subtree_has_action = std::cmp::max(self_render_action, descendant_has_action);

    let propagated_render_action =
        render_object.mark_render_action(self_render_action, descendant_has_action);

    let change = RenderObjectCommitResult::Keep {
        propagated_render_action,
        subtree_has_action,
    };

    return (Some(render_object), change);
}

#[inline(never)]
fn try_create_render_object<E, const PROVIDE_ELEMENT: bool>(
    element: &E,
    widget: &E::ArcWidget,
    element_context: &ArcElementContextNode,
    children: &ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
    render_object_changes: ContainerOf<
        E::ChildContainer,
        RenderObjectCommitResult<E::ChildProtocol>,
    >,
) -> Option<Arc<RenderObject<E::Render>>>
where
    E: RenderElement,
    ElementImpl<true, PROVIDE_ELEMENT>:
        ImplElementNode<E, OptionArcRenderObject = Option<Arc<RenderObject<E::Render>>>>,
{
    let mut suspended = false;
    let option_child_render_objects =
        children.zip_ref_collect(render_object_changes, |child, change| {
            if suspended {
                return None;
            }
            use RenderObjectCommitResult::*;
            match change {
                Keep { .. } => {
                    let child_render_object = child.get_current_subtree_render_object();
                    if child_render_object.is_none() {
                        suspended = true;
                    }
                    child_render_object
                }
                New(child_render_object) => Some(child_render_object),
                Suspend => panic!("Serious logic bug"),
            }
        });

    if suspended {
        None
    } else {
        let new_render_children =
            option_child_render_objects.map_collect(|child| child.expect("Impossible to fail"));
        let new_render_object = Arc::new(RenderObject::<E::Render>::new(
            E::create_render(&element, &widget), //TODO: This could panic
            new_render_children,
            element_context.clone(),
        ));
        Some(new_render_object)
    }
}

#[inline(always)]
pub(super) fn update_children<R: RenderBase>(
    children: &mut ContainerOf<R::ChildContainer, ArcChildRenderObject<R::ChildProtocol>>,
    shuffle: Option<ChildRenderObjectsUpdateCallback<R::ChildContainer, R::ChildProtocol>>,
    render_object_changes: ContainerOf<
        R::ChildContainer,
        RenderObjectCommitResult<R::ChildProtocol>,
    >,
    render_object_change_summary: RenderObjectCommitSummary,
) {
    if let Some(shuffle) = shuffle {
        replace_with::replace_with_or_abort(children, move |children| {
            let slots = (shuffle)(children);
            slots.zip_collect(render_object_changes, |slot, change| {
                use RenderObjectCommitResult::*;
                use RenderObjectSlots::*;
                match (slot, change) {
                    (Reuse(render_object), Keep { .. }) => render_object,
                    (_, New(render_object)) => render_object,
                    (_, Suspend) => panic!(
                        "Fatal logic bug in epgi-core reconcile logic. \
                            Please file issue report."
                    ),
                    (Inflate, Keep { .. }) => panic!(
                        "Render object update callback bug: \
                            Slot requested for a new render object \
                            but the child is not producing one"
                    ),
                }
            })
        })
    } else if !render_object_change_summary.is_keep_all() {
        replace_with::replace_with_or_abort(children, move |children| {
            children.zip_collect(render_object_changes, |child, change| {
                use RenderObjectCommitResult::*;
                match change {
                    Keep { .. } => child,
                    New(render_object) => render_object,
                    Suspend => panic!(
                        "Fatal logic bug in epgi-core reconcile logic. \
                                Please file issue report."
                    ),
                }
            })
        })
    }
}
// impl<R> RenderObjectInnerOld<R>
// where
//     R: Render,
// {
//     // //https://users.rust-lang.org/t/compiler-hint-for-unlikely-likely-for-if-branches/62102/4
//     // #[inline(always)]
//     // fn detach_and_cache_children(
//     //     &mut self,
//     //     shuffle: Option<
//     //         Box<
//     //             dyn FnOnce(
//     //                 <R::ChildContainer as HktContainer>::Container<
//     //                     ArcChildRenderObject<R::ChildProtocol>,
//     //                 >,
//     //             ) -> <R::ChildContainer as HktContainer>::Container<
//     //                 RenderObjectSlots<R::ChildProtocol>,
//     //             >,
//     //         >,
//     //     >,
//     //     render_object_changes: <R::ChildContainer as HktContainer>::Container<
//     //         SubtreeRenderObjectChange<R::ChildProtocol>,
//     //     >,
//     // ) -> <R::ChildContainer as HktContainer>::Container<
//     //     MaybeSuspendChildRenderObject<R::ChildProtocol>,
//     // > {
//     //     self.render.detach();

//     //     let maybe_suspend_child_render_object = if let Some(shuffle) = shuffle {
//     //         let slots = (shuffle)(self.children.map_ref_collect(Clone::clone));
//     //         slots.zip_collect(render_object_changes, |slot, change| {
//     //             use MaybeSuspendChildRenderObject::*;
//     //             use RenderObjectSlots::*;
//     //             use SubtreeRenderObjectChange::*;
//     //             match (slot, change) {
//     //                 (Reuse(render_object), Keep { .. }) => Ready(render_object),
//     //                 (_, New(render_object)) => Ready(render_object),
//     //                 (_, SuspendNew(render_object)) => ElementSuspended(render_object),
//     //                 (Reuse(render_object), SuspendKeep) => ElementSuspended(render_object),
//     //                 (_, Detach) => Detached,
//     //                 (Inflate, Keep { .. } | SuspendKeep) => panic!(
//     //                     "Render object update callback bug: \
//     //                     Slot requested for a new render object \
//     //                     but the child is not producing one"
//     //                 ),
//     //             }
//     //         })
//     //     } else {
//     //         self.children
//     //             .zip_ref_collect(render_object_changes, |child, change| {
//     //                 use MaybeSuspendChildRenderObject::*;
//     //                 use SubtreeRenderObjectChange::*;
//     //                 match change {
//     //                     Keep { .. } => Ready(child.clone()),
//     //                     New(render_object) => Ready(render_object),
//     //                     SuspendKeep => ElementSuspended(child.clone()),
//     //                     SuspendNew(render_object) => ElementSuspended(render_object),
//     //                     Detach => Detached,
//     //                 }
//     //             })
//     //     };
//     //     maybe_suspend_child_render_object
//     // }
// }
