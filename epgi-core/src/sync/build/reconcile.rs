use crate::{
    foundation::{Arc, Container, ContainerOf, Inlinable64Vec},
    scheduler::{get_current_scheduler, JobId, LanePos},
    sync::{LaneScheduler, SubtreeRenderObjectChange, SyncHookContext},
    tree::{
        apply_hook_updates, no_widget_update, ArcChildElementNode, Element, ElementContextNode, ElementNode, FullElement, HooksWithTearDowns, ImplElementNode, MainlineState
    },
};

use super::{
    provider::{read_and_update_subscriptions_sync, update_provided_value},
    CancelAsync, ImplReconcileCommit,
};

impl<E: FullElement> ElementNode<E> {
    pub(super) fn reconcile_node_sync(
        self: &Arc<Self>,
        widget: Option<E::ArcWidget>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        lane_scheduler: &LaneScheduler,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        let prepare_result = self.prepare_reconcile(widget, lane_scheduler);
        let change = match prepare_result {
            PrepareReconcileResult::SkipAndVisitChildren {
                children,
                render_object,
                self_rebuild_suspended,
            } => {
                let results = children
                    .par_map_collect(&get_current_scheduler().sync_threadpool, |child| {
                        child.visit_and_work_sync(job_ids, scope, lane_scheduler)
                    });
                let (_children, render_object_changes) = results.unzip_collect(|x| x);

                return <E as Element>::Impl::visit_commit(
                    &self,
                    render_object,
                    render_object_changes,
                    lane_scheduler,
                    scope,
                    self_rebuild_suspended,
                );
            }
            PrepareReconcileResult::Reconcile(reconcile) => {
                self.execute_reconcile(reconcile, job_ids, scope, lane_scheduler)
            }
            PrepareReconcileResult::SkipAndReturn => SubtreeRenderObjectChange::new_no_update(),
        };

        self.context.purge_lane(LanePos::Sync);
        return change;
    }
}

struct SyncReconcile<E: Element> {
    is_poll: bool,
    old_widget: E::ArcWidget,
    new_widget: Option<E::ArcWidget>,
    state: MainlineState<E, HooksWithTearDowns>,
    cancel_async:
        Option<CancelAsync<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>>,
}

enum PrepareReconcileResult<E: Element> {
    Reconcile(SyncReconcile<E>),
    /// Visit is needed when the node itself does not need reconcile, but
    /// lane marking has indicated that one of its descendants needs needs reconcile.
    ///
    /// The visit variant will under no circumstance change the mainline state.
    /// Therefore, this variant won't occupy the element node. As a result, exisiting async work won't be interrupted
    /// However, the visit variant WILL have other commit effects, such as createing/updating/detaching render object.
    SkipAndVisitChildren {
        children: ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
        // This has two variant in case the render object is detached
        // We do not store MaybeSuspendeChildRenderObject, because everytime we need to access it, (update children suspend state)
        // we have to write it into the node anyway. Could just lock the mutex
        // We also do not store the widget, because everytime we need to access it, (create render object)
        // we have to write the render object into the node anyway
        render_object: <<E as Element>::Impl as ImplElementNode<E>>::OptionArcRenderObject,
        // Optimization parameter to let commit bail out early if the element is known to have suspended
        self_rebuild_suspended: bool,
    },
    /// End-of-visit is triggered when both the node and its descendants (i.e. entire subtree) does not need reconcile
    SkipAndReturn,
}

impl<E: FullElement> ElementNode<E> {
    fn prepare_reconcile(
        self: &Arc<Self>,
        widget: Option<E::ArcWidget>,
        lane_scheduler: &LaneScheduler,
    ) -> PrepareReconcileResult<E> {
        // An oppurtunistic probe to allow bypass lock.
        let no_new_widget = widget.is_none();
        let no_mailbox_update = !self.context.mailbox_lanes().contains(LanePos::Sync);
        let no_consumer_root = !self.context.consumer_root_lanes().contains(LanePos::Sync);
        let no_poll = !self.context.needs_poll();
        let no_descendant_lanes = !self.context.descendant_lanes().contains(LanePos::Sync);

        if no_new_widget && no_mailbox_update && no_consumer_root && no_poll && no_descendant_lanes
        {
            // Subtree has no work, end of visit
            return PrepareReconcileResult::SkipAndReturn;
        }

        let mut snapshot = self.snapshot.lock();
        // https://bevy-cheatbook.github.io/pitfalls/split-borrows.html
        let snapshot_reborrow = &mut *snapshot;

        let mainline = snapshot_reborrow
            .inner
            .mainline_mut()
            .expect("An unmounted element node should not be reachable by a rebuild!");

        let state = mainline.state.as_ref().expect(
            "A sync task should not encounter another sync task contending over the same node",
        );

        let no_widget_update =
            no_widget_update::<E>(widget.as_ref(), &mut snapshot_reborrow.widget);

        // Self has no work, but subtree has work. Visit
        if no_widget_update && no_mailbox_update && no_consumer_root && no_poll {
            if no_descendant_lanes {
                // Subtree has no work, end of visit
                return PrepareReconcileResult::SkipAndReturn;
            }
            use MainlineState::*;
            return match state {
                Ready {
                    children,
                    render_object,
                    ..
                } => PrepareReconcileResult::SkipAndVisitChildren::<E> {
                    children: children.map_ref_collect(Clone::clone),
                    render_object: render_object.clone(),
                    self_rebuild_suspended: false,
                },
                RebuildSuspended { children, .. } => PrepareReconcileResult::SkipAndVisitChildren {
                    children: children.map_ref_collect(Clone::clone),
                    render_object: Default::default(),
                    self_rebuild_suspended: true,
                },
                InflateSuspended { .. } => {
                    debug_assert!(
                        false,
                        "Serious logic bug. \
                        The following three conditions cannot be true at the same time:\
                        1. Self has no work. \
                        2. Subtree has work. \
                        3. Self suspended during the last inflate attempt."
                    );
                    PrepareReconcileResult::SkipAndReturn
                }
            };
        }

        let state = (&mut mainline.state).take().expect("Impossible to fail"); // rust-analyzer#14933
                                                                               // Not able to use `Option::map` due to closure lifetime problem.
        let cancel_async = if let Some(entry) = mainline.async_queue.current() {
            let cancel = Self::prepare_cancel_async_work(
                mainline,
                entry.work_context.lane_pos,
                lane_scheduler,
            )
            .ok()
            .expect("Impossible to fail");
            Some(cancel)
        } else {
            None
        };

        // Cannot skip work but can skip rebuild, meaning there is a polling work here.
        if no_widget_update && no_mailbox_update {
            return PrepareReconcileResult::Reconcile(SyncReconcile {
                is_poll: true,
                old_widget: snapshot_reborrow.widget.clone(),
                new_widget: widget,
                state,
                cancel_async,
            });
        }
        let old_widget = if let Some(widget) = &widget {
            std::mem::replace(&mut snapshot_reborrow.widget, widget.clone())
        } else {
            snapshot_reborrow.widget.clone()
        };
        return PrepareReconcileResult::Reconcile(SyncReconcile {
            is_poll: false,
            old_widget,
            new_widget: widget,
            state,
            cancel_async,
        });
    }

    #[inline(always)]
    fn execute_reconcile(
        self: &Arc<Self>,
        reconcile: SyncReconcile<E>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        lane_scheduler: &LaneScheduler,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        let SyncReconcile {
            is_poll,
            old_widget,
            new_widget,
            state,
            cancel_async,
        } = reconcile;

        if let Some(cancel_async) = cancel_async {
            self.perform_cancel_async_work(cancel_async)
        }
        let new_widget_ref = new_widget.as_ref().unwrap_or(&old_widget);
        let consumed_values = read_and_update_subscriptions_sync(
            E::get_consumed_types(new_widget_ref),
            E::get_consumed_types(&old_widget),
            &self.context,
            lane_scheduler,
        );
        if let Some(widget) = new_widget.as_ref() {
            update_provided_value::<E>(&old_widget, widget, &self.context, lane_scheduler)
        }
        let is_new_widget = new_widget.is_some();
        let new_widget = &new_widget.unwrap_or(old_widget);
        match state {
            MainlineState::Ready {
                element,
                children,
                mut hooks,
                render_object,
            } => {
                assert!(!is_poll, "A non-suspended node should not be polled");
                apply_hook_updates(&self.context, job_ids, &mut hooks);
                self.perform_rebuild_node_sync(
                    new_widget,
                    element,
                    children,
                    SyncHookContext::new_rebuild(hooks),
                    render_object,
                    consumed_values,
                    job_ids,
                    scope,
                    lane_scheduler,
                    is_new_widget,
                )
            }
            MainlineState::RebuildSuspended {
                element,
                children,
                mut suspended_hooks,
                waker,
            } => {
                waker.set_completed();
                // If it is not poll, then it means a new job occurred on this previously suspended node
                if !is_poll {
                    apply_hook_updates(&self.context, job_ids, &mut suspended_hooks);
                }
                self.perform_rebuild_node_sync(
                    new_widget,
                    element,
                    children,
                    SyncHookContext::new_rebuild(suspended_hooks),
                    Default::default(),
                    consumed_values,
                    job_ids,
                    scope,
                    lane_scheduler,
                    is_new_widget,
                )
            }
            MainlineState::InflateSuspended {
                suspended_hooks,
                waker,
            } => {
                waker.set_completed();
                self.perform_inflate_node_sync::<false>(
                    new_widget,
                    if !is_poll {
                        SyncHookContext::new_inflate()
                    } else {
                        SyncHookContext::new_poll_inflate(suspended_hooks)
                    },
                    consumed_values,
                    lane_scheduler,
                )
            }
        }
    }
}
