use futures::stream::Aborted;

use crate::{
    foundation::{Arc, InlinableDwsizeVec, Provide, TryResult},
    scheduler::{get_current_scheduler, LanePos},
    sync::CommitBarrier,
    tree::{
        no_widget_update, AsyncOutput, ElementBase, ElementNode, FullElement, HooksWithEffects,
        Mainline, Work, WorkHandle,
    },
};

pub(crate) struct AsyncRebuild<E: ElementBase> {
    pub(crate) handle: WorkHandle,
    pub(crate) barrier: CommitBarrier,
    pub(crate) work: Work<E::ArcWidget>,
    pub(crate) old_widget: E::ArcWidget,
    pub(crate) provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    pub(crate) states: Option<(HooksWithEffects, E)>,
}

pub(crate) struct AsyncSkip<E: ElementBase> {
    barrier: CommitBarrier,
    work: Work<E::ArcWidget>,
}

pub(super) enum TryAsyncRebuild<E: ElementBase> {
    Success(AsyncRebuild<E>),
    Skip {
        barrier: CommitBarrier,
        work: Work<E::ArcWidget>,
    },
    Blocked,
    Backqueued,
}

impl<E: FullElement> ElementNode<E> {
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
            todo!();
            // if let Some(get_provided_value) = E::GET_PROVIDED_VALUE {
            //     let old_provided_value = get_provided_value(&old_widget);
            //     let new_provided_value = get_provided_value(new_widget_ref);
            //     if !Asc::ptr_eq(&old_provided_value, &new_provided_value)
            //         && !old_provided_value.eq_sized(new_provided_value.as_ref())
            //     {
            //         provider_value_to_write = Some(new_provided_value);
            //     }
            // };
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
                            let mainline_reader =
                                mainline_reader.upgrade().expect("Readers should be alive");
                            mainline_reader.mark_consumer_root(
                                work.context.lane_pos,
                                mainline_reader.assert_not_unmounted(),
                            );
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
                            states: todo!(), //Some((hooks.clone(), element.clone())),
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
}
