use crate::{
    foundation::Arc,
    scheduler::LanePos,
    sync::BuildScheduler,
    tree::{Element, ElementNode},
};

use super::reorder_work::ReorderAsync;

impl<E> ElementNode<E>
where
    E: Element,
{
    fn restart_async_work(
        self: &Arc<Self>,
        lane_pos: LanePos,
        // This BuildScheduler is necessary!
        // Since we need to revert whatever state the work is in back to the initial state,
        // there is a possiblility that we need to generate the CommitBarrier for the initial state, and the Completed state lacks it
        build_scheduler: &BuildScheduler,
    ) {
        let reorder = {
            let mut snapshot = self.snapshot.lock();
            let snapshot_rebrrow = &mut *snapshot;
            let mainline = snapshot_rebrrow
                .inner
                .mainline_mut()
                .expect("Restart can only be called on mainline nodes");

            let cancel = Self::prepare_cancel_async_work(mainline, lane_pos, build_scheduler)
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
        fn restart_async_work(self: Arc<Self>, lane_pos: LanePos, build_scheduler: &BuildScheduler);
    }

    impl<E> AnyElementNodeRestartAsyncExt for ElementNode<E>
    where
        E: Element,
    {
        fn restart_async_work(self: Arc<Self>, lane_pos: LanePos, build_scheduler: &BuildScheduler) {
            ElementNode::restart_async_work(&self, lane_pos, build_scheduler)
        }
    }
}
