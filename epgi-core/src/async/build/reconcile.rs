use std::borrow::Cow;

use futures::stream::Aborted;

use crate::{
    foundation::{Arc, Asc, Container, ContainerOf, InlinableDwsizeVec, Provide},
    scheduler::{get_current_scheduler, LanePos},
    sync::CommitBarrier,
    tree::{
        apply_hook_updates_async, no_widget_update, ArcChildElementNode, AsyncOutput,
        AsyncQueueCurrentEntry, AsyncStash, Element, ElementBase, ElementLockHeldToken,
        ElementNode, FullElement, HooksWithEffects, ImplProvide, Mainline, WorkContext, WorkHandle,
    },
};

impl<E: FullElement> ElementNode<E> {
    pub(in crate::r#async) fn reconcile_node_async(
        self: &Arc<Self>,
        widget: Option<E::ArcWidget>,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    ) -> Result<(), Aborted> {
        let setup_result =
            self.setup_reconcile_async(widget, work_context, parent_handle, barrier)?;

        use SetupAsyncReconcileResult::*;
        match setup_result {
            Reconcile(Ok(reconcile)) => Self::execute_reconcile_async(&self, reconcile),
            SkipAndVisit {
                barrier,
                work_context,
                parent_handle,
            } => {
                // Why do we need yield to scheduler to continue visit descendant roots?
                // Because visiting a subtree needs to access each node's children,
                // and the only way for async code to access children of a node is to occupy it (unlike sync code).
                // Occupy every node along the line is doable and would even improve the async batch performance.
                // But more likely than not, it would cause more interference, often severe interference, with sync batch.
                // Therefore, it is better to not occupy, and yield to scheduler to walk the tree in sync mode.
                get_current_scheduler().schedule_async_continue_work(
                    Arc::downgrade(self) as _,
                    work_context,
                    parent_handle,
                    barrier,
                );
            }
            SkipAndReturn => {}
            Reconcile(Err(OccupyError::Blocked)) => {
                get_current_scheduler().schedule_reorder_async_work(Arc::downgrade(self) as _);
                return Err(Aborted);
            }
            Reconcile(Err(OccupyError::Yielded)) => {}
        };
        Ok(())
    }
}

pub(crate) struct AsyncReconcile<E: ElementBase> {
    pub(super) widget: Option<E::ArcWidget>,
    pub(super) child_work_context: Asc<WorkContext>,
    pub(super) handle: WorkHandle,
    pub(super) barrier: CommitBarrier,
    pub(super) old_widget: E::ArcWidget,
    pub(super) provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    pub(super) states: Result<
        (
            E,
            HooksWithEffects,
            ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
        ),
        HooksWithEffects,
    >,
}

enum SetupAsyncReconcileResult<E: ElementBase> {
    Reconcile(Result<AsyncReconcile<E>, OccupyError>),
    SkipAndVisit {
        barrier: CommitBarrier,
        parent_handle: WorkHandle,
        work_context: Asc<WorkContext>,
    },
    SkipAndReturn,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OccupyError {
    Yielded,
    Blocked,
}

impl<E: FullElement> ElementNode<E> {
    fn setup_reconcile_async(
        self: &Arc<Self>,
        widget: Option<E::ArcWidget>,
        work_context: Asc<WorkContext>,
        parent_handle: WorkHandle,
        barrier: CommitBarrier,
    ) -> Result<SetupAsyncReconcileResult<E>, Aborted> {
        use SetupAsyncReconcileResult::*;
        let no_new_widget = widget.is_none();
        let no_mailbox_update = !self.context.mailbox_lanes().contains(work_context.lane_pos);
        let no_consumer_root = !self
            .context
            .consumer_lanes()
            .contains(work_context.lane_pos);
        let no_descendant_lanes = !self
            .context
            .descendant_lanes()
            .contains(work_context.lane_pos);

        if no_new_widget && no_mailbox_update && no_consumer_root {
            // Skips rebuilding entirely by not occupying the node.
            if no_descendant_lanes {
                return Ok(SkipAndReturn);
            }
            return Ok(SkipAndVisit {
                barrier,
                parent_handle,
                work_context,
            });
        }

        let mut snapshot = self.snapshot.lock();
        let snapshot_reborrow = &mut *snapshot;
        if parent_handle.is_aborted() {
            return Err(Aborted);
        }
        let mainline = snapshot_reborrow
            .inner
            .mainline_mut()
            .expect("A nonmainline node should not be reachable by a rebuild!");
        let no_widget_update =
            no_widget_update::<E>(widget.as_ref(), &mut snapshot_reborrow.widget);

        if no_widget_update && no_mailbox_update && no_consumer_root {
            // Skips rebuilding entirely by not occupying the node.
            // Safety of not occupying the node:
            // 1. If we reconciled with a widget (whether new or not, i.e. an Update), then we are confident that no other batch will update the widget while the parent work of this work is still alive.
            //      The reason is that an "Update" variant of work can only be a direct child of another work.
            //      If we have a widget to reconcile, then we are also the direct child of another work.
            //      Therefore, if another work updated the widget, it must have already content with our parent work.
            // 2. If we do not have a widget to reconcile (i.e., a pure refresh), then we have no reason to fear for another batch to change our widget.
            if no_descendant_lanes {
                return Ok(SkipAndReturn);
            }
            return Ok(SkipAndVisit {
                barrier,
                parent_handle,
                work_context,
            });
        }

        Ok(Reconcile(self.setup_occupy_async(
            mainline,
            &snapshot_reborrow.widget,
            widget,
            work_context,
            barrier,
            &snapshot_reborrow.element_lock_held,
        )))
    }

    /// A piece of logic that is shared by async build and reorder.
    pub(crate) fn setup_occupy_async(
        self: &Arc<Self>,
        mainline: &mut Mainline<E>,
        old_widget: &E::ArcWidget,
        widget: Option<E::ArcWidget>,
        work_context: Asc<WorkContext>,
        barrier: CommitBarrier,
        element_lock_held: &ElementLockHeldToken,
    ) -> Result<AsyncReconcile<E>, OccupyError> {
        let Mainline { state, async_queue } = mainline;

        let Some(state) = state else {
            // Occupied by an sync operation, do not retry. The sync operation will dispatch queued async works in the end
            async_queue.push_backqueue(widget, work_context, barrier);
            return Err(OccupyError::Blocked);
        };

        let try_push_result = async_queue.try_push_front_with(
            widget.clone(),
            work_context,
            barrier.clone(),
            |widget, work_context, barrier| {
                let old_consumed_types = E::get_consumed_types(old_widget);
                let new_widget_ref = widget.as_ref().unwrap_or(old_widget);
                let new_consumed_types = E::get_consumed_types(new_widget_ref);
                let subscription_diff = Self::calc_subscription_diff(
                    &new_consumed_types,
                    &old_consumed_types,
                    &work_context.recorded_provider_values,
                    &self.context.provider_map,
                );

                let provided_value_update =
                    <E as Element>::Impl::diff_provided_value(old_widget, new_widget_ref);

                let mut child_work_context = Cow::Borrowed(work_context.as_ref());
                let provider_values = self.read_consumed_values_async(
                    &new_consumed_types,
                    &old_consumed_types,
                    &mut child_work_context,
                    &barrier,
                    element_lock_held,
                );
                let mut spawned_consumers = None;
                if let Some((new_provided_value, type_key, is_new_value)) = provided_value_update {
                    child_work_context
                        .to_mut()
                        .recorded_provider_values
                        .insert(type_key, new_provided_value.clone());
                    if is_new_value {
                        let mainline_readers = self.context.reserve_write_async(
                            work_context.lane_pos,
                            new_provided_value,
                            work_context.batch.as_ref(),
                            &barrier,
                        );
                        spawned_consumers = Some(
                            mainline_readers
                                .into_iter()
                                .filter_map(|mainline_reader_weak| {
                                    let mainline_reader = mainline_reader_weak
                                        .upgrade()
                                        .expect("Readers should be alive");
                                    let spawn_token = mainline_reader.mark_consumer(
                                        work_context.lane_pos,
                                        mainline_reader.assert_not_unmounted(),
                                    );
                                    let spawn_token = spawn_token?;
                                    Some((mainline_reader_weak, spawn_token))
                                })
                                .collect(),
                        );
                    }
                }
                let child_work_context = match child_work_context {
                    Cow::Borrowed(_) => work_context.clone(),
                    Cow::Owned(work_context) => Asc::new(work_context),
                };
                let handle = WorkHandle::new();

                (
                    AsyncQueueCurrentEntry {
                        widget,
                        work_context,
                        stash: AsyncStash {
                            handle: handle.clone(),
                            subscription_diff,
                            spawned_consumers,
                            output: AsyncOutput::Uninitiated { barrier },
                        },
                    },
                    (handle, provider_values, child_work_context),
                )
            },
        );

        match try_push_result {
            Ok((handle, provider_values, child_work_context)) => {
                use crate::tree::MainlineState::*;
                let reconcile = match state {
                    InflateSuspended {
                        suspended_hooks,
                        waker: _,
                    } => AsyncReconcile {
                        handle,
                        widget,
                        child_work_context,
                        old_widget: old_widget.clone(),
                        provider_values,
                        barrier,
                        states: Err(suspended_hooks.read(|| None)),
                    },
                    Ready {
                        element,
                        hooks,
                        children,
                        ..
                    }
                    | RebuildSuspended {
                        element,
                        suspended_hooks: hooks,
                        children,
                        ..
                    } => AsyncReconcile {
                        handle,
                        widget,
                        child_work_context,
                        old_widget: old_widget.clone(),
                        provider_values,
                        barrier,
                        states: Ok((
                            element.clone(),
                            hooks.read(|| None),
                            children.map_ref_collect(Clone::clone),
                        )),
                    },
                };
                return Ok(reconcile);
            }
            Err((current, backqueue)) => {
                let should_yield_to_current =
                    current.work_context.batch.priority < backqueue.work_context.batch.priority;

                return if should_yield_to_current {
                    Err(OccupyError::Yielded)
                } else {
                    Err(OccupyError::Blocked)
                };
            }
        }
    }

    pub(super) fn execute_reconcile_async(self: &Arc<Self>, reconcile: AsyncReconcile<E>) {
        //TODO: Merge updates and bypass clean nodes.

        let AsyncReconcile {
            handle,
            barrier,
            widget,
            child_work_context,
            old_widget,
            provider_values,
            states,
        } = reconcile;

        match states {
            Ok((last_element, mut hooks, children)) => {
                apply_hook_updates_async(&self.context, child_work_context.job_ids(), &mut hooks);
                self.perform_rebuild_node_async(
                    widget.as_ref().unwrap_or(&old_widget),
                    last_element,
                    hooks,
                    children,
                    provider_values,
                    child_work_context,
                    handle,
                    barrier,
                )
            }
            Err(mut suspended_hooks) => {
                apply_hook_updates_async(
                    &self.context,
                    child_work_context.job_ids(),
                    &mut suspended_hooks,
                );
                self.perform_inflate_node_async::<false>(
                    &widget.unwrap_or(old_widget),
                    Some(suspended_hooks),
                    provider_values,
                    child_work_context,
                    handle,
                    barrier,
                )
            }
        }
    }

    pub(crate) fn execute_reconcile_node_async_detached(
        self: Arc<Self>,
        rebuild: AsyncReconcile<E>,
    ) {
        get_current_scheduler()
            .async_threadpool
            .spawn(move || self.execute_reconcile_async(rebuild))
    }

    pub(super) fn write_back_build_results<const IS_NEW_INFLATE: bool>(
        self: &Arc<Self>,
        new_stash: AsyncOutput<E>,
        lane_pos: LanePos,
        handle: &WorkHandle,
        // // When this is true, it means that we are inflating a new suspense above this suspended node. And we should commit the suspended result as-is rather than wait for it.
        // allow_suspend: bool,
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
                entry.work_context.lane_pos, lane_pos,
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
        // todo!()
    }
}
