use crate::{
    foundation::{Arc, Asc, Container, ContainerOf, InlinableDwsizeVec, Protocol, Provide},
    r#async::{AsyncBuildContext, AsyncHookContext},
    scheduler::get_current_scheduler,
    sync::CommitBarrier,
    tree::{
        ArcChildElementNode, AsyncOutput, BuildResults, BuildSuspendResults, ElementNode,
        ElementReconcileItem, ElementWidgetPair, FullElement, WorkContext, WorkHandle,
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
        mut hook_context: AsyncHookContext,
        children: ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        child_work_context: Asc<WorkContext>,
        handle: WorkHandle,
        barrier: CommitBarrier,
    ) {
        let mut nodes_needing_unmount = Default::default();
        let results = E::perform_rebuild_element(
            &mut element,
            &widget,
            AsyncBuildContext {
                hooks: &mut hook_context,
                element_context: &self.context,
            }
            .into(),
            provider_values,
            children,
            &mut nodes_needing_unmount,
        );

        let lane_pos = child_work_context.lane_pos;

        let output = match results {
            Ok((items, shuffle)) => {
                let async_threadpool = &get_current_scheduler().async_threadpool;
                let new_children = items.map_collect_with(
                    (child_work_context, handle.clone(), barrier),
                    |(child_work_context, handle, barrier), item| {
                        use ElementReconcileItem::*;
                        match item {
                            Keep(node) => node,
                            Update(pair) => {
                                let node = pair.element();
                                async_threadpool.spawn(move || {
                                    pair.rebuild_async_box(child_work_context, handle, barrier)
                                });
                                node
                            }
                            Inflate(widget) => {
                                let pair = widget.inflate_async(
                                    child_work_context,
                                    Some(self.context.clone()),
                                    handle,
                                    barrier,
                                );
                                let node = pair.element();
                                todo!();
                                node
                            }
                        }
                    },
                );

                AsyncOutput::Completed {
                    children: new_children,
                    results: BuildResults::from_pieces(
                        hook_context,
                        element,
                        nodes_needing_unmount,
                        shuffle,
                    ),
                }
            }
            Err((children, err)) => AsyncOutput::Suspended {
                suspend: Some(BuildSuspendResults::new(hook_context)),
                barrier: todo!(),
            },
        };

        self.write_back_build_results::<false>(output, lane_pos, &handle, todo!());
    }
}
