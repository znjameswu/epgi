use crate::{
    foundation::Arc,
    sync::LaneScheduler,
    tree::{ElementNode, FullElement, SuspendWaker},
};
use core::sync::atomic::Ordering::*;

use super::provider::AsyncWorkNeedsRestarting;

pub trait AnyElementNodeUnmountExt {
    fn unmount<'batch>(
        self: Arc<Self>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    );
}

impl<E: FullElement> AnyElementNodeUnmountExt for ElementNode<E> {
    fn unmount<'batch>(
        self: Arc<Self>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) {
        ElementNode::unmount(&self, scope, lane_scheduler)
    }
}

impl<E: FullElement> ElementNode<E> {
    // We could require a BuildScheduler in parameter to ensure the global lock
    // However, doing so on a virtual function incurs additional overhead.
    fn unmount<'batch>(
        self: &Arc<Self>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) {
        self.context.unmounted.store(true, Relaxed);
        let (children, widget, purge) = {
            // How do we ensure no one else will occupy/lane-mark this node after we unmount it?
            // 1. Ways of async batch to occupy this node
            //      1. Async reconciling down from the parent, which is occupied by the caller of this method
            //      2. Spawned batch root and visit down from the BuildScheduler
            //      3. Wake-up from suspend
            // 2. Ways to lane-mark this node
            //      1. primary root lane mark from BuildScheduler.
            //      2. secondary root lane mark from any async batch
            //      A staled call can still make scheduler enter this node later.
            let mut snapshot = self.snapshot.lock();

            let mainline = snapshot
                .inner
                .mainline_mut()
                .expect("Unmount should only be called on mainline nodes");
            let purge = if let Some(entry) = mainline.async_queue.current() {
                Some(
                    Self::prepare_purge_async_work_mainline(mainline, entry.work_context.lane_pos)
                        .ok()
                        .expect("Impossible to fail"),
                )
            } else {
                None
            };
            // Drop backqueue.
            mainline.async_queue.backqueue_mut().map(Vec::clear);
            // Read mainline children and drop suspended work.
            let state = mainline
                .state
                .as_ref()
                .expect("A mainline tree walk should not encounter another sync work");
            let children = state.children_cloned();
            state.waker_ref().map(SuspendWaker::set_completed);
            (children, snapshot.widget.clone(), purge)
        };

        if let Some(purge) = purge {
            self.perform_purge_async_work(purge)
        }

        let mut async_work_needs_restarting = AsyncWorkNeedsRestarting::new();
        for consumed_type in E::get_consumed_types(&widget) {
            let provider_node = self
                .context
                .provider_map
                .get(consumed_type)
                .expect("ProviderMap should be consistent");
            let contending_writer = provider_node
                .provider_object
                .as_ref()
                .expect("Element should provide types according to ProviderMap")
                .unregister_read(&Arc::downgrade(&self.context));
            if let Some(contending_lane) = contending_writer {
                async_work_needs_restarting.push_ref(contending_lane, provider_node)
            }
        }
        async_work_needs_restarting.execute_restarts(lane_scheduler);

        // // We just need to ensure the scheduler perform adequate unmount checks before performing below operations
        // todo!("Remove from batch data");
        // todo!("Prevent stale scheduler calls");

        if let Some(children) = children {
            let mut it = children.into_iter();
            if it.len() == 1 {
                let child = it.next().unwrap();
                child.unmount(scope, lane_scheduler)
            } else {
                it.for_each(|child| scope.spawn(|s| child.unmount(s, lane_scheduler)))
            }
            // children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
            //     child.unmount()
            // })
        }
    }
}
