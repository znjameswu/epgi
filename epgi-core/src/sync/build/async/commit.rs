use crate::{
    foundation::{Arc, Asc, Container, ContainerOf, Protocol},
    scheduler::{get_current_scheduler, LaneMask, LanePos},
    sync::{
        build::provider::AsyncWorkNeedsRestarting, ImplCommitRenderObject, LaneScheduler,
        RenderObjectCommitResult,
    },
    tree::{
        purge_mailbox_updates_async, ArcChildElementNode, AsyncInflating, AsyncOutput,
        AsyncWorkQueue, BuildResults, BuildSuspendResults, ChildRenderObjectsUpdateCallback,
        Element, ElementContextNode, ElementNode, ElementSnapshotInner, FullElement,
        HookContextMode, HooksWithCleanups, ImplElementNode, ImplProvide, Mainline, MainlineState,
        SubscriptionDiff, WorkContext,
    },
};

pub trait AnyElementAsyncCommitExt {
    fn visit_and_commit_async_any<'batch>(
        self: Arc<Self>,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    );
}

impl<E: FullElement> AnyElementAsyncCommitExt for ElementNode<E> {
    fn visit_and_commit_async_any<'batch>(
        self: Arc<Self>,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) {
        self.visit_and_commit_async_impl(finished_lanes, scope, lane_scheduler);
    }
}

pub trait ChildElementAsyncCommitExt<PP: Protocol> {
    fn visit_and_commit_async<'batch>(
        self: Arc<Self>,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> RenderObjectCommitResult<PP>;
}

impl<E: FullElement> ChildElementAsyncCommitExt<E::ParentProtocol> for ElementNode<E> {
    fn visit_and_commit_async<'batch>(
        self: Arc<Self>,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> RenderObjectCommitResult<E::ParentProtocol> {
        self.visit_and_commit_async_impl(finished_lanes, scope, lane_scheduler)
    }
}

impl<E> ElementNode<E>
where
    E: FullElement,
{
    fn visit_and_commit_async_impl<'batch>(
        self: &Arc<Self>,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> RenderObjectCommitResult<E::ParentProtocol> {
        let setup_result = self.setup_visit_and_commit_async(&self.context, finished_lanes);
        use SetupCommitAsyncResult::*;
        let result = match setup_result {
            VisitChildren {
                children,
                render_object,
                self_rebuild_suspended,
            } => {
                let render_object_changes = children
                    .par_map_collect(&get_current_scheduler().sync_threadpool, |child| {
                        child.visit_and_commit_async(finished_lanes, scope, lane_scheduler)
                    });
                <E as Element>::Impl::visit_commit_render_object(
                    &self,
                    render_object,
                    render_object_changes,
                    lane_scheduler,
                    scope,
                    self_rebuild_suspended,
                )
            }
            Commit {
                build_results,
                subscription_diff,
                write_provider,
                work_context,
                rebuild,
            } => self.execute_commit_async(
                build_results,
                subscription_diff,
                write_provider,
                &work_context,
                rebuild,
                finished_lanes,
                scope,
                lane_scheduler,
            ),
            SkipAndReturn => RenderObjectCommitResult::new_no_update(),
        };
        self.context.purge_lanes(finished_lanes);
        debug_assert!(
            self.context.provider_object.is_none()
                || self.context.provider_object.as_ref().is_some_and(
                    |provider| !provider.contains_reservation_from_lanes(finished_lanes)
                ),
            "The commit left residues inside this provider object"
        );
        return result;
    }
}

enum SetupCommitAsyncResult<E: Element> {
    VisitChildren {
        children: ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
        render_object: <<E as Element>::Impl as ImplElementNode<E>>::OptionArcRenderObject,
        // Optimization parameter carried over from sync commit (Because the infrasturcture is build around sync commit)
        self_rebuild_suspended: bool,
    },
    Commit {
        build_results: Result<BuildResults<E>, BuildSuspendResults>,
        subscription_diff: SubscriptionDiff,
        write_provider: bool,
        work_context: Asc<WorkContext>,
        rebuild: Option<(MainlineState<E, HooksWithCleanups>, bool)>,
    },
    SkipAndReturn,
}

impl<E> ElementNode<E>
where
    E: FullElement,
{
    fn setup_visit_and_commit_async<'a>(
        self: &Arc<Self>,
        element_context: &ElementContextNode,
        finished_lanes: LaneMask,
    ) -> SetupCommitAsyncResult<E> {
        use SetupCommitAsyncResult::*;
        let mut snapshot = self.snapshot.lock();
        // https://bevy-cheatbook.github.io/pitfalls/split-borrows.html
        let snapshot_reborrow = &mut *snapshot;

        match &mut snapshot_reborrow.inner {
            ElementSnapshotInner::AsyncInflating(AsyncInflating {
                work_context,
                stash,
            }) => {
                // let lane_pos = work_context.lane_pos;
                let subscription_diff = std::mem::take(&mut stash.subscription_diff);
                let work_context = work_context.clone();
                let write_provider = stash.spawned_consumers.is_some();
                assert!(
                    finished_lanes.contains(work_context.lane_pos),
                    "Async commit should not visit into non-mainline nodes from other lanes"
                );
                match std::mem::replace(&mut stash.output, AsyncOutput::Gone) {
                    AsyncOutput::Completed(build_results) => {
                        debug_assert!(
                            build_results.rebuild_state.is_none(),
                            "Async inflate node should not have a rebuild results"
                        );
                        return Commit {
                            build_results: Ok(build_results),
                            subscription_diff,
                            work_context,
                            write_provider,
                            rebuild: None,
                        };
                    }
                    AsyncOutput::Suspended {
                        mut suspended_results,
                        barrier: None,
                    } => {
                        let suspended_results = (&mut suspended_results).take().expect("Async build should fill back the results before commit ever took place");
                        return Commit {
                            build_results: Err(suspended_results),
                            subscription_diff,
                            work_context,
                            write_provider,
                            rebuild: None,
                        };
                    }
                    AsyncOutput::Uninitiated { barrier: _barrier }
                    | AsyncOutput::Suspended {
                        barrier: Some(_barrier),
                        ..
                    } => panic!("Async commit initiated when there is still commit barrier alive"),
                    AsyncOutput::Gone => panic!("Async commit encountered serious bug"),
                };
            }
            ElementSnapshotInner::Mainline(mainline) => {
                debug_assert!(
                    !mainline
                        .async_queue
                        .backqueue_ref()
                        .is_some_and(|backqueue| backqueue
                            .iter()
                            .any(|entry| finished_lanes.contains(entry.work_context.lane_pos))),
                    "Finished lanes should not show up in backqueue during commit!"
                );
                // We do not occupy this node
                // Because until we release this node, no other async work will start executing
                // Those trying to occupy this node will fail and request the scheduler (which we now hold) to reorder work.
                // After we are finished with the commit, the scheduler will proceed to reorder work.
                let current = mainline.async_queue.remove_current_if(|current| {
                    finished_lanes.contains(current.work_context.lane_pos)
                });

                if let Some(current) = current {
                    let state = (&mut mainline.state).take().expect(
                        "Async commit walk should not witness a node occupied by another sync walk",
                    );
                    let subscription_diff = current.stash.subscription_diff;
                    let write_provider = current.stash.spawned_consumers.is_some();
                    let mut is_new_widget = false;

                    // We update the widget right now right here
                    // Because the sync version also updated the widget right during the setup phase, before provider read and everything.
                    // We imitate the effect order from the sync phase
                    if let Some(widget) = current.widget {
                        is_new_widget = true;
                        snapshot_reborrow.widget = widget;
                    }

                    let build_results = match current.stash.output {
                        AsyncOutput::Completed(build_results) => Ok(build_results),
                        AsyncOutput::Suspended {
                            suspended_results: Some(suspended_results),
                            barrier: None,
                        } => Err(suspended_results),
                        AsyncOutput::Uninitiated { .. }
                        | AsyncOutput::Suspended {
                            barrier: Some(_), ..
                        } => panic!("CommitBarrier should not be encountered during commit"),
                        AsyncOutput::Gone
                        | AsyncOutput::Suspended {
                            suspended_results: None,
                            ..
                        } => {
                            panic!("Async results are gone before commit")
                        }
                    };
                    return Commit {
                        build_results,
                        subscription_diff,
                        write_provider,
                        work_context: current.work_context.clone(),
                        rebuild: Some((state, is_new_widget)),
                    };
                } else {
                    let state = mainline.state.as_ref().expect(
                        "Async commit walk should not witness a node occupied by another sync walk",
                    );
                    // No work in this node, check descendant
                    let no_descendant_lanes =
                        !element_context.descendant_lanes().overlaps(finished_lanes);
                    if no_descendant_lanes {
                        return SkipAndReturn;
                    }
                    // Skip and visit children
                    use MainlineState::*;
                    let (children, render_object, self_rebuild_suspended) = match state {
                        Ready {
                            children,
                            render_object,
                            ..
                        } => (children, render_object.clone(), false),
                        RebuildSuspended { children, .. } => (children, Default::default(), true),
                        InflateSuspended { .. } => panic!(
                            "Async commit walk should not walk into a \
                        inflate suspended node that it has no work on. \
                        Inflate suspended node has no children \
                        and therefore impossible to have work in its descendants"
                        ),
                    };
                    return VisitChildren {
                        children: children.map_ref_collect(Clone::clone),
                        render_object,
                        self_rebuild_suspended,
                    };
                }
            }
        }
    }

    fn execute_commit_async<'a, 'batch>(
        self: &Arc<Self>,
        build_results: Result<BuildResults<E>, BuildSuspendResults>,
        subscription_diff: SubscriptionDiff,
        write_provider: bool,
        work_context: &'a WorkContext,
        rebuild: Option<(MainlineState<E, HooksWithCleanups>, bool)>,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> RenderObjectCommitResult<E::ParentProtocol> {
        let lane_pos = work_context.lane_pos;
        // Fire common pre-visit effects
        // These effects are fired top-down in sync build. Therefore we must fire them before visit into children
        // Note: update widget is already done in the setup phase
        self.commit_async_read(subscription_diff, lane_pos, lane_scheduler);
        if <E as Element>::Impl::PROVIDE_ELEMENT {
            if write_provider {
                let provider =
                    self.context.provider_object.as_ref().expect(
                        "Provider element should have a provider in its element context node",
                    );
                provider.commit_async_write(lane_pos, work_context.batch.id, lane_scheduler);
            }
        } else {
            debug_assert!(!write_provider);
        }

        match build_results {
            Ok(build_results) => self.execute_commit_succeed_async(
                build_results,
                &work_context,
                rebuild,
                finished_lanes,
                scope,
                lane_scheduler,
            ),
            Err(suspended_results) => {
                // Note that how commit suspended does need to visit any children
                // Because for inflate suspended, there is no children to visit in the first place.
                // For rebuild suspended, in our current design, rebuild is not allowed to commit as suspended in async batches.
                // Instead, rebuild is always commit as completed. So, we do not need to consider the mainline children of rebuild suspended node.
                //
                // Note: Because we do not visit children, this path COULD have reused the element lock from the setup phase.
                // We wasted this oppurtunity by release the lock earlier and retake the lock again.
                // But the code style will be horrendous if we push for zero-cost abstraction
                // Failed attempt:
                // - Pass lock into setup and execute phase. Fail reason:
                //      - Technically no fail.
                //      - In that design we should also return work_context by reference, and that reference in the non-suspend branch severely interfered with lock's lifecycle in this branch (makes separation of concern a nightmare). Means it is still not the best solution.
                //      - In that design we have to hold the lock while commit_async_read, which is strange (though could be sound) and more restrictive compared to the sync version. And semantics of the sync version is why this code evolved into this state in the first place.
                self.execute_commit_suspended_async(suspended_results);
                RenderObjectCommitResult::Suspend
            }
        }
    }

    fn execute_commit_succeed_async<'batch>(
        self: &Arc<Self>,
        build_results: BuildResults<E>,
        work_context: &WorkContext,
        rebuild: Option<(MainlineState<E, HooksWithCleanups>, bool)>,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> RenderObjectCommitResult<E::ParentProtocol> {
        struct CommitPreVisitStash<E: Element> {
            element: E,
            hooks: HooksWithCleanups,
            children: ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
            rebuild_stash: Option<CommitPreVisitRebuildStash<E>>,
        }

        struct CommitPreVisitRebuildStash<E: Element> {
            is_new_widget: bool,
            render_object: Option<<E::Impl as ImplElementNode<E>>::OptionArcRenderObject>,
            shuffle: Option<ChildRenderObjectsUpdateCallback<E::ChildContainer, E::ChildProtocol>>,
        }
        let BuildResults {
            hooks: new_hooks,
            element,
            children,
            rebuild_state,
        } = build_results;

        // Fire pre-visit effects
        // These effects are fired top-down in sync build. Therefore we must fire them before visit into children
        // Note: update widget is already done in the setup phase
        let stash = if let Some((state, is_new_widget)) = rebuild {
            purge_mailbox_updates_async(&self.context, work_context.job_ids());
            use MainlineState::*;
            match state {
                Ready { .. } | RebuildSuspended { .. } => {
                    let rebuild_state =
                        rebuild_state.expect("Rebuild commit should see rebuild results");
                    let (mut hooks, render_object) = match state {
                        Ready {
                            hooks,
                            render_object,
                            ..
                        } => (hooks, Some(render_object)),
                        RebuildSuspended {
                            suspended_hooks,
                            waker,
                            ..
                        } => {
                            waker.abort();
                            (suspended_hooks, None)
                        }
                        _ => unreachable!(),
                    };
                    hooks.merge_with(new_hooks, false, HookContextMode::Rebuild);
                    for node_needing_unmount in rebuild_state.nodes_needing_unmount {
                        scope.spawn(|scope| node_needing_unmount.unmount(scope, lane_scheduler))
                    }
                    CommitPreVisitStash {
                        element,
                        hooks,
                        children,
                        rebuild_stash: Some(CommitPreVisitRebuildStash {
                            render_object,
                            shuffle: rebuild_state.shuffle,
                            is_new_widget,
                        }),
                    }
                }
                InflateSuspended {
                    mut suspended_hooks,
                    waker,
                } => {
                    debug_assert!(
                        rebuild_state.is_none(),
                        "Inflate should see rebuild results"
                    );
                    waker.abort();
                    suspended_hooks.merge_with(new_hooks, false, HookContextMode::PollInflate);
                    CommitPreVisitStash {
                        element,
                        hooks: suspended_hooks,
                        children,
                        rebuild_stash: None,
                    }
                }
            }
        } else {
            let hooks = new_hooks.fire_effects();
            CommitPreVisitStash {
                element,
                hooks,
                children,
                rebuild_stash: None,
            }
        };

        // Visit children
        let render_object_changes = stash
            .children
            .map_ref_collect(Clone::clone)
            .par_map_collect(&get_current_scheduler().sync_threadpool, |child| {
                child.visit_and_commit_async(finished_lanes, scope, lane_scheduler)
            });

        // Fire post-visit effects
        // These effects are fired bottom-up in the sync build.
        let mut snapshot = self.snapshot.lock();
        let snapshot_reborrow = &mut *snapshot;

        let CommitPreVisitStash {
            element,
            hooks,
            mut children,
            rebuild_stash,
        } = stash;
        let (state, change) = if let Some(rebuild_stash) = rebuild_stash {
            let CommitPreVisitRebuildStash {
                is_new_widget,
                render_object,
                shuffle,
            } = rebuild_stash;
            let (render_object, change) =
                <E as Element>::Impl::rebuild_success_commit_render_object(
                    &element,
                    &snapshot_reborrow.widget,
                    shuffle,
                    &mut children,
                    render_object,
                    render_object_changes,
                    &self.context,
                    lane_scheduler,
                    scope,
                    is_new_widget,
                );
            (
                MainlineState::Ready {
                    element,
                    hooks,
                    children,
                    render_object,
                },
                change,
            )
        } else {
            let (render_object, change) =
                <E as Element>::Impl::inflate_success_commit_render_object(
                    &element,
                    &snapshot_reborrow.widget,
                    &mut children,
                    render_object_changes,
                    &self.context,
                    lane_scheduler,
                );
            (
                MainlineState::Ready {
                    element,
                    hooks,
                    children,
                    render_object,
                },
                change,
            )
        };
        match &mut snapshot_reborrow.inner {
            ElementSnapshotInner::AsyncInflating(_) => {
                snapshot_reborrow.inner = ElementSnapshotInner::Mainline(Mainline {
                    state: Some(state),
                    async_queue: AsyncWorkQueue::new_empty(),
                });
            }
            ElementSnapshotInner::Mainline(mainline) => {
                mainline.state = Some(state);
            }
        }
        change
    }

    fn execute_commit_suspended_async<'batch>(&self, suspended_results: BuildSuspendResults) {
        suspended_results.waker.make_sync();

        let mut snapshot = self.snapshot.lock();
        match &mut snapshot.inner {
            ElementSnapshotInner::AsyncInflating(_async_inflating) => {
                snapshot.inner = ElementSnapshotInner::Mainline(Mainline {
                    state: Some(MainlineState::InflateSuspended {
                        suspended_hooks: suspended_results.hooks.fire_effects(),
                        waker: suspended_results.waker,
                    }),
                    async_queue: AsyncWorkQueue::new_empty(),
                });
            }
            ElementSnapshotInner::Mainline(mainline) => {
                let state = (&mut mainline.state).take().expect(
                    "Async commit walk should not witness a node occupied by another sync walk",
                );
                use MainlineState::*;
                let new_state = match state {
                    Ready {
                        element,
                        mut hooks,
                        children,
                        render_object,
                    } => {
                        hooks.merge_with(suspended_results.hooks, true, HookContextMode::Rebuild);
                        <<E as Element>::Impl as ImplCommitRenderObject<E>>::rebuild_suspend_commit_render_object(
                            Some(render_object),
                        );
                        RebuildSuspended {
                            element,
                            suspended_hooks: hooks,
                            children,
                            waker: suspended_results.waker,
                        }
                    }
                    InflateSuspended {
                        mut suspended_hooks,
                        waker,
                    } => {
                        waker.abort();
                        suspended_hooks.merge_with(
                            suspended_results.hooks,
                            true,
                            HookContextMode::PollInflate,
                        );
                        InflateSuspended {
                            suspended_hooks,
                            waker: suspended_results.waker,
                        }
                    }
                    RebuildSuspended {
                        element,
                        mut suspended_hooks,
                        children,
                        waker,
                    } => {
                        waker.abort();
                        suspended_hooks.merge_with(
                            suspended_results.hooks,
                            true,
                            HookContextMode::Rebuild,
                        );
                        RebuildSuspended {
                            element,
                            suspended_hooks,
                            children,
                            waker: suspended_results.waker,
                        }
                    }
                };
                mainline.state = Some(new_state);
            }
        };
    }

    fn commit_async_read(
        self: &Arc<Self>,
        subscription_diff: SubscriptionDiff,
        lane_pos: LanePos,
        lane_scheduler: &LaneScheduler,
    ) {
        let SubscriptionDiff {
            register,
            reserve,
            remove,
        } = subscription_diff;

        if register.is_empty() && reserve.is_empty() && remove.is_empty() {
            return;
        }

        let mut async_work_needs_restarting = AsyncWorkNeedsRestarting::new();

        let weak_element_context = Arc::downgrade(&self.context);

        for providing_element_context in reserve {
            let contending_writer = providing_element_context.register_reserved_read(
                weak_element_context.clone(),
                &(Arc::downgrade(self) as _),
                lane_pos,
            );
            if let Some(contending_lane) = contending_writer {
                async_work_needs_restarting.push(contending_lane, providing_element_context)
            }
        }

        for providing_element_context in register {
            let provider = providing_element_context
                .provider_object
                .as_ref()
                .expect("Recorded providers should exist");
            let contending_writer = provider.register_read(weak_element_context.clone());
            if let Some(contending_lane) = contending_writer {
                async_work_needs_restarting.push(contending_lane, providing_element_context)
            }
        }

        for providing_element_context in remove {
            let provider = providing_element_context
                .provider_object
                .as_ref()
                .expect("Recorded providers should exist");
            let contending_writer = provider.unregister_read(&weak_element_context);
            if let Some(contending_lane) = contending_writer {
                async_work_needs_restarting.push(contending_lane, providing_element_context)
            }
        }

        async_work_needs_restarting.execute_restarts(lane_scheduler);
    }
}
