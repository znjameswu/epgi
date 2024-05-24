use crate::{
    foundation::Arc,
    sync::LaneScheduler,
    tree::{AsyncInflating, AsyncOutput, AsyncStash, ElementNode, FullElement, SuspendWaker},
};
use core::sync::atomic::Ordering::*;

use super::provider::AsyncWorkNeedsRestarting;

pub trait AnyElementNodeUnmountExt {
    fn unmount<'batch>(
        self: Arc<Self>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    );

    fn unmount_if_async_inflating(self: Arc<Self>, scope: &rayon::Scope<'_>);
}

impl<E: FullElement> AnyElementNodeUnmountExt for ElementNode<E> {
    fn unmount<'batch>(
        self: Arc<Self>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) {
        self.unmount_impl(scope, lane_scheduler)
    }

    fn unmount_if_async_inflating(self: Arc<Self>, scope: &rayon::Scope<'_>) {
        self.unmount_if_async_inflating_impl(scope)
    }
}

impl<E: FullElement> ElementNode<E> {
    // We could require a BuildScheduler in parameter to ensure the global lock
    // However, doing so on a virtual function incurs additional overhead.
    fn unmount_impl<'batch>(
        self: &Arc<Self>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) {
        let unmounted = self.context.unmounted.swap(true, Relaxed);
        if unmounted {
            return;
        }
        let (children, widget, cancel_async) = {
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
            let cancel_async = Self::setup_unmount_async_work_mainline(mainline);
            // Drop backqueue.
            mainline.async_queue.backqueue_mut().map(Vec::clear);
            // Read mainline children and drop suspended work.
            let state = mainline
                .state
                .as_ref()
                .expect("A mainline tree walk should not encounter another sync work");
            let children = state.children_cloned();
            state.waker_ref().map(SuspendWaker::set_completed);
            (children, snapshot.widget.clone(), cancel_async)
        };

        if let Some(cancel_async) = cancel_async {
            self.execute_unmount_async_work(cancel_async, scope)
        }

        // These side effect reversal does not need the node lock held
        // Because those side effects are only fired in the commit phase, which is holding sync scheduler lock, which we are also holding.
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
        // todo!("Prevent stale scheduler calls");

        if let Some(children) = children {
            let mut it = children.into_iter();
            // Single child optimization
            if it.len() == 1 {
                let child = it.next().unwrap();
                child.unmount(scope, lane_scheduler)
            } else {
                it.for_each(|child| scope.spawn(|s| child.unmount(s, lane_scheduler)))
            }
        }
    }

    fn unmount_if_async_inflating_impl(self: &Arc<Self>, scope: &rayon::Scope<'_>) {
        let unmounted = self.context.unmounted.swap(true, Relaxed);
        if unmounted {
            return;
        }
        let new_children = {
            let mut snapshot = self.snapshot.lock();

            let async_inflating = snapshot
                .inner
                .async_inflating_mut()
                .expect("Unmount async inflating should only be called on async inflating nodes");

            let AsyncInflating {
                work_context,
                stash:
                    AsyncStash {
                        handle,
                        subscription_diff,
                        spawned_consumers,
                        output,
                    },
            } = async_inflating;

            handle.abort();
            for reserved in subscription_diff.reserve.iter() {
                reserved.unreserve_read(&(Arc::downgrade(self) as _), work_context.lane_pos);
            }
            debug_assert!(
                spawned_consumers.is_none(),
                "Async inflating should not spawn consumer work"
            );
            let mut new_children = None;
            // Force the destructor to run by taking it out
            match std::mem::replace(output, AsyncOutput::Gone) {
                AsyncOutput::Uninitiated { .. } => {}
                AsyncOutput::Suspended { suspend, .. } => todo!(),
                AsyncOutput::Completed(build_results) => {
                    new_children = Some(build_results.children)
                }
                AsyncOutput::Gone => debug_assert!(
                    false,
                    "Tried to unmount an async inflating node whose output has been taken"
                ),
            }
            new_children
        };
        if let Some(new_children) = new_children {
            let mut it = new_children.into_iter();
            // Single child optimization
            if it.len() == 1 {
                let child = it.next().unwrap();
                child.unmount_if_async_inflating(scope)
            } else {
                it.for_each(|child| scope.spawn(|s| child.unmount_if_async_inflating(s)))
            }
        }
    }
}
