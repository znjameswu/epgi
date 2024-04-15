use crate::{
    foundation::Arc,
    scheduler::LanePos,
    sync::LaneScheduler,
    tree::{ElementNode, FullElement},
};

use super::reorder_work::ReorderAsync;

impl<E: FullElement> ElementNode<E> {
    fn restart_async_work(
        self: &Arc<Self>,
        lane_pos: LanePos,
        // This BuildScheduler is necessary!
        // Since we need to revert whatever state the work is in back to the initial state,
        // there is a possiblility that we need to generate the CommitBarrier for the initial state, and the Completed state lacks it
        lane_scheduler: &LaneScheduler,
    ) {
        let reorder = {
            let mut snapshot = self.snapshot.lock();
            let snapshot_rebrrow = &mut *snapshot;
            let mainline = snapshot_rebrrow
                .inner
                .mainline_mut()
                .expect("Restart can only be called on mainline nodes");

            let cancel = Self::prepare_cancel_async_work(mainline, lane_pos, lane_scheduler)
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
        fn restart_async_work(self: Arc<Self>, lane_pos: LanePos, lane_scheduler: &LaneScheduler);
    }

    impl<E: FullElement> AnyElementNodeRestartAsyncExt for ElementNode<E> {
        fn restart_async_work(self: Arc<Self>, lane_pos: LanePos, lane_scheduler: &LaneScheduler) {
            ElementNode::restart_async_work(&self, lane_pos, lane_scheduler)
        }
    }
}
