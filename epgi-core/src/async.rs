mod build;
pub use build::*;

mod build_context;
pub use build_context::*;

use crate::{
    foundation::{Arc, Asc},
    scheduler::get_current_scheduler,
    sync::CommitBarrier,
    tree::{ElementNode, FullElement, WorkContext, WorkHandle},
};

impl<E: FullElement> ElementNode<E> {
    pub(crate) fn spawn_reconcile_node_async(
        self: Arc<Self>,
        work_context: Asc<WorkContext>,
        barrier: CommitBarrier,
    ) {
        {
            get_current_scheduler().async_threadpool.spawn(move || {
                self.reconcile_node_async(None, work_context, WorkHandle::new(), barrier);
            })
        }
    }
}
