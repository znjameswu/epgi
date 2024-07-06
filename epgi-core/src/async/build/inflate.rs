use std::borrow::Cow;

use crate::{
    foundation::{
        Arc, Asc, Container, InlinableDwsizeVec, Protocol, Provide, EMPTY_CONSUMED_TYPES,
    },
    scheduler::get_current_scheduler,
    sync::{CommitBarrier, ImplCommitRenderObject},
    tree::{
        ArcChildElementNode, ArcElementContextNode, AsyncInflating, AsyncOutput, AsyncStash,
        BuildContext, BuildResults, BuildSuspendResults, Element, ElementBase, ElementContextNode,
        ElementNode, ElementSnapshotInner, FullElement, HookContext, HookContextMode,
        HooksWithEffects, Widget, WorkContext, WorkHandle,
    },
};

pub trait ChildWidgetAsyncInflateExt<PP: Protocol> {
    fn inflate_async_placeholder(
        self: Arc<Self>,
        work_context: Asc<WorkContext>,
        parent_context: Option<ArcElementContextNode>,
        barrier: CommitBarrier,
    ) -> (ArcChildElementNode<PP>, WorkHandle);
}

impl<T> ChildWidgetAsyncInflateExt<<T::Element as ElementBase>::ParentProtocol> for T
where
    T: Widget,
{
    fn inflate_async_placeholder(
        self: Arc<Self>,
        work_context: Asc<WorkContext>,
        parent_context: Option<ArcElementContextNode>,
        barrier: CommitBarrier,
    ) -> (
        ArcChildElementNode<<T::Element as ElementBase>::ParentProtocol>,
        WorkHandle,
    ) {
        let arc_widget = self.into_arc_widget();
        let (node, handle) = ElementNode::<<T as Widget>::Element>::new_async_uninflated(
            arc_widget,
            work_context,
            parent_context,
            barrier,
        );
        (node, handle)
    }
}

pub trait AnyElementAsyncInflateExt {
    fn inflate_async(
        self: Arc<Self>,
        work_context: Asc<WorkContext>,
        handle: WorkHandle,
        barrier: CommitBarrier,
        allow_commit_suspend: bool,
    );
}

impl<E> AnyElementAsyncInflateExt for ElementNode<E>
where
    E: FullElement,
{
    fn inflate_async(
        self: Arc<Self>,
        work_context: Asc<WorkContext>,
        handle: WorkHandle,
        barrier: CommitBarrier,
        allow_commit_suspend: bool,
    ) {
        self.inflate_node_async_impl(work_context, handle, barrier, allow_commit_suspend)
    }
}

impl<E: FullElement> ElementNode<E> {
    fn new_async_uninflated(
        widget: E::ArcWidget,
        work_context: Asc<WorkContext>,
        parent_context: Option<ArcElementContextNode>,
        barrier: CommitBarrier,
    ) -> (Arc<Self>, WorkHandle) {
        // We cannot reserve our subscription before the node is fully constructed.
        // Otherwise a contending async writing commit may find an uninstantiated node in its reservation list. Which is odd.

        let handle = WorkHandle::new();
        let handle_clone = handle.clone();
        let node = Arc::new_cyclic(move |node| {
            let element_context =
                ElementContextNode::new_for::<E>(node.clone() as _, parent_context, &widget);
            let subscription_diff = Self::calc_subscription_diff(
                &E::get_consumed_types(&widget),
                EMPTY_CONSUMED_TYPES,
                &work_context.recorded_provider_values,
                &element_context.provider_map,
            );
            Self::new(
                Arc::new(element_context),
                widget,
                ElementSnapshotInner::AsyncInflating(AsyncInflating {
                    work_context,
                    stash: AsyncStash {
                        handle,
                        subscription_diff,
                        spawned_consumers: None,
                        output: AsyncOutput::Uninitiated { barrier },
                    },
                }),
            )
        });
        (node, handle_clone)
        // We could either read the subscription here or in the inflate method since async inflating is a two-step process. \
        // Decision: in the inflate method.
    }

    pub(super) fn inflate_node_async_impl(
        self: &Arc<Self>,
        work_context: Asc<WorkContext>,
        handle: WorkHandle,
        barrier: CommitBarrier,
        allow_commit_suspend: bool,
    ) {
        let (provider_values, widget, child_work_context) = {
            let mut snapshot = self.snapshot.lock();
            let snapshot_reborrow = &mut *snapshot;
            if handle.is_aborted() {
                return;
            }
            debug_assert!(
                snapshot_reborrow.inner.async_inflating_mut().is_some(),
                "Async inflate should only be called on a AsyncInflating node"
            );
            let mut child_work_context = Cow::Borrowed(work_context.as_ref());
            // Reversible side effect must happen with the node lock held and the work handle checked
            let provider_values = self.read_consumed_values_async(
                &E::get_consumed_types(&snapshot_reborrow.widget),
                EMPTY_CONSUMED_TYPES,
                &mut child_work_context,
                &barrier,
                &snapshot_reborrow.element_lock_held,
            );
            let child_work_context = match child_work_context {
                Cow::Borrowed(_) => work_context,
                Cow::Owned(work_context) => Asc::new(work_context),
            };
            (
                provider_values,
                snapshot_reborrow.widget.clone(),
                child_work_context,
            )
        };

        self.perform_inflate_node_async::<true>(
            &widget,
            None,
            provider_values,
            child_work_context,
            handle,
            barrier,
            allow_commit_suspend,
        );
    }

    pub(super) fn perform_inflate_node_async<const IS_NEW_INFLATE: bool>(
        self: &Arc<Self>,
        widget: &E::ArcWidget,
        suspended_hooks: Option<HooksWithEffects>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        child_work_context: Asc<WorkContext>,
        handle: WorkHandle,
        barrier: CommitBarrier,
        allow_commit_suspend: bool,
    ) {
        let hook_mode = if suspended_hooks.is_none() {
            HookContextMode::Inflate
        } else {
            HookContextMode::PollInflate
        };
        let mut hooks = suspended_hooks.unwrap_or_default();
        let mut ctx = BuildContext {
            lane_pos: child_work_context.lane_pos,
            element_context: &self.context,
            hook_context: HookContext::new_async(&mut hooks, hook_mode),
        };
        let result = E::perform_inflate_element(&widget, &mut ctx, provider_values);

        let lane_pos = child_work_context.lane_pos;

        let output = match result {
            Ok((element, child_widgets)) => {
                assert!(
                    ctx.hook_context.has_finished(),
                    "A build function should always invoke every hook whenever it is called"
                );

                let async_threadpool = &get_current_scheduler().async_threadpool;
                let children = child_widgets.map_collect_with(
                    (child_work_context, barrier),
                    |(child_work_context, barrier), child_widget| {
                        let (node, child_handle) = child_widget.inflate_async_placeholder(
                            child_work_context.clone(),
                            Some(self.context.clone()),
                            barrier.clone(),
                        );
                        let node_clone = node.clone();
                        async_threadpool.spawn(move || {
                            node_clone.inflate_async(
                                child_work_context,
                                child_handle,
                                barrier,
                                allow_commit_suspend
                                    | <E as Element>::Impl::ALLOW_ASYNC_COMMIT_INFLATE_SUSPENDED_CHILD,
                            )
                        });
                        node
                    },
                );
                AsyncOutput::Completed(BuildResults::new_inflate(hooks, element, children))
            }
            Err(err) => AsyncOutput::Suspended {
                suspended_results: Some(BuildSuspendResults::new(hooks, err.waker)),
                barrier: (!allow_commit_suspend).then_some(barrier),
            },
        };

        self.write_back_build_results::<IS_NEW_INFLATE>(output, lane_pos, &handle);
    }
}
