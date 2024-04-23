use either::Either;

use crate::{
    foundation::{Arc, Asc, EitherParallel, Protocol},
    nodes::{RenderSuspense, Suspense, SuspenseElement},
    sync::{LaneScheduler, SubtreeRenderObjectChange},
    tree::{
        AnyRenderObject, ArcChildElementNode, ArcChildRenderObject, ArcChildWidget,
        ArcElementContextNode, ChildRenderObjectsUpdateCallback, ElementBase, ElementImpl,
        ElementNode, MainlineState, RenderAction, RenderObject,
    },
};

use super::ImplReconcileCommit;

impl<P: Protocol> ImplReconcileCommit<SuspenseElement<P>> for ElementImpl<true, false> {
    fn visit_commit(
        element_node: &ElementNode<SuspenseElement<P>>,
        render_object: Option<Arc<RenderObject<RenderSuspense<P>>>>,
        render_object_changes: EitherParallel<
            [SubtreeRenderObjectChange<P>; 1],
            [SubtreeRenderObjectChange<P>; 2],
        >,
        lane_scheduler: &LaneScheduler,
        scope: &rayon::Scope<'_>,
        self_rebuild_suspended: bool,
    ) -> SubtreeRenderObjectChange<P> {
        debug_assert!(
            self_rebuild_suspended == false,
            "Suspense itself can never suspend"
        );
        let render_object = render_object.expect("Suspense itself can never suspend");
        use Either::*;
        use SubtreeRenderObjectChange::*;
        match render_object_changes.0 {
            // No update
            Left(
                [Keep {
                    child_render_action,
                    subtree_has_action,
                }],
            )
            | Right(
                [Keep { .. } | Suspend, Keep {
                    child_render_action,
                    subtree_has_action,
                }],
            ) => {
                let render_action =
                    render_object.mark_render_action(child_render_action, subtree_has_action);
                return SubtreeRenderObjectChange::Keep {
                    child_render_action: render_action,
                    subtree_has_action,
                };
            }
            // Normal render object update
            Left([New(child_render_object)])
            | Right([Keep { .. } | Suspend, New(child_render_object)]) => {
                let render_action = render_object
                    .mark_render_action(RenderAction::Relayout, RenderAction::Relayout);
                {
                    let mut inner = render_object.inner.lock();
                    let [old_child_render_object] =
                        std::mem::replace(&mut inner.children, [child_render_object]);
                    old_child_render_object.detach_render_object();
                }
                return SubtreeRenderObjectChange::Keep {
                    child_render_action: render_action,
                    subtree_has_action: RenderAction::Relayout,
                };
            }
            // Primary child suspened. Inflate fallback child and unmount primary.
            Left([Suspend]) => {
                // We choose to read widget right from inside the element node
                // Because requesting widget from context would heavily impact signature of all other visit methods and creates overhead
                // Since suspense is considered to be a rare case / slow path
                // Hence this cost is justified.

                let fallback_widget = {
                    let snapshot = element_node.snapshot.lock();
                    snapshot.widget.fallback.clone()
                };

                let (fallback, change) = inflate_fallback_and_attach_render_object(
                    &render_object,
                    fallback_widget,
                    element_node.context.clone(),
                    lane_scheduler,
                );

                let old_child = {
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

                    let MainlineState::Ready { children, .. } = state else {
                        panic!("Suspense should always be in the Ready state")
                    };
                    replace_suspended_primary_child(children, fallback)
                };

                scope.spawn(|scope| old_child.unmount(scope));

                return change;
            }

            Right([_child_change, Suspend]) => panic!(
                "The fallback component inside this Suspense has suspended. \
                    This is not supposed to happen. \
                    We have not decided to support cascaded suspense propagation."
            ),

            // The primary child has resumed, now we unmount the fallback and remount the primary
            Right([New(child_render_object), fallback_change @ (Keep { .. } | New(_))]) => {
                let change = swap_child_render_object(&render_object, child_render_object, false);

                let old_fallback_child = {
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

                    let MainlineState::Ready { children, .. } = state else {
                        panic!("Suspense should always be in the Ready state")
                    };
                    replace_fallback_child(children)
                };

                scope.spawn(|scope| {
                    old_fallback_child.unmount(scope);
                    if let New(fallback_render_object) = fallback_change {
                        fallback_render_object.detach_render_object();
                    }
                });

                return change;
            }
        };
    }

    fn rebuild_success_commit(
        _element: &SuspenseElement<P>,
        widget: &Asc<Suspense<P>>,
        _shuffle: Option<
            ChildRenderObjectsUpdateCallback<
                <SuspenseElement<P> as ElementBase>::ChildContainer,
                P,
            >,
        >,
        children: &mut EitherParallel<[ArcChildElementNode<P>; 1], [ArcChildElementNode<P>; 2]>,
        render_object: &mut Option<Arc<RenderObject<RenderSuspense<P>>>>,
        render_object_changes: EitherParallel<
            [SubtreeRenderObjectChange<P>; 1],
            [SubtreeRenderObjectChange<P>; 2],
        >,
        element_context: &ArcElementContextNode,
        lane_scheduler: &LaneScheduler,
        scope: &rayon::Scope<'_>,
        _is_new_widget: bool,
    ) -> SubtreeRenderObjectChange<P> {
        debug_assert!(_shuffle.is_none(), "Suspense cannot shuffle its child");
        let render_object = render_object.as_mut().expect("Suspense can never suspend");

        use Either::*;
        use SubtreeRenderObjectChange::*;
        match render_object_changes.0 {
            Left(
                [Keep {
                    child_render_action,
                    subtree_has_action,
                }],
            )
            | Right(
                [Keep { .. } | Suspend, Keep {
                    child_render_action,
                    subtree_has_action,
                }],
            ) => {
                let render_action =
                    render_object.mark_render_action(child_render_action, subtree_has_action);
                return SubtreeRenderObjectChange::Keep {
                    child_render_action: render_action,
                    subtree_has_action,
                };
            }
            Left([New(child_render_object)])
            | Right([Keep { .. } | Suspend, New(child_render_object)]) => {
                let render_action = render_object
                    .mark_render_action(RenderAction::Relayout, RenderAction::Relayout);
                {
                    let mut inner = render_object.inner.lock();
                    let [old_child_render_object] =
                        std::mem::replace(&mut inner.children, [child_render_object]);
                    old_child_render_object.detach_render_object();
                }
                return SubtreeRenderObjectChange::Keep {
                    child_render_action: render_action,
                    subtree_has_action: RenderAction::Relayout,
                };
            }
            Left([Suspend]) => {
                // We choose to read widget right from inside the element node
                // Because requesting widget from context would heavily impact performance of all other visit
                // And suspense is considered to be a rare case / slow path
                // Hence this cost is justified.

                // This is a commit-time effect

                let (fallback, change) = inflate_fallback_and_attach_render_object(
                    &render_object,
                    widget.fallback.clone(),
                    element_context.clone(),
                    lane_scheduler,
                );

                replace_suspended_primary_child(children, fallback);

                return change;
            }

            Right([_child_change, Suspend]) => panic!(
                "The fallback component inside a Suspense has suspended. \
                This is not supposed to happen. \
                We have not decided to support cascaded suspense propagation."
            ),
            Right([New(child_render_object), fallback_change @ (Keep { .. } | New(_))]) => {
                let change = swap_child_render_object(render_object, child_render_object, false);

                let old_fallback_child = replace_fallback_child(children);

                scope.spawn(|scope| {
                    old_fallback_child.unmount(scope);
                    if let New(fallback_render_object) = fallback_change {
                        fallback_render_object.detach_render_object();
                    }
                });

                return change;
            }
        };
    }

    fn rebuild_suspend_commit(
        _render_object: Option<Arc<RenderObject<RenderSuspense<P>>>>,
    ) -> SubtreeRenderObjectChange<P> {
        panic!("Suspense can not suspend on itself")
    }

    fn inflate_success_commit(
        _element: &SuspenseElement<P>,
        widget: &Asc<Suspense<P>>,
        children: &mut EitherParallel<[ArcChildElementNode<P>; 1], [ArcChildElementNode<P>; 2]>,
        render_object_changes: EitherParallel<
            [SubtreeRenderObjectChange<P>; 1],
            [SubtreeRenderObjectChange<P>; 2],
        >,
        element_context: &ArcElementContextNode,
        lane_scheduler: &LaneScheduler,
    ) -> (
        Option<Arc<RenderObject<RenderSuspense<P>>>>,
        SubtreeRenderObjectChange<P>,
    ) {
        let [child_change] = render_object_changes
            .0
            .left()
            .expect("Suspense should always try inflate its primary child first");

        debug_assert!(
            !child_change.is_keep_render_object(),
            "Fatal logic bug in epgi-core reconcile logic. Please file issue report."
        );

        use SubtreeRenderObjectChange::*;
        let (is_suspended, child_render_object) = if let New(child_render_object) = child_change {
            (false, child_render_object)
        } else {
            // Else, the primary child has suspended
            let (fallback, fallback_render_object) = inflate_fallback(
                widget.fallback.clone(),
                element_context.clone(),
                lane_scheduler,
            );
            replace_suspended_primary_child(children, fallback);
            (true, fallback_render_object)
        };

        let new_render_object = Arc::new(RenderObject::new(
            RenderSuspense::new(is_suspended),
            [child_render_object],
            element_context.clone(),
        ));
        return (Some(new_render_object.clone()), New(new_render_object));
    }
}

fn inflate_fallback<P: Protocol>(
    fallback_widget: ArcChildWidget<P>,
    element_context: ArcElementContextNode,
    lane_scheduler: &LaneScheduler,
) -> (ArcChildElementNode<P>, ArcChildRenderObject<P>) {
    let (fallback, change) = fallback_widget.inflate_sync(Some(element_context), lane_scheduler);

    let SubtreeRenderObjectChange::New(fallback_render_object) = change else {
        panic!(
            "The fallback component inside this Suspense has suspended. \
            This is not supposed to happen. \
            We have not decided to support cascaded suspense propagation."
        )
    };

    (fallback, fallback_render_object)
}

fn inflate_fallback_and_attach_render_object<P: Protocol>(
    render_object: &Arc<RenderObject<RenderSuspense<P>>>,
    fallback_widget: ArcChildWidget<P>,
    element_context: ArcElementContextNode,
    lane_scheduler: &LaneScheduler,
) -> (ArcChildElementNode<P>, SubtreeRenderObjectChange<P>) {
    let (fallback, fallback_render_object) =
        inflate_fallback(fallback_widget, element_context, lane_scheduler);

    (
        fallback,
        swap_child_render_object(render_object, fallback_render_object, true),
    )
}

// Either the Suspense is suspend from ready state, or resumed from suspend state
fn swap_child_render_object<P: Protocol>(
    render_object: &Arc<RenderObject<RenderSuspense<P>>>,
    child_render_object: ArcChildRenderObject<P>,
    is_suspended: bool,
) -> SubtreeRenderObjectChange<P> {
    {
        let mut inner = render_object.inner.lock();
        let [_old_child_render_object] =
            std::mem::replace(&mut inner.children, [child_render_object]);
        inner.render.is_suspended = is_suspended;
        // Actually, the child should have already detached its render object
        // if is_suspended {
        //     old_child_render_object.detach_render_object();
        //     // If the change is from suspended to resumed, then we don't need to detach here. Later, unmount will detach the render object.
        // }
    }
    let render_action =
        render_object.mark_render_action(RenderAction::Relayout, RenderAction::Relayout);

    // Suspense will always return Keep during rebuild
    return SubtreeRenderObjectChange::Keep {
        child_render_action: render_action,
        subtree_has_action: RenderAction::Relayout,
    };
}

fn replace_suspended_primary_child<P: Protocol>(
    children: &mut EitherParallel<[ArcChildElementNode<P>; 1], [ArcChildElementNode<P>; 2]>,
    fallback: ArcChildElementNode<P>,
) -> ArcChildElementNode<P> {
    let [child] = children.0.as_ref().left().expect(
        "State corrupted. \
            This suspense has reported to be in an non-fallback state. \
            However, when we inspect it, \
            it found a fallback child is present.",
    );
    let old_children = std::mem::replace(
        children,
        EitherParallel::new_right([child.clone(), fallback]),
    );
    let [old_primary] = old_children.0.left().expect("Impossible to fail");
    old_primary
}

fn replace_fallback_child<P: Protocol>(
    children: &mut EitherParallel<[ArcChildElementNode<P>; 1], [ArcChildElementNode<P>; 2]>,
) -> ArcChildElementNode<P> {
    let [primary_child, _fallback_child] = children.0.as_ref().right().expect(
        "State corrupted. \
        This suspense has reported to be in an fallback state. \
        However, when we inspect it, \
        it found a non-fallback child is present.",
    );
    let old_children =
        std::mem::replace(children, EitherParallel::new_left([primary_child.clone()]));
    let [_old_primary_child, old_fallback_child] =
        old_children.0.right().expect("Impossible to fail");
    old_fallback_child
}
