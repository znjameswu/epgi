use crate::{
    foundation::{Arc, Asc, Container, ContainerOf, InlinableDwsizeVec, Protocol, Provide},
    r#async::{AsyncBuildContext, AsyncHookContext},
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
        work_context: Asc<WorkContext>,
        handle: &WorkHandle,
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

        let output = match results {
            Ok((items, shuffle)) => {
                let (new_children, pair) = items.unzip_collect(|item| {
                    use ElementReconcileItem::*;
                    match item {
                        Keep(node) => (node, todo!()),
                        Update(pair) => {
                            pair.rebuild_async_box(todo!(), todo!(), todo!());
                            (pair.element(), pair)
                        }
                        Inflate(widget) => {
                            let pair = widget.inflate_async(
                                todo!(),
                                Some(self.context.clone()),
                                todo!(),
                                todo!(),
                            );
                            (pair.element(), pair)
                        }
                    }
                });

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

        self.write_back_build_results::<false>(output, work_context.lane_pos, handle, todo!());
        todo!("Child Tasks");
    }
}
