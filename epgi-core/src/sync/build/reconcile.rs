use crate::{
    foundation::{Arc, Container, ContainerOf, HktContainer, Inlinable64Vec},
    scheduler::{get_current_scheduler, JobId, LanePos},
    sync::{LaneScheduler, RenderObjectCommitResult},
    tree::{
        apply_hook_updates_sync, no_widget_update, ArcChildElementNode, Element, ElementNode,
        FullElement, HooksWithCleanups, ImplElementNode, MainlineState,
    },
};

use super::{
    provider::{read_and_update_subscriptions_sync, update_provided_value},
    AsyncCancel, CommitResult, ImplCommitRenderObject,
};

impl<E: FullElement> ElementNode<E> {
    pub(super) fn reconcile_node_sync<'batch>(
        self: &Arc<Self>,
        widget: Option<E::ArcWidget>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> CommitResult<E::ParentProtocol> {
        let setup_result = self.setup_reconcile(widget, lane_scheduler);
        use SetupReconcileResult::*;
        let change = match setup_result {
            SkipAndVisitChildren {
                children,
                render_object,
                self_rebuild_suspended,
            } => {
                let render_object_changes =
                    children.par_map_collect(&get_current_scheduler().sync_threadpool, |child| {
                        let (_child, commit_result) =
                            child.visit_and_work_sync(job_ids, scope, lane_scheduler);
                        commit_result.render_object
                    });

                let render_object_commit_result = <E as Element>::Impl::visit_commit_render_object(
                    &self,
                    render_object,
                    render_object_changes,
                    lane_scheduler,
                    scope,
                    self_rebuild_suspended,
                );
                CommitResult::new(render_object_commit_result)
            }
            Reconcile(reconcile) => {
                self.execute_reconcile(reconcile, job_ids, scope, lane_scheduler)
            }
            SkipAndReturn => CommitResult::new(RenderObjectCommitResult::new_no_update()),
        };

        self.context.purge_lane(LanePos::SYNC);
        return change;
    }
}

struct SyncReconcile<E: Element> {
    has_poll: bool,
    has_mailbox_update: bool,
    old_widget: E::ArcWidget,
    new_widget: Option<E::ArcWidget>,
    state: MainlineState<E, HooksWithCleanups>,
    async_cancel:
        Option<AsyncCancel<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>>>,
}

enum SetupReconcileResult<E: Element> {
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
    fn setup_reconcile(
        self: &Arc<Self>,
        widget: Option<E::ArcWidget>,
        lane_scheduler: &LaneScheduler,
    ) -> SetupReconcileResult<E> {
        // An oppurtunistic probe to allow bypass lock.
        let no_new_widget = widget.is_none();
        let no_mailbox_update = !self.context.mailbox_lanes().contains(LanePos::SYNC);
        let no_consumer_root = !self.context.consumer_lanes().contains(LanePos::SYNC);
        let no_poll = !self.context.needs_poll();
        let no_descendant_lanes = !self.context.descendant_lanes().contains(LanePos::SYNC);

        if no_new_widget && no_mailbox_update && no_consumer_root && no_poll && no_descendant_lanes
        {
            // Subtree has no work, end of visit
            return SetupReconcileResult::SkipAndReturn;
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
                return SetupReconcileResult::SkipAndReturn;
            }
            use MainlineState::*;
            return match state {
                Ready {
                    children,
                    render_object,
                    ..
                } => SetupReconcileResult::SkipAndVisitChildren::<E> {
                    children: E::ChildContainer::clone_container(children),
                    render_object: render_object.clone(),
                    self_rebuild_suspended: false,
                },
                RebuildSuspended { children, .. } => SetupReconcileResult::SkipAndVisitChildren {
                    children: E::ChildContainer::clone_container(children),
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
                    SetupReconcileResult::SkipAndReturn
                }
            };
        }

        let state = (&mut mainline.state).take().expect("Impossible to fail"); // rust-analyzer#14933
                                                                               // Not able to use `Option::map` due to closure lifetime problem.
        let async_cancel = if let Some(entry) = mainline.async_queue.current_ref() {
            let cancel = Self::setup_interrupt_async_work(
                mainline,
                entry.work_context.lane_pos,
                lane_scheduler,
                &self.context,
            )
            .ok()
            .expect("Impossible to fail");
            Some(cancel)
        } else {
            None
        };

        let old_widget = if let Some(widget) = &widget {
            std::mem::replace(&mut snapshot_reborrow.widget, widget.clone())
        } else {
            snapshot_reborrow.widget.clone()
        };
        return SetupReconcileResult::Reconcile(SyncReconcile {
            has_poll: !no_poll,
            has_mailbox_update: !no_mailbox_update,
            old_widget,
            new_widget: widget,
            state,
            async_cancel,
        });
    }

    #[inline(always)]
    fn execute_reconcile<'batch>(
        self: &Arc<Self>,
        reconcile: SyncReconcile<E>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> CommitResult<E::ParentProtocol> {
        let SyncReconcile {
            has_poll,
            has_mailbox_update,
            old_widget,
            new_widget,
            state,
            async_cancel,
        } = reconcile;

        if let Some(async_cancel) = async_cancel {
            self.execute_cancel_async_work(async_cancel, false)
        }
        let new_widget_ref = new_widget.as_ref().unwrap_or(&old_widget);
        let consumed_values = read_and_update_subscriptions_sync(
            E::get_consumed_types(new_widget_ref).as_ref(),
            E::get_consumed_types(&old_widget).as_ref(),
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
                assert!(!has_poll, "A non-suspended node should not be polled");
                if has_mailbox_update {
                    apply_hook_updates_sync(&self.context, job_ids, &mut hooks);
                }
                self.perform_rebuild_node_sync(
                    new_widget,
                    element,
                    children,
                    hooks,
                    Some(render_object),
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
                waker.abort();
                if has_mailbox_update {
                    apply_hook_updates_sync(&self.context, job_ids, &mut suspended_hooks);
                }
                self.perform_rebuild_node_sync(
                    new_widget,
                    element,
                    children,
                    suspended_hooks,
                    None,
                    consumed_values,
                    job_ids,
                    scope,
                    lane_scheduler,
                    is_new_widget,
                )
            }
            MainlineState::InflateSuspended {
                mut suspended_hooks,
                waker,
            } => {
                waker.abort();
                if has_mailbox_update {
                    apply_hook_updates_sync(&self.context, job_ids, &mut suspended_hooks);
                    // TODO: This is impossible and should trigger a warning
                }
                self.perform_inflate_node_sync::<false>(
                    new_widget,
                    Some(suspended_hooks),
                    consumed_values,
                    lane_scheduler,
                )
            }
        }
    }
}
