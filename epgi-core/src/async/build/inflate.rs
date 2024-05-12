use std::borrow::Cow;

use crate::{
    foundation::{
        Arc, Asc, Container, InlinableDwsizeVec, Protocol, Provide, SyncMutex, EMPTY_CONSUMED_TYPES,
    },
    r#async::{AsyncBuildContext, AsyncHookContext},
    scheduler::get_current_scheduler,
    sync::CommitBarrier,
    tree::{
        ArcElementContextNode, AsyncInflating, AsyncOutput, AsyncStash, BuildResults,
        BuildSuspendResults, ChildElementWidgetPair, ElementBase, ElementContextNode, ElementNode,
        ElementSnapshot, ElementSnapshotInner, ElementWidgetPair, FullElement, Widget, WorkContext,
        WorkHandle,
    },
};

pub trait ChildWidgetAsyncInflateExt<PP: Protocol> {
    fn inflate_async_placeholder(
        self: Arc<Self>,
        work_context: Asc<WorkContext>,
        parent_context: Option<ArcElementContextNode>,
        barrier: CommitBarrier,
    ) -> (Box<dyn ChildElementWidgetPair<PP>>, WorkHandle);
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
        Box<dyn ChildElementWidgetPair<<T::Element as ElementBase>::ParentProtocol>>,
        WorkHandle,
    ) {
        let arc_widget = self.into_arc_widget();
        let (node, handle) = ElementNode::<<T as Widget>::Element>::new_async_uninflated(
            arc_widget.clone(),
            work_context,
            parent_context,
            barrier,
        );

        (
            Box::new(ElementWidgetPair {
                element: node,
                widget: arc_widget,
            }),
            handle,
        )
    }
}

pub trait ChildElementWidgetPairAsyncInflateExt<P: Protocol> {
    fn inflate_async(
        self,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    );

    fn inflate_async_box(
        self: Box<Self>,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    );
}

impl<E> ChildElementWidgetPairAsyncInflateExt<E::ParentProtocol> for ElementWidgetPair<E>
where
    E: FullElement,
{
    fn inflate_async(
        self,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    ) {
        todo!()
    }

    fn inflate_async_box(
        self: Box<Self>,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    ) {
        todo!()
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
                E::get_consumed_types(&widget),
                EMPTY_CONSUMED_TYPES,
                &work_context.recorded_provider_values,
                &element_context.provider_map,
            );
            Self {
                context: Arc::new(element_context),
                snapshot: SyncMutex::new(ElementSnapshot {
                    widget,
                    inner: ElementSnapshotInner::AsyncInflating(AsyncInflating {
                        work_context,
                        stash: AsyncStash {
                            handle,
                            subscription_diff,
                            updated_consumers: None,
                            output: AsyncOutput::Uninitiated { barrier },
                        },
                    }),
                }),
            }
        });
        (node, handle_clone)
        // We could either read the subscription here or in the inflate method since async inflating is a two-step process. \
        // Decision: in the inflate method.
    }

    pub(super) fn inflate_node_async_(
        self: &Arc<Self>,
        widget: &E::ArcWidget,
        work_context: Asc<WorkContext>,
        handle: WorkHandle,
        barrier: CommitBarrier,
    ) {
        // let (provider_values, widget) = {
        //     let mut snapshot = self.snapshot.lock();
        //     let snapshot_reborrow = &mut *snapshot;
        //     if parent_handle.is_aborted() {
        //         return;
        //     }
        //     let async_inflating = snapshot_reborrow
        //         .inner
        //         .async_inflating_mut()
        //         .expect("Async inflate should only be called on a AsyncInflating node");
        //     let provider_values = self.read_consumed_values_async(
        //         E::get_consumed_types(&snapshot_reborrow.widget),
        //         EMPTY_CONSUMED_TYPES,
        //         &mut Cow::Borrowed(&work_context),
        //         &barrier,
        //     );
        //     (provider_values, snapshot.widget.clone())
        // };

        let provider_values = self.read_consumed_values_async(
            E::get_consumed_types(widget),
            EMPTY_CONSUMED_TYPES,
            &mut Cow::Borrowed(&work_context),
            &barrier,
        );

        self.perform_inflate_node_async::<true>(
            &widget,
            AsyncHookContext::new_inflate(),
            provider_values,
            work_context,
            handle,
            barrier,
        );
    }

    pub(super) fn perform_inflate_node_async<const IS_NEW_INFLATE: bool>(
        self: &Arc<Self>,
        widget: &E::ArcWidget,
        mut hook_context: AsyncHookContext,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        child_work_context: Asc<WorkContext>,
        handle: WorkHandle,
        barrier: CommitBarrier,
    ) {
        let result = E::perform_inflate_element(
            &widget,
            AsyncBuildContext {
                hooks: &mut hook_context,
                element_context: &self.context,
            }
            .into(),
            provider_values,
        );

        let lane_pos = child_work_context.lane_pos;

        let output = match result {
            Ok((element, child_widgets)) => {
                let async_threadpool = &get_current_scheduler().async_threadpool;
                let children = child_widgets.map_collect_with(
                    (child_work_context, barrier),
                    |(child_work_context, barrier), child_widget| {
                        let (pair, child_handle) = child_widget.inflate_async_placeholder(
                            child_work_context.clone(),
                            Some(self.context.clone()),
                            barrier.clone(),
                        );
                        let node = pair.element();
                        async_threadpool.spawn(move || {
                            pair.inflate_async_box(child_work_context, child_handle, barrier)
                        });
                        node
                    },
                );
                AsyncOutput::Completed(BuildResults::new_inflate(hook_context, element, children))
            }
            Err(err) => AsyncOutput::Suspended {
                suspend: Some(BuildSuspendResults::new(hook_context)),
                barrier: Some(barrier),
            },
        };

        self.write_back_build_results::<IS_NEW_INFLATE>(output, lane_pos, &handle);
    }
}
