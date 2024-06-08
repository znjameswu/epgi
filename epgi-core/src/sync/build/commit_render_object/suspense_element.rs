use either::Either;

use crate::{
    foundation::{Arc, Asc, EitherParallel, Protocol},
    nodes::{RenderSuspense, Suspense, SuspenseElement},
    sync::{LaneScheduler, RenderObjectCommitResult},
    tree::{
        AnyRenderObject, ArcChildElementNode, ArcChildRenderObject, ArcChildWidget,
        ArcElementContextNode, ChildRenderObjectsUpdateCallback, ElementBase, ElementImpl,
        ElementNode, MainlineState, RenderAction, RenderObject,
    },
};

use super::ImplCommitRenderObject;

impl<P: Protocol> ImplCommitRenderObject<SuspenseElement<P>> for ElementImpl<true, false> {
    fn visit_commit_render_object<'batch>(
        element_node: &ElementNode<SuspenseElement<P>>,
        render_object: Option<Arc<RenderObject<RenderSuspense<P>>>>,
        render_object_changes: EitherParallel<
            [RenderObjectCommitResult<P>; 1],
            [RenderObjectCommitResult<P>; 2],
        >,
        lane_scheduler: &'batch LaneScheduler,
        scope: &rayon::Scope<'batch>,
        self_rebuild_suspended: bool,
    ) -> RenderObjectCommitResult<P> {
        debug_assert!(
            self_rebuild_suspended == false,
            "Suspense itself can never suspend"
        );
        let render_object = render_object.expect("Suspense itself can never suspend");
        use Either::*;
        use RenderObjectCommitResult::*;
        match render_object_changes.0 {
            // No update
            Left(
                [Keep {
                    propagated_render_action,
                    subtree_has_action,
                }],
            )
            | Right(
                [Keep { .. } | Suspend, Keep {
                    propagated_render_action,
                    subtree_has_action,
                }],
            ) => {
                let propagated_render_action =
                    render_object.mark_render_action(propagated_render_action, subtree_has_action);
                return RenderObjectCommitResult::Keep {
                    propagated_render_action,
                    subtree_has_action,
                };
            }
            // Normal render object update
            Left([New(child_render_object)])
            | Right([Keep { .. } | Suspend, New(child_render_object)]) => {
                return replace_child_render_object(&render_object, child_render_object, None)
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

                let (fallback, commit_result) = inflate_fallback_and_replace_render_object(
                    &render_object,
                    fallback_widget,
                    element_node.context.clone(),
                    lane_scheduler,
                );

                {
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

                // We shall never unmount the primary child element!

                return commit_result;
            }

            Right([_child_change, Suspend]) => panic!(
                "The fallback component inside this Suspense has suspended. \
                    This is not supposed to happen. \
                    We have not decided to support cascaded suspense propagation."
            ),

            // The primary child has resumed, now we unmount the fallback and remount the primary
            Right([New(child_render_object), fallback_change @ (Keep { .. } | New(_))]) => {
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

                // The fallback element shall be unmounted
                scope.spawn(|scope| {
                    old_fallback_child.unmount(scope, lane_scheduler);
                    if let New(fallback_render_object) = fallback_change {
                        fallback_render_object.detach_render_object();
                    }
                });

                return replace_child_render_object(
                    &render_object,
                    child_render_object,
                    Some(false),
                );
            }
        };
    }

    fn rebuild_success_commit_render_object<'batch>(
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
            [RenderObjectCommitResult<P>; 1],
            [RenderObjectCommitResult<P>; 2],
        >,
        element_context: &ArcElementContextNode,
        lane_scheduler: &'batch LaneScheduler,
        scope: &rayon::Scope<'batch>,
        _is_new_widget: bool,
    ) -> RenderObjectCommitResult<P> {
        debug_assert!(_shuffle.is_none(), "Suspense cannot shuffle its child");
        let render_object = render_object.as_mut().expect("Suspense can never suspend");

        use Either::*;
        use RenderObjectCommitResult::*;
        match render_object_changes.0 {
            Left(
                [Keep {
                    propagated_render_action,
                    subtree_has_action,
                }],
            )
            | Right(
                [Keep { .. } | Suspend, Keep {
                    propagated_render_action,
                    subtree_has_action,
                }],
            ) => {
                let render_action =
                    render_object.mark_render_action(propagated_render_action, subtree_has_action);
                return RenderObjectCommitResult::Keep {
                    propagated_render_action: render_action,
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
                return RenderObjectCommitResult::Keep {
                    propagated_render_action: render_action,
                    subtree_has_action: RenderAction::Relayout,
                };
            }
            Left([Suspend]) => {
                // We choose to read widget right from inside the element node
                // Because requesting widget from context would heavily impact performance of all other visit
                // And suspense is considered to be a rare case / slow path
                // Hence this cost is justified.

                // This is a commit-time effect

                let (fallback, change) = inflate_fallback_and_replace_render_object(
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
                let old_fallback_child = replace_fallback_child(children);

                scope.spawn(|scope| {
                    old_fallback_child.unmount(scope, lane_scheduler);
                    if let New(fallback_render_object) = fallback_change {
                        fallback_render_object.detach_render_object();
                    }
                });

                return replace_child_render_object(
                    render_object,
                    child_render_object,
                    Some(false),
                );
            }
        };
    }

    fn rebuild_suspend_commit_render_object(
        _render_object: Option<Arc<RenderObject<RenderSuspense<P>>>>,
    ) -> RenderObjectCommitResult<P> {
        panic!("Suspense can not suspend on itself")
    }

    fn inflate_success_commit_render_object(
        _element: &SuspenseElement<P>,
        widget: &Asc<Suspense<P>>,
        children: &mut EitherParallel<[ArcChildElementNode<P>; 1], [ArcChildElementNode<P>; 2]>,
        render_object_changes: EitherParallel<
            [RenderObjectCommitResult<P>; 1],
            [RenderObjectCommitResult<P>; 2],
        >,
        element_context: &ArcElementContextNode,
        lane_scheduler: &LaneScheduler,
    ) -> (
        Option<Arc<RenderObject<RenderSuspense<P>>>>,
        RenderObjectCommitResult<P>,
    ) {
        let [child_change] = render_object_changes
            .0
            .left()
            .expect("Suspense should always try inflate its primary child first");

        debug_assert!(
            !child_change.is_keep_render_object(),
            "Fatal logic bug in epgi-core reconcile logic. Please file issue report."
        );

        use RenderObjectCommitResult::*;
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
    let (fallback, commit_result) =
        fallback_widget.inflate_sync(Some(element_context), lane_scheduler);

    let RenderObjectCommitResult::New(fallback_render_object) = commit_result.render_object else {
        panic!(
            "The fallback component inside this Suspense has suspended. \
            This is not supposed to happen. \
            We have not decided to support cascaded suspense propagation."
        )
    };

    (fallback, fallback_render_object)
}

fn inflate_fallback_and_replace_render_object<P: Protocol>(
    render_object: &Arc<RenderObject<RenderSuspense<P>>>,
    fallback_widget: ArcChildWidget<P>,
    element_context: ArcElementContextNode,
    lane_scheduler: &LaneScheduler,
) -> (ArcChildElementNode<P>, RenderObjectCommitResult<P>) {
    let (fallback, fallback_render_object) =
        inflate_fallback(fallback_widget, element_context, lane_scheduler);

    (
        fallback,
        replace_child_render_object(render_object, fallback_render_object, Some(true)),
    )
}

fn replace_child_render_object<P: Protocol>(
    render_object: &Arc<RenderObject<RenderSuspense<P>>>,
    child_render_object: ArcChildRenderObject<P>,
    new_suspend_state: Option<bool>,
) -> RenderObjectCommitResult<P> {
    {
        let mut inner = render_object.inner.lock();
        debug_assert!(
            inner.children[0].render_mark().is_detached().is_ok(),
            "Replaced old child render object should have already been detached 
            when their element was unmounted or suspended"
        );
        inner.children = [child_render_object];
        if let Some(is_suspended) = new_suspend_state {
            inner.render.is_suspended = is_suspended;
        }
        // Actually, the child should have already detached its render object
        // if is_suspended {
        //     old_child_render_object.detach_render_object();
        //     // If the change is from suspended to resumed, then we don't need to detach here. Later, unmount will detach the render object.
        // }
    }
    let propagated_render_action =
        render_object.mark_render_action(RenderAction::Relayout, RenderAction::Relayout);

    // Suspense will always return Keep during rebuild
    return RenderObjectCommitResult::Keep {
        propagated_render_action,
        subtree_has_action: RenderAction::Relayout,
    };
}

fn replace_suspended_primary_child<P: Protocol>(
    children: &mut EitherParallel<[ArcChildElementNode<P>; 1], [ArcChildElementNode<P>; 2]>,
    fallback: ArcChildElementNode<P>,
) {
    let [child] = children.0.as_ref().left().expect(
        "State corrupted. \
            This suspense has reported to be in an non-fallback state. \
            However, when we inspect it, \
            it found a fallback child is present.",
    );
    *children = EitherParallel::new_right([child.clone(), fallback]);
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
