use crate::{
    foundation::{Arc, ContainerOf},
    r#async::AsyncReconcile,
    sync::LaneScheduler,
    tree::{
        ArcChildElementNode, ElementBase, ElementLockHeldToken, ElementNode, FullElement, Mainline,
    },
};

use super::cancel::CancelAsync;

pub(in super::super) struct ReorderAsync<E: ElementBase> {
    pub(in super::super) cancel:
        Option<CancelAsync<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>>,
    pub(in super::super) start: AsyncReconcile<E>,
}

impl<E: FullElement> ElementNode<E> {
    fn reorder_async_work(self: &Arc<Self>, lane_scheduler: &LaneScheduler) {
        let try_reorder_result = {
            let mut snapshot = self.snapshot.lock();
            let snapshot_reborrow = &mut *snapshot;
            let mainline = snapshot_reborrow
                .inner
                .mainline_mut()
                .expect("reorder_async_work should only be performed on mainline nodes");
            self.prepare_reorder_async_work(
                mainline,
                &snapshot_reborrow.widget,
                lane_scheduler,
                &snapshot_reborrow.element_lock_held,
            )
        };

        if let Some(reorder) = try_reorder_result {
            self.perform_reorder_async_work(reorder)
        }
    }

    pub(in super::super) fn prepare_reorder_async_work(
        self: &Arc<Self>,
        mainline: &mut Mainline<E>,
        old_widget: &E::ArcWidget,
        lane_scheduler: &LaneScheduler,
        element_lock_held: &ElementLockHeldToken,
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
            .min_by_key(|(_, entry)| entry.work_context.batch.priority)
        else {
            return None;
        };

        let backqueue_priority = entry.work_context.batch.priority;
        if let Some(ref curr) = current {
            if backqueue_priority >= curr.work_context.batch.priority {
                return None;
            }
        }

        let backqueue_candidate = backqueue.swap_remove(index);
        let mut cancel = None;
        if let Some(ref curr) = current {
            let curr_lane_pos = curr.work_context.lane_pos;
            cancel = Some(
                Self::prepare_cancel_async_work(mainline, curr_lane_pos, lane_scheduler)
                    .ok()
                    .expect("Impossible to fail"),
            );
        }
        // Why it can't Skip?
        // Because the backqueue_candidate previous tried to occupy this node (hence the entry)
        // Suppose now it comes back with Skip, the only thing that could have caused this change is that the widget has been changed since then.
        // It means that, previously the backqueue_candidate determines there is a widget update, now there isn't.
        // In order to achieve this, the backqueue_candidate needs to have an explicit new widget (otherwise there will always be no widget update)
        // Which means the backqueue_candidate must be a child work of a parent work (only root work can have no explicit new widget)
        // But since the widget has changed since then, it means another committed work must have an explicit new widget and changed it during commit.
        // Then backqueue_candidate would have already conflict with that hypothetical work in the parent node, and must have already been cancelled.
        // Conflict! Therefore it can't return Skip now.
        let Ok(reconcile) = self.prepare_occupy_async(
            mainline,
            old_widget,
            backqueue_candidate.widget,
            backqueue_candidate.work_context,
            backqueue_candidate.barrier,
            element_lock_held,
        ) else {
            panic!("Impossible to fail")
        };
        return Some(ReorderAsync {
            cancel,
            start: reconcile,
        });
    }

    pub(in super::super) fn perform_reorder_async_work(self: &Arc<Self>, reorder: ReorderAsync<E>) {
        let ReorderAsync { cancel, start } = reorder;
        if let Some(remove) = cancel {
            self.perform_cancel_async_work(remove)
        }
        let node = self.clone();
        node.execute_reconcile_node_async_detached(start);
    }

    pub(in super::super) fn prepare_execute_backqueue(
        self: &Arc<Self>,
        mainline: &mut Mainline<E>,
        old_widget: &E::ArcWidget,
        element_lock_held: &ElementLockHeldToken,
    ) -> Option<AsyncReconcile<E>> {
        let async_queue = &mut mainline.async_queue;
        let Some(backqueue) = async_queue.backqueue_mut() else {
            return None;
        };

        let Some((index, _)) = backqueue
            .iter()
            .rev()
            .enumerate()
            .min_by_key(|(_, entry)| entry.work_context.batch.priority)
        else {
            return None;
        };

        let backqueue_candidate = backqueue.swap_remove(index);

        // Why it can't be Skip?
        // Because the backqueue_candidate previous tried to occupy this node (hence the entry)
        // ~~Suppose now it comes back with Skip, the only thing that could have caused this change is that the widget has been changed since then.~~
        // (???? A subscription could also have been cancelled)
        // (Decision: we also revert the work at the provider if we cancel a subscription)
        // It means that, previously the backqueue_candidate determines there is a widget update, now there isn't.
        // In order to achieve this, the backqueue_candidate needs to have an explicit new widget (otherwise there will always be no widget update)
        // Which means the backqueue_candidate must be a child work of a parent work (only root work can have no explicit new widget)
        // But since the widget has changed since then, it means another committed work must have an explicit new widget and changed it during commit.
        // Then backqueue_candidate would have already conflict with that hypothetical work in the parent node, and must have already been cancelled.
        // Conflict! Therefore it can't return Skip now.
        let Ok(reconcile) = self.prepare_occupy_async(
            mainline,
            old_widget,
            backqueue_candidate.widget,
            backqueue_candidate.work_context,
            backqueue_candidate.barrier,
            element_lock_held,
        ) else {
            panic!("Impossible to fail")
        };
        return Some(reconcile);
    }
}

pub(crate) mod reorder_work_private {
    use super::*;
    pub trait AnyElementNodeReorderAsyncWorkExt {
        fn reorder_async_work(self: Arc<Self>, lane_scheduler: &LaneScheduler);
    }

    impl<E: FullElement> AnyElementNodeReorderAsyncWorkExt for ElementNode<E> {
        fn reorder_async_work(self: Arc<Self>, lane_scheduler: &LaneScheduler) {
            ElementNode::reorder_async_work(&self, lane_scheduler)
        }
    }
}
