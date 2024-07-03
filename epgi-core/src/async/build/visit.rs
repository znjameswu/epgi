use crate::{
    foundation::{Arc, Asc, Protocol},
    sync::CommitBarrier,
    tree::{ElementNode, FullElement, WorkContext, WorkHandle},
};

pub trait ChildElementAsyncReconcileExt<P: Protocol> {
    fn visit_and_work_async(
        self: Arc<Self>,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    );
}

impl<E> ChildElementAsyncReconcileExt<E::ParentProtocol> for ElementNode<E>
where
    E: FullElement,
{
    fn visit_and_work_async(
        self: Arc<Self>,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    ) {
        let _ = self.reconcile_node_async(None, work_context, parent_handle, barrier);
    }
}
