use either::Either;

use crate::{
    foundation::{Arc, Asc, EitherParallel, Protocol},
    nodes::{RenderSuspense, Suspense, SuspenseElement},
    sync::{LaneScheduler, SubtreeRenderObjectChange},
    tree::{
        AnyRenderObject, ArcChildElementNode, ArcElementContextNode,
        ChildRenderObjectsUpdateCallback, ElementBase, ElementImpl, ElementNode, MainlineState,
        RenderAction, RenderObject,
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
        self_rebuild_suspended: bool,
        scope: &rayon::Scope<'_>,
        lane_scheduler: &LaneScheduler,
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

                // This is a commit-time effect

                let fallback = {
                    let snapshot = element_node.snapshot.lock();
                    snapshot.widget.fallback.clone()
                };

                let (fallback, change) =
                    fallback.inflate_sync(Some(element_node.context.clone()), lane_scheduler);

                let SubtreeRenderObjectChange::New(fallback_render_object) = change else {
                    panic!(
                        "The fallback component inside this Suspense has suspended. \
                            This is not supposed to happen. \
                            We have not decided to support cascaded suspense propagation."
                    )
                };

                {
                    let mut inner = render_object.inner.lock();
                    inner.children = [fallback_render_object];
                    // Detach of the old render object happens at unmount
                }

                let render_action = render_object
                    .mark_render_action(RenderAction::Relayout, RenderAction::Relayout);

                let [old_child] = {
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
                    old_children.0.left().expect("Impossible to fail")
                };

                scope.spawn(|scope| old_child.unmount(scope));

                return SubtreeRenderObjectChange::Keep {
                    child_render_action: render_action,
                    subtree_has_action: RenderAction::Relayout,
                };
            }

            Right([_child_change, Suspend]) => panic!(
                "The fallback component inside this Suspense has suspended. \
                    This is not supposed to happen. \
                    We have not decided to support cascaded suspense propagation."
            ),

            // The primary child has resumed, now we unmount the fallback and remount the primary
            Right([New(child_render_object), fallback_change @ (Keep { .. } | New(_))]) => {
                {
                    let mut inner = render_object.inner.lock();
                    inner.children = [child_render_object];
                    // Detach of the old render object happens at unmount
                }

                let render_action = render_object
                    .mark_render_action(RenderAction::Relayout, RenderAction::Relayout);

                let [_old_primary_child, old_fallback_child] = {
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
                    let [primary_child, fallback_child] = children.0.as_ref().right().expect(
                        "State corrupted. \
                            This suspense has reported to be in an fallback state. \
                            However, when we inspect it, \
                            it found a non-fallback child is present.",
                    );
                    let old_children = std::mem::replace(
                        children,
                        EitherParallel::new_left([primary_child.clone()]),
                    );
                    old_children.0.right().expect("Impossible to fail")
                };

                scope.spawn(|scope| {
                    old_fallback_child.unmount(scope);
                    if let New(fallback_render_object) = fallback_change {
                        fallback_render_object.detach_render_object();
                    }
                });

                return SubtreeRenderObjectChange::Keep {
                    child_render_action: render_action,
                    subtree_has_action: RenderAction::Relayout,
                };
            }
        };
    }

    fn rebuild_success_commit(
        element: &SuspenseElement<P>,
        widget: &Asc<Suspense<P>>,
        _shuffle: Option<
            ChildRenderObjectsUpdateCallback<
                <SuspenseElement<P> as ElementBase>::ChildContainer,
                P,
            >,
        >,
        children: &EitherParallel<[ArcChildElementNode<P>; 1], [ArcChildElementNode<P>; 2]>,
        render_object: Option<Arc<RenderObject<RenderSuspense<P>>>>,
        render_object_changes: EitherParallel<
            [SubtreeRenderObjectChange<P>; 1],
            [SubtreeRenderObjectChange<P>; 2],
        >,
        element_context: &ArcElementContextNode,
        is_new_widget: bool,
    ) -> (
        Option<Arc<RenderObject<RenderSuspense<P>>>>,
        SubtreeRenderObjectChange<P>,
    ) {
        debug_assert!(_shuffle.is_none(), "Suspense cannot shuffle its child");
        let render_object = render_object.expect("Suspense can never suspend");

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
                return (
                    Some(render_object),
                    SubtreeRenderObjectChange::Keep {
                        child_render_action: render_action,
                        subtree_has_action,
                    },
                );
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
                return (
                    Some(render_object),
                    SubtreeRenderObjectChange::Keep {
                        child_render_action: render_action,
                        subtree_has_action: RenderAction::Relayout,
                    },
                );
            }
            Left([Suspend]) => {
                // We choose to read widget right from inside the element node
                // Because requesting widget from context would heavily impact performance of all other visit
                // And suspense is considered to be a rare case / slow path
                // Hence this cost is justified.

                // This is a commit-time effect

                todo!()
            }

            Right([_child_change, Suspend]) => panic!(
                "The fallback component inside a Suspense has suspended. \
                This is not supposed to happen. \
                We have not decided to support cascaded suspense propagation."
            ),
            Right([New(child), fallback_change @ (Keep { .. } | New(_))]) => todo!(),
        };
    }

    fn rebuild_suspend_commit(
        render_object: Option<Arc<RenderObject<RenderSuspense<P>>>>,
    ) -> SubtreeRenderObjectChange<P> {
        panic!("Suspense can not suspend on itself")
    }

    fn inflate_success_commit(
        element: &SuspenseElement<P>,
        widget: &Asc<Suspense<P>>,
        element_context: &ArcElementContextNode,
        render_object_changes: EitherParallel<
            [SubtreeRenderObjectChange<P>; 1],
            [SubtreeRenderObjectChange<P>; 2],
        >,
    ) -> (
        Option<Arc<RenderObject<RenderSuspense<P>>>>,
        SubtreeRenderObjectChange<P>,
    ) {
        todo!()
    }
}

// fn handle_primary_child_suspended<P: Protocol>(
//     render_object: Arc<RenderObject<RenderSuspense<P>>>,
//     fallback: ArcChildWidget<P>,
//     element_context: ArcElementContextNode,
//     lane_scheduler: &LaneScheduler,
// ) {
//     let (fallback, change) = fallback.inflate_sync(Some(element_context), lane_scheduler);

//     let SubtreeRenderObjectChange::New(fallback_render_object) = change else {
//         panic!(
//             "The fallback component inside this Suspense has suspended. \
//                             This is not supposed to happen. \
//                             We have not decided to support cascaded suspense propagation."
//         )
//     };

//     {
//         let mut inner = render_object.inner.lock();
//         inner.children = [fallback_render_object];
//         // Detach of the old render object happens at unmount
//     }

//     let render_action =
//         render_object.mark_render_action(RenderAction::Relayout, RenderAction::Relayout);

//     let [old_child] = {
//         let mut snapshot = element_node.snapshot.lock();
//         let state = snapshot
//             .inner
//             .mainline_mut()
//             .expect("An unmounted element node should not be reachable by a rebuild!")
//             .state
//             .as_mut()
//             .expect(
//                 "State corrupted. \
//                             This node has been previously designated to visit by a sync batch. \
//                             However, when the visit returns, \
//                             it found the sync state has been occupied.",
//             );

//         let MainlineState::Ready { children, .. } = state else {
//             panic!("Suspense should always be in the Ready state")
//         };
//         let [child] = children.0.as_ref().left().expect(
//             "State corrupted. \
//                             This suspense has reported to be in an non-fallback state. \
//                             However, when we inspect it, \
//                             it found a fallback child is present.",
//         );
//         let old_children = std::mem::replace(
//             children,
//             EitherParallel::new_right([child.clone(), fallback]),
//         );
//         old_children.0.left().expect("Impossible to fail")
//     };

//     scope.spawn(|scope| old_child.unmount(scope));
// }
