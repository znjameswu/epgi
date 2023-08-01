use crate::{
    common::{
        ArcChildElementNode, ArcElementContextNode, BuildContext, Element, ElementWidgetPair,
        ReconcileItem, Reconciler, WorkContext, WorkHandle,
    },
    foundation::{Asc, HktContainer, InlinableDwsizeVec, Parallel, Protocol},
    sync::CommitBarrier,
};

pub(super) struct AsyncReconciler<'a, CP: Protocol> {
    pub(super) host_handle: &'a WorkHandle,
    pub(super) work_context: Asc<WorkContext>,
    pub(super) child_tasks: &'a mut Vec<Box<dyn FnOnce() + Send + Sync + 'static>>,
    pub(super) barrier: CommitBarrier,
    pub(super) host_context: &'a ArcElementContextNode,
    pub(super) build_context: &'a mut BuildContext,
    pub(super) nodes_needing_unmount: &'a mut InlinableDwsizeVec<ArcChildElementNode<CP>>,
}

impl<'a, CP> Reconciler<CP> for AsyncReconciler<'a, CP>
where
    CP: Protocol,
{
    fn build_context_mut(&mut self) -> &mut BuildContext {
        self.build_context
    }

    fn nodes_needing_unmount_mut(&mut self) -> &mut InlinableDwsizeVec<ArcChildElementNode<CP>> {
        self.nodes_needing_unmount
    }

    fn into_reconcile<I: Parallel<Item = ReconcileItem<CP>>>(
        self,
        items: I,
    ) -> <I::HktContainer as HktContainer>::Container<ArcChildElementNode<CP>> {
        todo!()
    }
}

pub(crate) mod reconciler_private {
    use crate::{
        common::{
            ArcElementContextNode, ChildElementWidgetPair, ElementNode, Widget, Work, WorkContext,
            WorkHandle,
        },
        foundation::{Arc, Asc},
        sync::CommitBarrier,
    };

    use super::*;
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
        E: Element,
    {
        fn rebuild_async(
            self,
            work_context: Asc<WorkContext>,
            parent_handle: WorkHandle,
            barrier: CommitBarrier,
        ) {
            let _ = self.element.rebuild_node_async(
                Work {
                    widget: Some(self.widget),
                    context: work_context,
                },
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

    pub trait ChildWidgetAsyncInflateExt<PP: Protocol> {
        fn inflate_async(
            self: Arc<Self>,
            work_context: Asc<WorkContext>,
            parent_context: &ArcElementContextNode,
            barrier: CommitBarrier,
            handle: WorkHandle,
        ) -> Box<dyn ChildElementWidgetPair<PP>>;
    }

    impl<T> ChildWidgetAsyncInflateExt<<<T as Widget>::Element as Element>::ParentProtocol> for T
    where
        T: Widget,
    {
        fn inflate_async(
            self: Arc<Self>,
            work_context: Asc<WorkContext>,
            parent_context: &ArcElementContextNode,
            barrier: CommitBarrier,
            handle: WorkHandle,
        ) -> Box<dyn ChildElementWidgetPair<<<T as Widget>::Element as Element>::ParentProtocol>>
        {
            let node = ElementNode::<<T as Widget>::Element>::new_async_uninflated(
                self.clone().into_arc_widget(),
                work_context,
                parent_context,
                handle,
                barrier,
            );
            return Box::new(ElementWidgetPair {
                widget: self.into_arc_widget(),
                element: node,
            });
        }
    }
}
