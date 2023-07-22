use crate::{
    common::{Element, ElementNode, Mainline},
    foundation::Arc,
    r#async::AsyncRebuild,
    sync::TreeScheduler,
};

use super::cancel::CancelAsync;

pub(in super::super) struct ReorderAsync<E: Element> {
    pub(in super::super) cancel: Option<CancelAsync<E::ChildIter>>,
    pub(in super::super) start: AsyncRebuild<E>,
}

impl<E> ElementNode<E>
where
    E: Element,
{
    fn reorder_async_work(self: &Arc<Self>, tree_scheduler: &TreeScheduler) {
        let try_reorder_result = {
            let mut snapshot = self.snapshot.lock();
            let snapshot_reborrow = &mut *snapshot;
            let mainline = snapshot_reborrow
                .inner
                .mainline_mut()
                .expect("reorder_async_work should only be performed on mainline nodes");
            self.prepare_reorder_async_work(mainline, &snapshot_reborrow.widget, tree_scheduler)
        };

        if let Some(reorder) = try_reorder_result {
            self.perform_reorder_async_work(reorder)
        }
    }

    pub(in super::super) fn prepare_reorder_async_work(
        self: &Arc<Self>,
        mainline: &mut Mainline<E>,
        old_widget: &E::ArcWidget,
        tree_scheduler: &TreeScheduler,
    ) -> Option<ReorderAsync<E>> {
        let async_queue = &mut mainline.async_queue;

        // Return None if the backqueue is empty
        let (current, Some(backqueue)) = async_queue.current_and_backqueue_mut() else {
            return None;
        };
        let Some((index, entry)) = backqueue
            .iter()
            .rev()
            .enumerate()
            .min_by_key(|(_, entry)| entry.work.context.batch.priority)
        else {
            return None;
        };

        let backqueue_priority = entry.work.context.batch.priority;
        if let Some(ref curr) = current {
            if backqueue_priority >= curr.work.context.batch.priority {
                return None;
            }
        }

        let backqueue_candidate = backqueue.swap_remove(index);
        let mut cancel = None;
        if let Some(ref curr) = current {
            let curr_lane_pos = curr.work.context.lane_pos;
            cancel = Some(
                Self::prepare_cancel_async_work(mainline, curr_lane_pos, tree_scheduler)
                    .ok()
                    .expect("Impossible to fail"),
            );
        }
        let rebuild = self
            .prepare_rebuild_async(
                mainline,
                old_widget,
                backqueue_candidate.work,
                backqueue_candidate.barrier,
            )
            .expect("Impossible to fail")
            .ok()
            .expect("The candidate should not produce a SkipRebuild result");
        return Some(ReorderAsync {
            cancel,
            start: rebuild,
        });
    }

    pub(in super::super) fn perform_reorder_async_work(self: &Arc<Self>, reorder: ReorderAsync<E>) {
        let ReorderAsync { cancel, start } = reorder;
        if let Some(remove) = cancel {
            self.perform_cancel_async_work(remove)
        }
        let node = self.clone();
        node.execute_rebuild_node_async_detached(start);
    }

    pub(in super::super) fn prepare_execute_backqueue(
        self: &Arc<Self>,
        mainline: &mut Mainline<E>,
        old_widget: &E::ArcWidget,
    ) -> Option<AsyncRebuild<E>> {
        let async_queue = &mut mainline.async_queue;
        let Some(backqueue) = async_queue.backqueue_mut() else {
            return None;
        };

        let Some((index, _)) = backqueue
            .iter()
            .rev()
            .enumerate()
            .min_by_key(|(_, entry)| entry.work.context.batch.priority)
        else {
            return None;
        };

        let backqueue_candidate = backqueue.swap_remove(index);

        let rebuild = self
            .prepare_rebuild_async(
                mainline,
                old_widget,
                backqueue_candidate.work,
                backqueue_candidate.barrier,
            )
            .expect("Execute_backqueue should not be performed on an occupied node")
            .ok()
            .expect("The candidate should not produce a SkipRebuild result");

        return Some(rebuild);
    }
}

pub(crate) mod reorder_work_private {
    use super::*;
    pub trait AnyElementNodeReorderAsyncWorkExt {
        fn reorder_async_work(self: Arc<Self>, tree_scheduler: &TreeScheduler);
    }

    impl<E> AnyElementNodeReorderAsyncWorkExt for ElementNode<E>
    where
        E: Element,
    {
        fn reorder_async_work(self: Arc<Self>, tree_scheduler: &TreeScheduler) {
            ElementNode::reorder_async_work(&self, tree_scheduler)
        }
    }
}
