use crate::{
    foundation::Arc,
    sync::LaneScheduler,
    tree::{Element, ElementNode, FullElement, MainlineState},
};
use core::sync::atomic::Ordering::*;

use super::{provider::AsyncWorkNeedsRestarting, ImplCommitRenderObject};

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
        let (children, widget, async_cancel) = {
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
            let async_cancel = Self::setup_unmount_async_work_mainline(mainline);
            // Drop backqueue.
            mainline.async_queue.backqueue_mut().map(Vec::clear);
            // Read mainline children and drop suspended work.
            let state = mainline
                .state
                .take() // We have to take the states because we need to clean up hooks
                .expect("A mainline tree walk should not encounter another sync work");
            use MainlineState::*;
            let (children, hooks) = match state {
                Ready {
                    element: _,
                    hooks,
                    children,
                    render_object,
                } => {
                    <<E as Element>::Impl as ImplCommitRenderObject<E>>::detach_render_object(
                        &render_object,
                    );
                    (Some(children), hooks)
                }
                InflateSuspended {
                    suspended_hooks,
                    waker,
                } => {
                    waker.abort();
                    (None, suspended_hooks)
                }
                RebuildSuspended {
                    element: _,
                    suspended_hooks,
                    children,
                    waker,
                } => {
                    waker.abort();
                    (Some(children), suspended_hooks)
                }
            };
            hooks.cleanup();
            (children, snapshot.widget.clone(), async_cancel)
        };

        if let Some(async_cancel) = async_cancel {
            self.execute_unmount_async_work(async_cancel, scope, false)
        }

        // These side effect reversal does not need the node lock held
        // Because those side effects are only fired in the commit phase, which is holding sync scheduler lock, which we are also holding.
        let mut async_work_needs_restarting = AsyncWorkNeedsRestarting::new();
        for consumed_type in E::get_consumed_types(&widget).iter() {
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
        let async_cancel = {
            let mut snapshot = self.snapshot.lock();
            let snapshot_reborrow = &mut *snapshot;

            let async_inflating = snapshot_reborrow
                .inner
                .async_inflating_mut()
                .expect("Unmount async inflating should only be called on async inflating nodes");

            Self::setup_cancel_async_work_async_inflating(
                async_inflating,
                async_inflating.work_context.lane_pos,
            )
        };
        self.execute_unmount_async_work(async_cancel, scope, true)
    }
}
