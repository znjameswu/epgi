use crate::{
    foundation::Arc,
    scheduler::LanePos,
    sync::LaneScheduler,
    tree::{ElementNode, FullElement},
};

use super::reorder_work::ReorderAsync;

pub trait AnyElementNodeRestartAsyncExt {
    fn restart_async_work(self: Arc<Self>, lane_pos: LanePos, lane_scheduler: &LaneScheduler);
}

impl<E: FullElement> AnyElementNodeRestartAsyncExt for ElementNode<E> {
    fn restart_async_work(self: Arc<Self>, lane_pos: LanePos, lane_scheduler: &LaneScheduler) {
        self.restart_async_work_impl(lane_pos, lane_scheduler)
    }
}

impl<E: FullElement> ElementNode<E> {
    fn restart_async_work_impl(
        self: &Arc<Self>,
        lane_pos: LanePos,
        // This BuildScheduler is necessary!
        // Since we need to revert whatever state the work is in back to the initial state,
        // there is a possiblility that we need to generate the CommitBarrier for the initial state, and the Completed state lacks it
        lane_scheduler: &LaneScheduler,
    ) {
        let reorder = {
            let mut snapshot = self.snapshot.lock();
            let snapshot_reborrow = &mut *snapshot;
            let mainline = snapshot_reborrow
                .inner
                .mainline_mut()
                .expect("Restart can only be called on mainline nodes");

            let cancel =
                Self::setup_interrupt_async_work(mainline, lane_pos, lane_scheduler, &self.context)
                    .ok()
                    .expect("Lane to be canceled must exist");
            let rebuild = self
                .setup_execute_backqueue(
                    mainline,
                    &snapshot_reborrow.widget,
                    &snapshot_reborrow.element_lock_held,
                )
                .expect("Impossible to fail");

            ReorderAsync {
                cancel: Some(cancel),
                start: rebuild,
            }
        };

        self.execute_reorder_async_work(reorder);
    }
}
