use crate::{
    common::{
        ArcChildElementNode, ArcElementContextNode, BuildContext, Element, ElementWidgetPair,
        ReconcileItem, Reconciler2, WorkContext, WorkHandle,
    },
    foundation::{Asc, HktContainer, Parallel, Protocol},
    sync::CommitBarrier,
};

struct AsyncReconciler<'a> {
    host_handle: &'a WorkHandle,
    work_context: Asc<WorkContext>,
    child_tasks: &'a mut Vec<Box<dyn FnOnce() + Send + Sync + 'static>>,
    barrier: CommitBarrier,
    host_context: &'a ArcElementContextNode,
    build_context: &'a mut BuildContext,
}

impl<'a> Reconciler2 for AsyncReconciler<'a> {
    fn build_context_mut(&mut self) -> &mut BuildContext {
        self.build_context
    }

    fn into_reconcile<CP: crate::foundation::Protocol, I: Parallel<Item = ReconcileItem<CP>>>(
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

    impl<E> ChildElementWidgetPairAsyncBuildExt<E::SelfProtocol> for ElementWidgetPair<E>
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

    pub trait ChildWidgetAsyncInflateExt<SP: Protocol> {
        fn inflate_async(
            self: Arc<Self>,
            work_context: Asc<WorkContext>,
            parent_context: &ArcElementContextNode,
            barrier: CommitBarrier,
            handle: WorkHandle,
        ) -> Box<dyn ChildElementWidgetPair<SP>>;
    }

    impl<T> ChildWidgetAsyncInflateExt<<<T as Widget>::Element as Element>::SelfProtocol> for T
    where
        T: Widget,
    {
        fn inflate_async(
            self: Arc<Self>,
            work_context: Asc<WorkContext>,
            parent_context: &ArcElementContextNode,
            barrier: CommitBarrier,
            handle: WorkHandle,
        ) -> Box<dyn ChildElementWidgetPair<<<T as Widget>::Element as Element>::SelfProtocol>>
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
