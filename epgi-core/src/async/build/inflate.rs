use crate::{
    foundation::{
        Arc, Asc, InlinableDwsizeVec, Protocol, Provide, SyncMutex, EMPTY_CONSUMED_TYPES,
    },
    sync::CommitBarrier,
    tree::{
        ArcElementContextNode, AsyncInflating, AsyncOutput, AsyncStash, ChildElementWidgetPair,
        ElementBase, ElementContextNode, ElementNode, ElementSnapshot, ElementSnapshotInner,
        ElementWidgetPair, FullElement, Widget, WorkContext, WorkHandle,
    },
};

pub trait ChildWidgetAsyncInflateExt<PP: Protocol> {
    fn inflate_async(
        self: Arc<Self>,
        work_context: Asc<WorkContext>,
        parent_context: Option<ArcElementContextNode>,
        barrier: CommitBarrier,
        handle: WorkHandle,
    ) -> Box<dyn ChildElementWidgetPair<PP>>;
}

impl<T> ChildWidgetAsyncInflateExt<<T::Element as ElementBase>::ParentProtocol> for T
where
    T: Widget,
{
    fn inflate_async(
        self: Arc<Self>,
        work_context: Asc<WorkContext>,
        parent_context: Option<ArcElementContextNode>,
        barrier: CommitBarrier,
        handle: WorkHandle,
    ) -> Box<dyn ChildElementWidgetPair<<T::Element as ElementBase>::ParentProtocol>>
    {
        let node = ElementNode::<<T as Widget>::Element>::new_async_uninflated(
            self.clone().into_arc_widget(),
            work_context,
            parent_context,
            handle,
            barrier,
        );
        return Box::new(ElementWidgetPair {
            widget: self.into_arc_widget(),
            element: node,
        });
    }
}

impl<E: FullElement> ElementNode<E> {
    fn new_async_uninflated(
        widget: E::ArcWidget,
        work_context: Asc<WorkContext>,
        parent_context: Option<ArcElementContextNode>,
        handle: WorkHandle,
        barrier: CommitBarrier,
    ) -> Arc<Self> {
        // We cannot reserve our subscription before the node is fully constructed.
        // Otherwise a contending async writing commit may find an uninstantiated node in its reservation list. Which is odd.

        Arc::new_cyclic(move |node| {
            let element_context =
                ElementContextNode::new_for::<E>(node.clone() as _, parent_context, &widget);
            let subscription_diff = Self::calc_subscription_diff(
                E::get_consumed_types(&widget),
                EMPTY_CONSUMED_TYPES,
                &work_context.reserved_provider_values,
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
                            reserved_provider_write: false,
                            output: AsyncOutput::Uninitiated { barrier },
                        },
                    }),
                }),
            }
        })
        // We could either read the subscription here or in the inflate method since async inflating is a two-step process. \
        // Decision: in the inflate method.
    }

    pub(super) fn inflate_node_async_(
        self: &Arc<Self>,
        work_context: Asc<WorkContext>,
        parent_handle: &WorkHandle,
        barrier: CommitBarrier,
    ) {
        let (provider_values, widget) = {
            let mut snapshot = self.snapshot.lock();
            let snapshot_reborrow = &mut *snapshot;
            if parent_handle.is_aborted() {
                return;
            }
            let async_inflating = snapshot_reborrow
                .inner
                .async_inflating_mut()
                .expect("Async inflate should only be called on a AsyncInflating node");
            let provider_values = self.read_consumed_values_async(
                E::get_consumed_types(&snapshot_reborrow.widget),
                EMPTY_CONSUMED_TYPES,
                &work_context,
                &barrier,
            );
            (provider_values, snapshot.widget.clone())
        };

        self.perform_inflate_node_async::<true>(
            &widget,
            work_context,
            provider_values,
            parent_handle,
            barrier,
        );
    }

    pub(super) fn perform_inflate_node_async<const IS_NEW_INFLATE: bool>(
        self: &Arc<Self>,
        widget: &E::ArcWidget,
        work_context: Asc<WorkContext>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        handle: &WorkHandle,
        barrier: CommitBarrier,
    ) {
        let lane_pos = work_context.lane_pos;

        // let mut hooks_iter = HookContext::new_inflate();
        // let mut child_tasks = Default::default();
        // let mut nodes_needing_unmount = Default::default();
        // let reconciler = AsyncReconciler {
        //     host_handle: handle,
        //     work_context,
        //     child_tasks: &mut child_tasks,
        //     barrier,
        //     host_context: &self.context,
        //     hooks: &mut hooks_iter,
        //     nodes_needing_unmount: &mut nodes_needing_unmount,
        // };
        // let results = E::perform_inflate_element(widget, provider_values, reconciler);
        // let new_stash = match results {
        //     Ok(element) => AsyncOutput::Completed {
        //         children: element.children(),
        //         results: BuildResults::from_pieces(hooks_iter, element, nodes_needing_unmount),
        //     },
        //     Err(err) => AsyncOutput::Suspended {
        //         suspend: Some(BuildSuspendResults::new(hooks_iter)),
        //         barrier: None,
        //     },
        // };

        // self.write_back_build_results::<IS_NEW_INFLATE>(new_stash, lane_pos, handle, todo!());
        todo!("Child Tasks");
    }
}
