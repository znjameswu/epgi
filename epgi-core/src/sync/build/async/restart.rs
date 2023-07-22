use crate::{
    common::{Element, ElementNode},
    foundation::Arc,
    scheduler::LanePos,
    sync::TreeScheduler,
};

use super::reorder_work::ReorderAsync;

impl<E> ElementNode<E>
where
    E: Element,
{
    fn restart_async_work(
        self: &Arc<Self>,
        lane_pos: LanePos,
        // This TreeScheduler is necessary!
        // Since we need to revert whatever state the work is in back to the initial state,
        // there is a possiblility that we need to generate the CommitBarrier for the initial state, and the Completed state lacks it
        tree_scheduler: &TreeScheduler,
    ) {
        let reorder = {
            let mut snapshot = self.snapshot.lock();
            let snapshot_rebrrow = &mut *snapshot;
            let mainline = snapshot_rebrrow
                .inner
                .mainline_mut()
                .expect("Restart can only be called on mainline nodes");

            let cancel = Self::prepare_cancel_async_work(mainline, lane_pos, tree_scheduler)
                .ok()
                .expect("Lane to be canceled must exist");
            let rebuild = self
                .prepare_execute_backqueue(mainline, &snapshot_rebrrow.widget)
                .expect("Impossible to fail");

            ReorderAsync {
                cancel: Some(cancel),
                start: rebuild,
            }
        };

        self.perform_reorder_async_work(reorder);
    }
}

pub(crate) mod restart_private {
    use super::*;
    pub trait AnyElementNodeRestartAsyncExt {
        fn restart_async_work(self: Arc<Self>, lane_pos: LanePos, tree_scheduler: &TreeScheduler);
    }

    impl<E> AnyElementNodeRestartAsyncExt for ElementNode<E>
    where
        E: Element,
    {
        fn restart_async_work(self: Arc<Self>, lane_pos: LanePos, tree_scheduler: &TreeScheduler) {
            ElementNode::restart_async_work(&self, lane_pos, tree_scheduler)
        }
    }
}
