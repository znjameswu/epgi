use either::Either;

use crate::{
    foundation::{Arc, ArrayContainer, Asc, EitherContainer, EitherParallel, Protocol},
    nodes::{RenderSuspense, Suspense, SuspenseElement},
    scheduler::BuildScheduler,
    sync::{
        render_element::update_children, SubtreeRenderObjectChange,
        SubtreeRenderObjectChangeSummary,
    },
    tree::{
        AnyRenderObject, ArcChildElementNode, ArcElementContextNode,
        ChildRenderObjectsUpdateCallback, Element, ElementBase, ElementImpl, ElementNode,
        MainlineState, RenderAction, RenderObject,
    },
};

use super::ImplReconcileCommit;

impl<P: Protocol, const PROVIDE_ELEMENT: bool> ImplReconcileCommit<SuspenseElement<P>>
    for ElementImpl<SuspenseElement<P>, true, PROVIDE_ELEMENT>
where
    // This is added because rustc unable to prove for any concrete type in this scenario
    SuspenseElement<P>: Element<
        ArcWidget = Asc<Suspense<P>>,
        ParentProtocol = P,
        ChildProtocol = P,
        ChildContainer = EitherContainer<ArrayContainer<1>, ArrayContainer<2>>,
    >,
{
    fn visit_commit(
        element_node: &ElementNode<SuspenseElement<P>>,
        render_object: Option<Arc<RenderObject<RenderSuspense<P>>>>,
        render_object_changes: EitherParallel<
            [SubtreeRenderObjectChange<P>; 1],
            [SubtreeRenderObjectChange<P>; 2],
        >,
        self_rebuild_suspended: bool,
        scope: &rayon::Scope<'_>,
        build_scheduler: &BuildScheduler,
    ) -> SubtreeRenderObjectChange<P> {
        let render_object = render_object.expect("Suspense can never suspend");
        use Either::*;
        use SubtreeRenderObjectChange::*;
        match render_object_changes.0 {
            Left(
                [Keep {
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
            Left([child_change @ New(_)]) => {
                let render_action = render_object
                    .mark_render_action(RenderAction::Relayout, RenderAction::Relayout);
                {
                    let mut inner = render_object.inner.lock();
                    update_children::<RenderSuspense<P>>(
                        &mut inner.children,
                        None,
                        [child_change],
                        SubtreeRenderObjectChangeSummary::HasNewNoSuspend,
                    );
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

                let fallback = {
                    let snapshot = element_node.snapshot.lock();
                    snapshot.widget.fallback.clone()
                };

                let (fallback, change) =
                    fallback.inflate_sync(element_node.context.clone(), build_scheduler);

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

            Right([New(child), Keep { .. } | New(_)]) => {}
            Right([Keep { .. } | Suspend, New(fallback_child)]) => {
                todo!()
            }
            Right(
                [Keep { .. } | Suspend, Keep {
                    child_render_action,
                    subtree_has_action,
                }],
            ) => todo!(),
        };

        todo!()
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
        Self::OptionArcRenderObject,
        SubtreeRenderObjectChange<<SuspenseElement<P> as crate::tree::ElementBase>::ParentProtocol>,
    ) {
        let render_object = render_object.expect("Suspense can never suspend");

        use Either::*;
        use SubtreeRenderObjectChange::*;
        match render_object_changes.0 {
            Left(
                [Keep {
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
            Left([child_change @ New(_)]) => {
                let render_action = render_object
                    .mark_render_action(RenderAction::Relayout, RenderAction::Relayout);
                {
                    let mut inner = render_object.inner.lock();
                    update_children::<RenderSuspense<P>>(
                        &mut inner.children,
                        None,
                        [child_change],
                        SubtreeRenderObjectChangeSummary::HasNewNoSuspend,
                    );
                }
                todo!()
                // return SubtreeRenderObjectChange::Keep {
                //     child_render_action: render_action,
                //     subtree_has_action: RenderAction::Relayout,
                // };
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
            Right([Keep { .. } | Suspend, New(fallback_child)]) => todo!(),
            Right(
                [Keep { .. } | Suspend, Keep {
                    child_render_action,
                    subtree_has_action,
                }],
            ) => todo!(),
        };
    }

    fn rebuild_suspend_commit(
        render_object: Self::OptionArcRenderObject,
    ) -> SubtreeRenderObjectChange<<SuspenseElement<P> as crate::tree::ElementBase>::ParentProtocol>
    {
        panic!("Suspense can not suspend on itself")
    }

    fn inflate_success_commit(
        element: &SuspenseElement<P>,
        widget: &<SuspenseElement<P> as crate::tree::ElementBase>::ArcWidget,
        element_context: &ArcElementContextNode,
        render_object_changes: crate::foundation::ContainerOf<
            <SuspenseElement<P> as crate::tree::ElementBase>::ChildContainer,
            SubtreeRenderObjectChange<
                <SuspenseElement<P> as crate::tree::ElementBase>::ChildProtocol,
            >,
        >,
    ) -> (
        Self::OptionArcRenderObject,
        SubtreeRenderObjectChange<<SuspenseElement<P> as crate::tree::ElementBase>::ParentProtocol>,
    ) {
        todo!()
    }
}
