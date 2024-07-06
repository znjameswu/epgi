use std::borrow::Cow;

use futures::stream::Aborted;

use crate::{
    foundation::{Arc, Asc, Container, EMPTY_CONSUMED_TYPES},
    r#async::AsyncReconcile,
    scheduler::LanePos,
    sync::CommitBarrier,
    tree::{
        ArcSuspendWaker, AsyncInflating, AsyncOutput, AsyncQueueCurrentEntry, BuildSuspendResults,
        Element, ElementLockHeldToken, ElementNode, ElementSnapshotInner, FullElement, ImplProvide,
        Mainline,
    },
};

use super::AsyncReconcileVariant;

pub trait AnyElementAsyncPollExt {
    fn poll_async(self: Arc<Self>, waker: ArcSuspendWaker, barrier: CommitBarrier);
}

impl<E: FullElement> AnyElementAsyncPollExt for ElementNode<E> {
    fn poll_async(self: Arc<Self>, waker: ArcSuspendWaker, barrier: CommitBarrier) {
        let _ = self.poll_async_impl(waker, barrier);
    }
}

impl<E: FullElement> ElementNode<E> {
    fn poll_async_impl(
        self: &Arc<Self>,
        waker: ArcSuspendWaker,
        barrier: CommitBarrier,
    ) -> Result<(), Aborted> {
        let (reconcile, is_async_inflating) = {
            let mut snapshot = self.snapshot.lock();
            let snapshot_reborrow = &mut *snapshot;

            let lane_pos = waker.lane_pos();
            if lane_pos.is_sync() || waker.is_aborted() {
                return Err(Aborted);
            }
            waker.abort();

            let reconcile = match &mut snapshot_reborrow.inner {
                ElementSnapshotInner::AsyncInflating(async_inflating) => self
                    .setup_poll_async_async_inflating(
                        async_inflating,
                        &snapshot_reborrow.widget,
                        lane_pos,
                        barrier,
                        &snapshot_reborrow.element_lock_held,
                    )?,
                ElementSnapshotInner::Mainline(mainline) => self.setup_poll_async_mainline(
                    mainline,
                    &snapshot_reborrow.widget,
                    lane_pos,
                    barrier,
                    &snapshot_reborrow.element_lock_held,
                )?,
            };

            (reconcile, snapshot_reborrow.inner.is_async_inflating())
        };

        if !is_async_inflating {
            self.execute_reconcile_async(reconcile)
        } else {
            let AsyncReconcile {
                widget: None,
                child_work_context,
                handle,
                barrier,
                old_widget,
                provider_values,
                variant:
                    AsyncReconcileVariant::Inflate {
                        suspended_hooks,
                        allow_commit_suspend,
                    },
            } = reconcile
            else {
                panic!("Impossible to fail")
            };
            self.perform_inflate_node_async::<true>(
                &old_widget,
                Some(suspended_hooks),
                provider_values,
                child_work_context,
                handle,
                barrier,
                allow_commit_suspend,
            )
        }

        Ok(())
    }

    fn setup_poll_async_mainline(
        self: &Arc<Self>,
        mainline: &mut Mainline<E>,
        old_widget: &E::ArcWidget,
        lane_pos: LanePos,
        barrier: CommitBarrier,
        element_lock_held: &ElementLockHeldToken,
    ) -> Result<AsyncReconcile<E>, Aborted> {
        let current = mainline
            .async_queue
            .current_mut()
            .expect("Async-polled node should have an async work");

        let state = mainline.state.as_ref().expect(
            "There should not be a sync visit occupying this node
                if the previous async work has not been aborted",
        );

        let AsyncQueueCurrentEntry {
            ref widget, // ref is needed because rust-analyzer bugs out on mutable reference
            ref work_context,
            stash,
        } = current;

        debug_assert_eq!(
            work_context.lane_pos, lane_pos,
            "Polled async work should have the corresponding lane"
        );

        if stash.handle.is_aborted() {
            return Err(Aborted);
        }

        let AsyncOutput::Suspended {
            suspended_results,
            barrier: stored_suspended_barrier,
        } = &mut stash.output
        else {
            panic!("Async-polled node should have a suspended output")
        };
        let allow_commit_suspend = stored_suspended_barrier.is_none();
        let suspended_results = suspended_results
            .take()
            .expect("Async polling should not witness another polling taken the results");

        let BuildSuspendResults { hooks, waker } = suspended_results;

        waker.abort();

        let new_widget_ref = widget.as_ref().unwrap_or(old_widget);
        let new_consumed_types = E::get_consumed_types(new_widget_ref);
        let provided_value_update =
            <E as Element>::Impl::diff_provided_value(old_widget, new_widget_ref);

        let mut child_work_context = Cow::Borrowed(work_context.as_ref());
        let provider_values = self.read_consumed_values_async(
            &new_consumed_types,
            &new_consumed_types, // We previously has already read all the value once
            &mut child_work_context,
            &barrier,
            element_lock_held,
        );
        if let Some((new_provided_value, type_key, _is_new_value)) = provided_value_update {
            child_work_context
                .to_mut()
                .recorded_provider_values
                .insert(type_key, new_provided_value.clone());
        }

        let child_work_context = match child_work_context {
            Cow::Borrowed(_) => work_context.clone(),
            Cow::Owned(work_context) => Asc::new(work_context),
        };

        use crate::tree::MainlineState::*;
        Ok(AsyncReconcile {
            handle: stash.handle.clone(),
            widget: widget.clone(),
            child_work_context,
            old_widget: old_widget.clone(),
            provider_values,
            barrier,
            variant: match state {
                Ready {
                    element, children, ..
                }
                | RebuildSuspended {
                    element, children, ..
                } => AsyncReconcileVariant::Rebuild {
                    element: element.clone(),
                    hooks,
                    children: children.map_ref_collect(Clone::clone),
                },
                InflateSuspended { .. } => AsyncReconcileVariant::Inflate {
                    suspended_hooks: hooks,
                    allow_commit_suspend,
                },
            },
        })
    }

    fn setup_poll_async_async_inflating(
        self: &Arc<Self>,
        async_inflating: &mut AsyncInflating<E>,
        widget: &E::ArcWidget,
        lane_pos: LanePos,
        barrier: CommitBarrier,
        element_lock_held: &ElementLockHeldToken,
    ) -> Result<AsyncReconcile<E>, Aborted> {
        let AsyncInflating {
            ref work_context,
            stash,
        } = async_inflating;

        debug_assert_eq!(
            work_context.lane_pos, lane_pos,
            "Polled async work should have the corresponding lane"
        );

        if stash.handle.is_aborted() {
            return Err(Aborted);
        }

        let AsyncOutput::Suspended {
            suspended_results,
            barrier: stored_suspended_barrier,
        } = &mut stash.output
        else {
            panic!("Async-polled node should have a suspended output")
        };
        let allow_commit_suspend = stored_suspended_barrier.is_none();
        let suspended_results = suspended_results
            .take()
            .expect("Async polling should not witness another polling taken the results");

        let BuildSuspendResults { hooks, waker } = suspended_results;

        waker.abort();

        let mut child_work_context = Cow::Borrowed(work_context.as_ref());
        let provider_values = self.read_consumed_values_async(
            &E::get_consumed_types(widget),
            EMPTY_CONSUMED_TYPES,
            &mut child_work_context,
            &barrier,
            &element_lock_held,
        );
        let child_work_context = match child_work_context {
            Cow::Borrowed(_) => work_context.clone(),
            Cow::Owned(work_context) => Asc::new(work_context),
        };

        Ok(AsyncReconcile {
            widget: None,
            child_work_context,
            handle: stash.handle.clone(),
            barrier,
            old_widget: widget.clone(),
            provider_values,
            variant: AsyncReconcileVariant::Inflate {
                suspended_hooks: hooks,
                allow_commit_suspend,
            },
        })
    }
}
