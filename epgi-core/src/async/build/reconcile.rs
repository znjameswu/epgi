use futures::stream::Aborted;
use hashbrown::HashMap;

use crate::{
    foundation::{
        Arc, Asc, InlinableDwsizeVec, InlinableUsizeVec, Provide, SyncMutex, TryResult, TypeKey,
        EMPTY_CONSUMED_TYPES,
    },
    scheduler::{get_current_scheduler, LanePos},
    sync::CommitBarrier,
    tree::{
        no_widget_update, ArcElementContextNode, AsyncInflating, AsyncOutput, AsyncStash, Element,
        ElementContextNode, ElementNode, ElementSnapshot, ElementSnapshotInner, Hooks, Mainline,
        ProviderElementMap, SubscriptionDiff, Work, WorkContext, WorkHandle,
    },
};

pub(crate) struct AsyncRebuild<E: Element> {
    pub(crate) handle: WorkHandle,
    pub(crate) barrier: CommitBarrier,
    pub(crate) work: Work<E::ArcWidget>,
    pub(crate) old_widget: E::ArcWidget,
    pub(crate) provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    pub(crate) states: Option<(Hooks, E)>,
}

pub(crate) struct AsyncSkip<E: Element> {
    barrier: CommitBarrier,
    work: Work<E::ArcWidget>,
}

pub(super) enum TryAsyncRebuild<E: Element> {
    Success(AsyncRebuild<E>),
    Skip {
        barrier: CommitBarrier,
        work: Work<E::ArcWidget>,
    },
    Blocked,
    Backqueued,
}

impl<E> ElementNode<E>
where
    E: Element,
{
    pub(super) fn new_async_uninflated(
        widget: E::ArcWidget,
        work_context: Asc<WorkContext>,
        parent_context: ArcElementContextNode,
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

    pub(super) fn rebuild_node_async(
        self: &Arc<Self>,
        work: Work<E::ArcWidget>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    ) -> Result<(), Aborted> {
        let rebuild = {
            let mut snapshot = self.snapshot.lock();
            let snapshot_reborrow = &mut *snapshot;
            if parent_handle.is_aborted() {
                return Err(Aborted);
            }
            let mainline = snapshot_reborrow
                .inner
                .mainline_mut()
                .expect("A nonmainline node should not be reachable by a rebuild!");

            self.prepare_rebuild_async(mainline, &snapshot_reborrow.widget, work, barrier)
            // if let Some(get_provided_types) = E::GET_PROVIDED_VALUE {

            // }
        };

        use TryResult::*;
        match rebuild {
            Success(Ok(rebuild)) => Self::execute_rebuild_node_async(&self, rebuild),
            Success(Err(AsyncSkip { barrier, work })) => {
                get_current_scheduler().schedule_async_yield_subtree(
                    Arc::downgrade(self) as _,
                    work.context,
                    parent_handle,
                    barrier,
                );
            }
            Blocked(_) => {
                get_current_scheduler().schedule_reorder_async_work(Arc::downgrade(self) as _);
                return Err(Aborted);
            }
            Yielded(_) => {}
        };
        Ok(())
    }

    pub(crate) fn prepare_rebuild_async(
        self: &Arc<Self>,
        mainline: &mut Mainline<E>,
        old_widget: &E::ArcWidget,
        work: Work<E::ArcWidget>,
        barrier: CommitBarrier,
    ) -> TryResult<Result<AsyncRebuild<E>, AsyncSkip<E>>> {
        use TryResult::*;

        let Mainline { state, async_queue } = mainline;

        // Occupied by an sync operation, do not retry. The sync operation will dispatch queued async works in the end
        let Some(state) = state else {
            return Blocked(());
        };

        let no_mailbox_update = !self.context.mailbox_lanes().contains(work.context.lane_pos);
        let no_consumer_root = !self
            .context
            .consumer_root_lanes()
            .contains(work.context.lane_pos);
        let no_descendant_lanes = !self
            .context
            .descendant_lanes()
            .contains(work.context.lane_pos);
        let no_widget_update = no_widget_update::<E>(work.widget.as_ref(), old_widget);

        if no_mailbox_update && no_descendant_lanes && no_descendant_lanes && no_widget_update {
            // Skips rebuilding entirely by not occupying the node.
            // Safety of not occupying the node:
            // 1. If we reconciled with a widget (whether new or not, i.e. an Update), then we are confident that no other batch will update the widget while the parent work of this work is still alive.
            //      The reason is that an "Update" variant of work can only be a direct child of another work.
            //      If we have a widget to reconcile, then we are also the direct child of another work.
            //      Therefore, if another work updated the widget, it must have already content with our parent work.
            // 2. If we do not have a widget to reconcile (i.e., a pure refresh), then we have no reason to fear for another batch to change our widget.
            return Success(Err(AsyncSkip { barrier, work }));
        } else {
            let old_consumed_types = E::get_consumed_types(old_widget);
            let new_widget_ref = work.widget.as_ref().unwrap_or(old_widget);
            let new_consumed_types = E::get_consumed_types(new_widget_ref);
            let subscription_diff = Self::calc_subscription_diff(
                new_consumed_types,
                old_consumed_types,
                &work.context.reserved_provider_values,
                &self.context.provider_map,
            );
            let mut provider_value_to_write = None;
            if let Some(get_provided_value) = E::GET_PROVIDED_VALUE {
                let old_provided_value = get_provided_value(&old_widget);
                let new_provided_value = get_provided_value(new_widget_ref);
                if !Asc::ptr_eq(&old_provided_value, &new_provided_value)
                    && !old_provided_value.eq_sized(new_provided_value.as_ref())
                {
                    provider_value_to_write = Some(new_provided_value);
                }
            };
            // Cannot use `TryReuslt::map` due to lifetime problems from the limited closure expressiveness.
            match async_queue.try_push_front(
                &work,
                &barrier,
                subscription_diff,
                provider_value_to_write.is_some(),
            ) {
                Success(handle) => {
                    let provider_values = self.read_consumed_values_async(
                        new_consumed_types,
                        old_consumed_types,
                        &work.context,
                        &barrier,
                    );
                    if let Some(provider_value_to_write) = provider_value_to_write {
                        let mainline_readers = self.context.reserve_write_async(
                            work.context.lane_pos,
                            provider_value_to_write,
                            &barrier,
                        );
                        for mainline_reader in mainline_readers.into_iter() {
                            mainline_reader
                                .upgrade()
                                .expect("Readers should be alive")
                                .mark_consumer_root(work.context.lane_pos);
                        }
                    }
                    use crate::tree::MainlineState::*;
                    let rebuild = match state {
                        InflateSuspended {
                            suspended_hooks: last_hooks,
                            waker,
                        } => AsyncRebuild {
                            handle,
                            old_widget: old_widget.clone(),
                            provider_values,
                            barrier,
                            work,
                            states: None,
                        },
                        Ready { hooks, element, .. }
                        | RebuildSuspended {
                            suspended_hooks: hooks,
                            element,
                            ..
                        } => AsyncRebuild {
                            handle,
                            old_widget: old_widget.clone(),
                            provider_values,
                            barrier,
                            work,
                            states: Some((hooks.clone(), element.clone())),
                        },
                    };
                    Success(Ok(rebuild))
                }
                Blocked(_) => {
                    mainline.async_queue.push_backqueue(work, barrier);
                    Blocked(())
                }
                Yielded(_) => {
                    mainline.async_queue.push_backqueue(work, barrier);
                    Yielded(())
                }
            }
        }
    }

    pub(super) fn execute_rebuild_node_async(self: &Arc<Self>, rebuild: AsyncRebuild<E>) {
        //TODO: Merge updates and bypass clean nodes.

        let AsyncRebuild {
            handle,
            barrier,
            work,
            old_widget,
            provider_values,
            states,
        } = rebuild;

        if let Some((hooks, last_element)) = states {
            self.perform_rebuild_node_async(
                work.widget.as_ref().unwrap_or(&old_widget),
                work.context,
                hooks,
                last_element,
                provider_values,
                &handle,
                barrier,
            )
        } else {
            self.perform_inflate_node_async::<false>(
                &work.widget.unwrap_or(old_widget),
                work.context,
                provider_values,
                &handle,
                barrier,
            )
        }
    }

    pub(crate) fn execute_rebuild_node_async_detached(self: Arc<Self>, rebuild: AsyncRebuild<E>) {
        get_current_scheduler()
            .async_threadpool
            .spawn(move || self.execute_rebuild_node_async(rebuild))
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

    fn perform_rebuild_node_async(
        self: &Arc<Self>,
        widget: &E::ArcWidget,
        work_context: Asc<WorkContext>,
        mut hooks: Hooks,
        element: E,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        handle: &WorkHandle,
        barrier: CommitBarrier,
    ) {
        let lane_pos = work_context.lane_pos;

        let mut jobs = {
            self.context
                .mailbox
                .lock()
                .iter()
                .filter_map(|(job_id, update)| {
                    work_context
                        .job_ids()
                        .contains(job_id)
                        .then_some((*job_id, update.clone()))
                })
                .collect::<Vec<_>>()
        };
        jobs.sort_by_key(|(job_id, ..)| *job_id);

        let updates = jobs
            .into_iter()
            .flat_map(|(_, updates)| updates)
            .collect::<Vec<_>>();

        // let mut hooks = state.hooks;

        for update in updates {
            todo!()
        }

        // let mut hooks_iter = HookContext::new_rebuild(hooks);
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
        // let results = element.perform_rebuild_element(widget, provider_values, reconciler);
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

        // self.write_back_build_results::<false>(new_stash, lane_pos, handle, todo!());
        todo!("Child Tasks");
    }

    fn perform_inflate_node_async<const IS_NEW_INFLATE: bool>(
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

    fn write_back_build_results<const IS_NEW_INFLATE: bool>(
        self: &Arc<Self>,
        new_stash: AsyncOutput<E>,
        lane_pos: LanePos,
        handle: &WorkHandle,
        // When this is true, it means that we are inflating a new suspense above this suspended node. And we should commit the suspended result as-is rather than wait for it.
        allow_suspend: bool,
    ) {
        let mut snapshot = self.snapshot.lock();
        if handle.is_aborted() {
            return;
        }
        let output = if !IS_NEW_INFLATE {
            let entry = snapshot
                .inner
                .mainline_mut()
                .expect("Async work should be still alive")
                .async_queue
                .current_mut()
                .expect("Async work should be still alive");
            assert_eq!(
                entry.work.context.lane_pos, lane_pos,
                "The same async work should be still alive"
            );
            &mut entry.stash.output
        } else {
            let inflating = snapshot
                .inner
                .async_inflating_mut()
                .expect("Async work should be still alive");
            assert_eq!(
                inflating.work_context.lane_pos, lane_pos,
                "The same async work should be still alive"
            );
            &mut inflating.stash.output
        };

        *output = new_stash;
        todo!()
    }

    fn calc_subscription_diff(
        new_consumed_types: &[TypeKey],
        old_consumed_types: &[TypeKey],
        reserved_provider_values: &HashMap<TypeKey, Asc<dyn Provide>>,
        provider_map: &ProviderElementMap,
    ) -> SubscriptionDiff {
        let is_old_consumed_types = std::ptr::eq(new_consumed_types, old_consumed_types);
        if is_old_consumed_types {
            return Default::default();
        }
        let remove = old_consumed_types
            .iter()
            .filter(|consumed_type| !new_consumed_types.contains(consumed_type))
            .map(|consumed_type| {
                provider_map
                    .get(consumed_type)
                    .expect("Requested provider should exist")
                    .clone()
            })
            .collect();
        let mut register = InlinableUsizeVec::<ArcElementContextNode>::default();
        let mut reserve = InlinableUsizeVec::<ArcElementContextNode>::default();

        // Filter and group-by
        for consumed_type in new_consumed_types.iter() {
            let is_old = old_consumed_types.contains(consumed_type);
            if !is_old {
                let subscription = provider_map
                    .get(consumed_type)
                    .expect("Requested provider should exist")
                    .clone();
                if reserved_provider_values.contains_key(consumed_type) {
                    register.push(subscription);
                } else {
                    reserve.push(subscription);
                }
            }
        }
        return SubscriptionDiff {
            register,
            reserve,
            remove,
        };
    }

    // Warning 1: This method will acquire provider locks one by one. Make sure your hold no other lock than the single element snapshot lock in question.
    // Warning 2: You must hold the element snapshot lock before calling this method.
    //      Otherwise another contending async writing commit may trace back to this node (by the reservation you left) at anytime
    //      The contending commit may decide cancel your async work while you are still reserving. And then create a mess of racing conditions.
    //
    //      This could have been solved by requiring a lock guard as parameter.
    //      However, the two callsites do not share a common inner type as guard.
    //
    //      The correct design under a cooperative cancellation framework should reqruie a cooperative WorkHandle while reserving.
    //      However, since we already hold the element snapshot lock. We decide to do this clever optimization.
    fn read_consumed_values_async(
        self: &Arc<Self>,
        new_consumed_types: &[TypeKey],
        old_consumed_types: &[TypeKey],
        work_context: &WorkContext,
        barrier: &CommitBarrier,
    ) -> InlinableDwsizeVec<Arc<dyn Provide>> {
        let is_old_consumed_types = std::ptr::eq(new_consumed_types, old_consumed_types);

        let consumed_values = new_consumed_types
            .iter()
            .map(|consumed_type| {
                work_context
                    .reserved_provider_values
                    .get(consumed_type)
                    .cloned()
                    .unwrap_or_else(|| {
                        let subscription = self
                            .context
                            .provider_map
                            .get(consumed_type)
                            .expect("The context node of the requested provider should exist");
                        if is_old_consumed_types || old_consumed_types.contains(consumed_type) {
                            subscription
                                .provider
                                .as_ref()
                                .expect("The requested provider should exist")
                                .read()
                        } else {
                            subscription.reserve_read(
                                Arc::downgrade(self) as _,
                                work_context.lane_pos,
                                barrier,
                            )
                        }
                    })
            })
            .collect();
        return consumed_values;
    }
}
