use crate::{
    foundation::{Arc, Asc, Container, ContainerOf, InlinableDwsizeVec, Protocol, Provide},
    scheduler::get_current_scheduler,
    sync::{CommitBarrier, ImplCommitRenderObject},
    tree::{
        ArcChildElementNode, AsyncOutput, BuildContext, BuildResults, BuildSuspendResults, Element,
        ElementNode, ElementReconcileItem, ElementWidgetPair, FullElement, HookContext,
        HookContextMode, HooksWithEffects, WorkContext, WorkHandle,
    },
};

pub trait ChildElementWidgetPairAsyncBuildExt<P: Protocol> {
    fn rebuild_async(
        self,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    );

    fn rebuild_async_box(
        self: Box<Self>,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    );
}

impl<E> ChildElementWidgetPairAsyncBuildExt<E::ParentProtocol> for ElementWidgetPair<E>
where
    E: FullElement,
{
    fn rebuild_async(
        self,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    ) {
        let _ = self.element.reconcile_node_async(
            Some(self.widget),
            work_context,
            parent_handle,
            barrier,
        );
    }

    fn rebuild_async_box(
        self: Box<Self>,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    ) {
        self.rebuild_async(work_context, parent_handle, barrier)
    }
}

impl<E: FullElement> ElementNode<E> {
    pub(super) fn perform_rebuild_node_async(
        self: &Arc<Self>,
        widget: &E::ArcWidget,
        mut element: E,
        mut hooks: HooksWithEffects,
        children: ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        child_work_context: Asc<WorkContext>,
        handle: WorkHandle,
        barrier: CommitBarrier,
    ) {
        let mut nodes_needing_unmount = Default::default();
        let mut ctx = BuildContext {
            lane_pos: child_work_context.lane_pos,
            element_context: &self.context,
            hook_context: HookContext::new_async(&mut hooks, HookContextMode::Rebuild),
        };
        let results = E::perform_rebuild_element(
            &mut element,
            &widget,
            &mut ctx,
            provider_values,
            children,
            &mut nodes_needing_unmount,
        );

        let lane_pos = child_work_context.lane_pos;

        let output = match results {
            Ok((items, shuffle)) => {
                assert!(
                    ctx.hook_context.has_finished(),
                    "A build function should always invoke every hook whenever it is called"
                );

                let async_threadpool = &get_current_scheduler().async_threadpool;
                // let mut nodes_inflating = InlinableDwsizeVec::new();
                let new_children = items.map_collect_with(
                    (child_work_context, handle.clone(), barrier),
                    |(child_work_context, handle, barrier), item| {
                        use ElementReconcileItem::*;
                        match item {
                            Keep(node) => {
                                let node_clone = node.clone();
                                async_threadpool.spawn(move || {
                                    node_clone.visit_and_work_async(
                                        child_work_context,
                                        handle,
                                        barrier,
                                    );
                                });
                                node
                            }
                            Update(pair) => {
                                let node = pair.element();
                                async_threadpool.spawn(move || {
                                    pair.rebuild_async_box(child_work_context, handle, barrier)
                                });
                                node
                            }
                            Inflate(widget) => {
                                let (node, child_handle) = widget.inflate_async_placeholder(
                                    child_work_context.clone(),
                                    Some(self.context.clone()),
                                    barrier.clone(),
                                );
                                let node_clone = node.clone();
                                async_threadpool.spawn(move || {
                                    node_clone.inflate_async(
                                        child_work_context,
                                        child_handle,
                                        barrier,
                                        <E as Element>::Impl::ALLOW_ASYNC_COMMIT_INFLATE_SUSPENDED_CHILD,
                                    )
                                });
                                // nodes_inflating.push(node.clone());
                                node
                            }
                        }
                    },
                );

                AsyncOutput::Completed(BuildResults::new_rebuild(
                    hooks,
                    element,
                    new_children,
                    nodes_needing_unmount,
                    // nodes_inflating,
                    shuffle,
                ))
            }
            Err((children, err)) => {
                // This is a rebuild. As a result, in the current design, we will always wait until the suspended node is resolved.
                // When we do commit, the node is guaranteed to be resolved. By then, we have already visited and finished building the new children.
                // Therefore, we do not need to visit the mainline children.
                // Note, this is different from the sync rebuild, which does need to visit the mainline children.
                drop(children);
                AsyncOutput::Suspended {
                    suspended_results: Some(BuildSuspendResults::new(hooks, err.waker)),
                    barrier: Some(barrier),
                }
            }
        };

        self.write_back_build_results::<false>(output, lane_pos, &handle);
    }
}
