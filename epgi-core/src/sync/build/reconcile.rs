use linear_map::LinearMap;

use crate::{
    foundation::{
        Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, LinearMapEntryExt, Parallel, Provide,
        SmallSet, SyncMutex, TypeKey, EMPTY_CONSUMED_TYPES,
    },
    scheduler::{get_current_scheduler, JobId, LanePos},
    sync::{SubtreeCommitResult, TreeScheduler},
    tree::{
        ArcChildElementNode, ArcElementContextNode, ArcRenderObject, AsyncWorkQueue, Element,
        ElementContextNode, ElementNode, ElementSnapshot, ElementSnapshotInner, GetRenderObject,
        HookContext, Hooks, Mainline, MainlineState,
    },
};

use super::{CancelAsync, SyncReconciler};

impl<E> ElementNode<E>
where
    E: Element,
{
    // Reconciler needs this
    pub(in super::super) fn rebuild_node_sync<'a, 'batch>(
        self: &Arc<Self>,
        widget: Option<E::ArcWidget>,
        job_ids: &'a SmallSet<JobId>,
        scope: &'a rayon::Scope<'batch>,
        tree_scheduler: &'batch TreeScheduler,
    ) -> SubtreeCommitResult {
        if !self.context.subtree_lanes().contains(LanePos::Sync) {
            return SubtreeCommitResult::NoUpdate;
        }

        // Skip variant will not occupy the node (i.e., Option::take() from the shared states)
        struct SyncRebuild<E: Element> {
            is_poll: bool,
            old_widget: E::ArcWidget,
            state: MainlineState<E>,
            cancel_async: Option<CancelAsync<E::ChildIter>>,
        }
        let rebuild: Result<SyncRebuild<E>, Option<E::ChildIter>> = {
            let mut snapshot = self.snapshot.lock();
            // https://bevy-cheatbook.github.io/pitfalls/split-borrows.html
            let snapshot_reborrow = &mut *snapshot;
            let old_widget = &mut snapshot_reborrow.widget;

            let mainline = snapshot_reborrow
                .inner
                .mainline_mut()
                .expect("An unmounted element node should not be reachable by a rebuild!");

            let state = mainline.state.as_ref().expect(
                "A sync task should not encounter another sync task contending over the same node",
            );
            if Self::can_skip_work(&widget, old_widget, LanePos::Sync, &self.context) {
                Err(state.last_element_ref().map(Element::children))
            } else {
                let state = (&mut mainline.state).take().expect("Impossible to fail"); // rust-analyzer#14933
                                                                                       // Not able to use `Option::map` due to closure lifetime problem.
                let cancel_async = if let Some(entry) = mainline.async_queue.current() {
                    let cancel = Self::prepare_cancel_async_work(
                        mainline,
                        entry.work.context.lane_pos,
                        tree_scheduler,
                    )
                    .ok()
                    .expect("Impossible to fail");
                    Some(cancel)
                } else {
                    None
                };
                if Self::can_skip_rebuild(&widget, old_widget, LanePos::Sync, &self.context) {
                    // Cannot skip work but can skip rebuild, meaning there is a polling work here.
                    Ok(SyncRebuild {
                        is_poll: true,
                        old_widget: old_widget.clone(),
                        state,
                        cancel_async,
                    })
                } else {
                    let old_widget = if let Some(widget) = &widget {
                        std::mem::replace(old_widget, widget.clone())
                    } else {
                        old_widget.clone()
                    };
                    Ok(SyncRebuild {
                        is_poll: false,
                        old_widget,
                        state,
                        cancel_async,
                    })
                }
            }
        };

        match rebuild {
            Ok(SyncRebuild {
                is_poll,
                old_widget,
                state,
                cancel_async,
            }) => {
                if let Some(cancel_async) = cancel_async {
                    self.perform_cancel_async_work(cancel_async)
                }
                let new_widget_ref = widget.as_ref().unwrap_or(&old_widget);
                let consumed_values = Self::read_and_update_subscriptions_sync(
                    E::get_consumed_types(new_widget_ref),
                    E::get_consumed_types(&old_widget),
                    &self.context,
                    tree_scheduler,
                );
                if let Some(get_provided_value) = E::GET_PROVIDED_VALUE {
                    let old_provided_value = get_provided_value(&old_widget);
                    let new_provided_value = get_provided_value(new_widget_ref);
                    if !Asc::ptr_eq(&old_provided_value, &new_provided_value)
                        && !old_provided_value.eq_sized(new_provided_value.as_ref())
                    {
                        let contending_readers = self
                            .context
                            .provider
                            .as_ref()
                            .expect("Element with a provided value should have a provider")
                            .write_sync(new_provided_value);

                        contending_readers.non_mainline.par_for_each(
                            &get_current_scheduler().sync_threadpool,
                            |(lane_pos, node)| {
                                let node = node.upgrade().expect("ElementNode should be alive");
                                node.restart_async_work(lane_pos, tree_scheduler)
                            },
                        );

                        // This is the a operation, we do not fear any inconsistencies caused by cancellation.
                        for reader in contending_readers.mainline {
                            reader
                                .upgrade()
                                .expect("Readers should be alive")
                                .mark_secondary_root(LanePos::Sync)
                        }
                    }
                }
                let new_widget = &widget.unwrap_or(old_widget);
                match state {
                    MainlineState::Ready {
                        hooks,
                        element: last_element,
                        render_object: attached_object,
                    } => {
                        assert!(!is_poll, "A non-suspended node should not be polled");
                        self.perform_rebuild_node_sync(
                            new_widget,
                            job_ids,
                            hooks,
                            last_element,
                            attached_object,
                            consumed_values,
                            scope,
                            tree_scheduler,
                        )
                    }
                    MainlineState::RebuildSuspended {
                        hooks,
                        last_element,
                        waker,
                    } => {
                        waker.abort();
                        if !is_poll {
                            self.perform_rebuild_node_sync(
                                new_widget,
                                job_ids,
                                hooks,
                                last_element,
                                None,
                                consumed_values,
                                scope,
                                tree_scheduler,
                            )
                        } else {
                            self.perform_poll_rebuild_node_sync(
                                new_widget,
                                job_ids,
                                hooks,
                                last_element,
                                consumed_values,
                                scope,
                                tree_scheduler,
                            )
                        }
                    }
                    MainlineState::InflateSuspended { last_hooks, waker } => {
                        waker.abort();
                        if !is_poll {
                            self.perform_inflate_node_sync(
                                new_widget,
                                job_ids,
                                consumed_values,
                                scope,
                                tree_scheduler,
                            )
                        } else {
                            self.perform_poll_inflate_node_sync(
                                new_widget,
                                job_ids,
                                last_hooks,
                                consumed_values,
                                scope,
                                tree_scheduler,
                            )
                        }
                    }
                }
            }

            Err(Some(children)) => {
                let subtree_commit_results = children
                    .par_map_collect(&get_current_scheduler().sync_threadpool, |child| {
                        child.visit_and_work_sync(job_ids, scope, tree_scheduler)
                    })
                    .into_iter()
                    .reduce(SubtreeCommitResult::merge)
                    .unwrap_or_default();
                // TODO: Absorb new renderobject from subtree by updating the children of this renderobject
                return todo!();
            }

            Err(None) => SubtreeCommitResult::NoUpdate,
        }
    }

    // Reconciler needs this
    pub(in super::super) fn inflate_node_sync<'a, 'batch>(
        widget: &E::ArcWidget,
        parent_context: &ArcElementContextNode,
        job_ids: &'a SmallSet<JobId>,
        scope: &'a rayon::Scope<'batch>,
        tree_scheduler: &'batch TreeScheduler,
    ) -> (Arc<ElementNode<E>>, SubtreeCommitResult) {
        let node = Arc::new_cyclic(|weak| {
            let context = if let Some(get_provided_value) = E::GET_PROVIDED_VALUE {
                ElementContextNode::new_with_provide(
                    weak.clone() as _,
                    parent_context,
                    get_provided_value(&widget),
                )
            } else {
                ElementContextNode::new_no_provide(weak.clone() as _, Some(parent_context))
            };
            ElementNode {
                context: Arc::new(context),
                snapshot: SyncMutex::new(ElementSnapshot {
                    widget: widget.clone(),
                    inner: ElementSnapshotInner::Mainline(Mainline {
                        state: None,
                        async_queue: AsyncWorkQueue::new_empty(),
                    }),
                }),
            }
        });

        // let weak_node: AweakAnyElementNode = Arc::downgrade(&node) as _;
        let consumed_values = Self::read_and_update_subscriptions_sync(
            E::get_consumed_types(widget),
            EMPTY_CONSUMED_TYPES,
            &node.context,
            tree_scheduler,
        );

        let subtree_results = Self::perform_inflate_node_sync(
            &node,
            widget,
            job_ids,
            consumed_values,
            scope,
            tree_scheduler,
        );
        // node.snapshot.lock().inner = snapshot_inner;
        return (node, subtree_results);
    }

    fn perform_rebuild_node_sync<'a, 'batch>(
        self: &'a Arc<Self>,
        widget: &'a E::ArcWidget,
        job_ids: &'a SmallSet<JobId>,
        mut hooks: Hooks,
        element: E,
        old_attached_object: Option<E::ArcRenderObject>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        scope: &'a rayon::Scope<'batch>,
        tree_scheduler: &'batch TreeScheduler,
    ) -> SubtreeCommitResult {
        let mut jobs = {
            self.context
                .mailbox
                .lock()
                .extract_if(|job_id, update| job_ids.contains(job_id))
                .collect::<Vec<_>>()
        };
        jobs.sort_by_key(|(job_id, ..)| *job_id);

        let updates = jobs
            .into_iter()
            .flat_map(|(_, updates)| updates)
            .collect::<Vec<_>>();

        for update in updates {
            todo!()
        }

        let mut hooks_iter = HookContext::new_rebuild(hooks);
        let mut subtree_results = SubtreeCommitResult::NoUpdate;
        let mut nodes_needing_unmount = Default::default();
        let reconciler = SyncReconciler {
            job_ids,
            scope,
            tree_scheduler,
            subtree_results: &mut subtree_results,
            host_context: &self.context,
            hooks: &mut hooks_iter,
            nodes_needing_unmount: &mut nodes_needing_unmount,
        };
        let results = element.perform_rebuild_element(&widget, provider_values, reconciler);
        let (state, subtree_results) = self.process_rebuild_results(
            results,
            hooks_iter,
            &widget,
            job_ids,
            old_attached_object,
            &mut nodes_needing_unmount,
            subtree_results,
            tree_scheduler,
        );
        self.commit_sync(state, nodes_needing_unmount, scope);
        return subtree_results;
    }

    fn perform_inflate_node_sync<'a, 'batch>(
        self: &'a Arc<Self>,
        widget: &E::ArcWidget,
        job_ids: &'a SmallSet<JobId>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        scope: &'a rayon::Scope<'batch>,
        tree_scheduler: &'batch TreeScheduler,
    ) -> SubtreeCommitResult {
        let mut hooks_iter = HookContext::new_inflate();
        let mut subtree_results = SubtreeCommitResult::NoUpdate;
        let mut nodes_needing_unmount = Default::default();
        let reconciler = SyncReconciler {
            job_ids,
            scope,
            tree_scheduler,
            subtree_results: &mut subtree_results,
            host_context: &self.context,
            hooks: &mut hooks_iter,
            nodes_needing_unmount: &mut nodes_needing_unmount,
        };
        let results = E::perform_inflate_element(&widget, provider_values, reconciler);

        let (state, subtree_results) = self.process_inflate_results(
            results,
            hooks_iter,
            &widget,
            job_ids,
            subtree_results,
            tree_scheduler,
        );

        self.commit_sync(state, nodes_needing_unmount, scope);
        return subtree_results;
    }

    fn perform_poll_rebuild_node_sync<'a, 'batch>(
        self: &'a Arc<Self>,
        widget: &E::ArcWidget,
        job_ids: &'a SmallSet<JobId>,
        hooks: Hooks,
        last_element: E,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        scope: &'a rayon::Scope<'batch>,
        tree_scheduler: &'batch TreeScheduler,
    ) -> SubtreeCommitResult {
        let mut hooks_iter = HookContext::new_rebuild(hooks);
        let mut subtree_results = SubtreeCommitResult::NoUpdate;
        let mut nodes_needing_unmount = Default::default();
        let reconciler = SyncReconciler {
            job_ids,
            scope,
            tree_scheduler,
            subtree_results: &mut subtree_results,
            host_context: &self.context,
            hooks: &mut hooks_iter,
            nodes_needing_unmount: &mut nodes_needing_unmount,
        };
        let results = last_element.perform_rebuild_element(&widget, provider_values, reconciler);

        let (state, subtree_results) = self.process_rebuild_results(
            results,
            hooks_iter,
            &widget,
            job_ids,
            None,
            &mut nodes_needing_unmount,
            subtree_results,
            tree_scheduler,
        );
        self.commit_sync(state, nodes_needing_unmount, scope);
        return subtree_results;
    }

    fn perform_poll_inflate_node_sync<'a, 'batch>(
        self: &'a Arc<Self>,
        widget: &E::ArcWidget,
        job_ids: &'a SmallSet<JobId>,
        last_hooks: Hooks,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        scope: &'a rayon::Scope<'batch>,
        tree_scheduler: &'batch TreeScheduler,
    ) -> SubtreeCommitResult {
        let mut hooks_iter = HookContext::new_poll_inflate(last_hooks);
        let mut subtree_results = SubtreeCommitResult::NoUpdate;
        let mut nodes_needing_unmount = Default::default();
        let reconciler = SyncReconciler {
            job_ids,
            scope,
            tree_scheduler,
            subtree_results: &mut subtree_results,
            host_context: &self.context,
            hooks: &mut hooks_iter,
            nodes_needing_unmount: &mut nodes_needing_unmount,
        };
        let results = E::perform_inflate_element(&widget, provider_values, reconciler);

        let (state, subtree_results) = self.process_inflate_results(
            results,
            hooks_iter,
            &widget,
            job_ids,
            subtree_results,
            tree_scheduler,
        );

        self.commit_sync(state, nodes_needing_unmount, scope);
        return subtree_results;
    }

    #[inline(always)]
    fn process_rebuild_results<'a, 'batch>(
        self: &'a Arc<Self>,
        results: Result<E, (E, BuildSuspendedError)>,
        hooks_iter: HookContext,
        widget: &E::ArcWidget,
        job_ids: &'a SmallSet<JobId>,
        mut render_object: Option<E::ArcRenderObject>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<E::ChildProtocol>>,
        subtree_results: SubtreeCommitResult,
        tree_scheduler: &'batch TreeScheduler,
    ) -> (MainlineState<E>, SubtreeCommitResult) {
        match results {
            Ok(mut element) => match E::ArcRenderObject::GET_RENDER_OBJECT {
                GetRenderObject::None(_) => {
                    debug_assert!(
                        render_object.is_none(),
                        "ComponentElement should not have a render object"
                    );
                    (
                        MainlineState::Ready {
                            hooks: hooks_iter.hooks,
                            render_object,
                            element,
                        },
                        subtree_results,
                    )
                }
                GetRenderObject::RenderObject {
                    get_suspense: None,
                    try_create_render_object,
                    update_render_object,
                    try_update_render_object_children,
                    detach_render_object,
                    ..
                } => {
                    let mut suspended = subtree_results == SubtreeCommitResult::Suspended;
                    if let Some(render_object) = render_object.as_ref() {
                        let should_update_render_object =
                            !suspended && subtree_results == SubtreeCommitResult::NewRenderObject;
                        let should_lock = (should_update_render_object
                            && try_update_render_object_children.is_some())
                            || (!suspended && update_render_object.is_some());
                        if should_lock {
                            render_object.with_inner(|render, element_context| {
                                if should_update_render_object {
                                    if let Some(try_update_render_object_children) =
                                        try_update_render_object_children
                                    {
                                        let res =
                                            try_update_render_object_children(render, &element);
                                        suspended |= res.is_err();
                                    }
                                }
                                if !suspended {
                                    if let Some(update_render_object) = update_render_object {
                                        let result = update_render_object(render, widget);
                                        todo!("Mark needs layout")
                                    }
                                }
                            })
                        }
                    } else if !suspended {
                        render_object = try_create_render_object(&element, widget, &self.context);
                        suspended = render_object.is_none();
                    }
                    if suspended {
                        (&mut render_object)
                            .take() // rust-analyzer#14933
                            .map(|render_object| {
                                if let Some(detach_render_object) = detach_render_object {
                                    render_object
                                        .with_inner(|render, _| detach_render_object(render))
                                }
                            });
                    }
                    if !suspended {
                        debug_assert!(render_object.is_some(), "RenderObjectElement that are not suspended should attach a render object")
                    }
                    (
                        MainlineState::Ready {
                            hooks: hooks_iter.hooks,
                            render_object,
                            element,
                        },
                        if suspended {
                            SubtreeCommitResult::NoUpdate
                        } else {
                            SubtreeCommitResult::Suspended
                        },
                    )
                }
                GetRenderObject::RenderObject {
                    get_suspense: Some(get_suspense),
                    try_update_render_object_children,
                    ..
                } => {
                    let suspense = (get_suspense.get_suspense_element_mut)(&mut element);
                    // We only need to handle structural changes (i.e. creation and unmount of fallback element, update render object instances) here.
                    // Since those not changing the structure (i.e. updates inside both the child and the fallback) were already handled by the rebuild.
                    match (suspense.fallback.as_ref(), subtree_results) {
                        (Some(_), SubtreeCommitResult::NewRenderObject) => {
                            // If we have mounted a fallback, then the subtree_results is merged from the subtree_results from both the child and the fallback.
                            // If neither the child nor the fallback was suspended, and at least one of them has a new render object, then we need to check both of them for updates.
                            if let Some(_) = suspense.child.get_current_subtree_render_object() {
                                let fallback_node =
                                    (&mut suspense.fallback).take().expect("Impossible to fail"); // rust-analyzer#14933
                                nodes_needing_unmount.push(fallback_node);
                            }
                        }
                        (None, SubtreeCommitResult::Suspended) => {
                            let (node, subtree_results) = rayon::scope(|scope| {
                                suspense.fallback_widget.clone().inflate_sync(
                                    &self.context,
                                    job_ids,
                                    scope,
                                    tree_scheduler,
                                )
                            });
                            assert_eq!(subtree_results, SubtreeCommitResult::NewRenderObject,
                                    "Fallback widget must not suspend and its subtree must always provide an attached renderobject");
                            suspense.fallback = Some(node);
                        }
                        _ => {}
                    }
                    todo!()
                    // if subtree_results != SubtreeCommitResult::NoUpdate {
                    //     try_update_render_object_children(
                    //         &element,
                    //         render_object
                    //             .as_ref()
                    //             .expect("Suspense itself could never suspend or get detached"),
                    //     )
                    //     .expect("Impossible to fail");
                    // }
                    // (
                    //     MainlineState::Ready {
                    //         hooks: build_context.hooks,
                    //         render_object,
                    //         element,
                    //     },
                    //     SubtreeCommitResult::NoUpdate,
                    // )
                }
            },

            Err((element, err)) => (
                MainlineState::RebuildSuspended {
                    hooks: hooks_iter.hooks,
                    last_element: element,
                    waker: err.waker,
                },
                SubtreeCommitResult::Suspended,
            ),
        }
    }

    #[inline(always)]
    fn process_inflate_results<'a, 'batch>(
        self: &'a Arc<Self>,
        results: Result<E, BuildSuspendedError>,
        hooks_iter: HookContext,
        widget: &E::ArcWidget,
        job_ids: &'a SmallSet<JobId>,
        subtree_results: SubtreeCommitResult,
        tree_scheduler: &'batch TreeScheduler,
    ) -> (MainlineState<E>, SubtreeCommitResult) {
        match results {
            Ok(mut element) => match E::ArcRenderObject::GET_RENDER_OBJECT {
                GetRenderObject::None(_) => (
                    MainlineState::Ready {
                        hooks: hooks_iter.hooks,
                        render_object: None,
                        element,
                    },
                    SubtreeCommitResult::Suspended,
                ),
                GetRenderObject::RenderObject {
                    try_create_render_object,
                    get_suspense: None,
                    ..
                } => {
                    let render_object = (subtree_results != SubtreeCommitResult::Suspended).then(|| {
                        try_create_render_object(&element, widget, &self.context).expect("Unsuspended inflating subtree should succeed in creating render object")
                    });
                    (
                        MainlineState::Ready {
                            hooks: hooks_iter.hooks,
                            render_object,
                            element,
                        },
                        subtree_results.absorb(),
                    )
                }
                GetRenderObject::RenderObject {
                    try_create_render_object,
                    get_suspense: Some(get_suspense),
                    ..
                } => {
                    let suspense = (get_suspense.get_suspense_element_mut)(&mut element);
                    if subtree_results == SubtreeCommitResult::Suspended {
                        let (node, subtree_results) = rayon::scope(|scope| {
                            suspense.fallback_widget.clone().inflate_sync(
                                &self.context,
                                job_ids,
                                scope,
                                tree_scheduler,
                            )
                        });
                        assert_eq!(subtree_results, SubtreeCommitResult::NewRenderObject,
                                "Fallback widget must not suspend and its subtree must always provide an attached renderobject");
                        suspense.fallback = Some(node);
                    }
                    let render_object = try_create_render_object(&element, widget, &self.context);
                    debug_assert!(render_object.is_some(), "Impossible to fail");
                    (
                        MainlineState::Ready {
                            hooks: hooks_iter.hooks,
                            render_object,
                            element,
                        },
                        SubtreeCommitResult::NoUpdate,
                    )
                }
            },
            Err(err) => (
                MainlineState::InflateSuspended {
                    last_hooks: hooks_iter.hooks,
                    waker: err.waker,
                },
                SubtreeCommitResult::Suspended,
            ),
        }
    }

    fn commit_sync<'a, 'batch>(
        self: &'a Arc<Self>,
        state: MainlineState<E>,
        nodes_needing_unmount: InlinableDwsizeVec<ArcChildElementNode<E::ChildProtocol>>,
        scope: &'a rayon::Scope<'batch>,
    ) {
        let suspended = state.is_suspended();

        let async_work_needing_start = {
            let mut snapshot = self.snapshot.lock();
            let snapshot_reborrow = &mut *snapshot;
            let mainline = snapshot_reborrow
                .inner
                .mainline_mut()
                .expect("An unmounted element node should not be reachable by a rebuild!");
            debug_assert!(
                mainline.async_queue.current().is_none(),
                "An async work should not be executing alongside a sync work"
            );
            mainline.state = Some(state);
            self.prepare_execute_backqueue(mainline, &snapshot_reborrow.widget)
        };

        if suspended {
            debug_assert!(
                nodes_needing_unmount.is_empty(),
                "A suspended build should not submit unmount request!"
            )
        }

        for node in nodes_needing_unmount {
            scope.spawn(|_| node.unmount());
        }

        if let Some(async_work_needing_start) = async_work_needing_start {
            let node = self.clone();
            node.execute_rebuild_node_async_detached(async_work_needing_start);
        }
    }

    fn read_and_update_subscriptions_sync(
        new_consumed_types: &[TypeKey],
        old_consumed_types: &[TypeKey],
        element_context: &ArcElementContextNode,
        tree_scheduler: &TreeScheduler,
    ) -> InlinableDwsizeVec<Arc<dyn Provide>> {
        let is_old_consumed_types = std::ptr::eq(new_consumed_types, old_consumed_types);

        // Unregister
        for consumed in old_consumed_types.iter() {
            if !new_consumed_types.contains(consumed) {
                let removed = element_context
                    .provider_map
                    .get(consumed)
                    .expect("ProviderMap should be consistent")
                    .provider
                    .as_ref()
                    .expect("Element should provide types according to ProviderMap")
                    .unregister_read(&Arc::downgrade(element_context));
                debug_assert!(removed)
            }
        }

        // Why do we need to restart contending async writers at all?
        // Because if we are registering a new read, they will be unaware of us as a secondary root.

        // We only need to cancel contending async writers only if this is a new subscription.
        // Because a contending async writer on an old subsciption will naturally find this node as a secondary root.

        // We only need to cancel the topmost contending writes from a single lane. Because all its subtree will be purged.
        let mut async_work_needs_restarting = LinearMap::<LanePos, ArcElementContextNode>::new();

        let consumed_values = new_consumed_types
            .iter()
            .map(|consumed| {
                let is_old = is_old_consumed_types || old_consumed_types.contains(consumed);
                let subscription = element_context
                    .provider_map
                    .get(consumed)
                    .expect("Requested provider should exist");
                let provider = subscription
                    .provider
                    .as_ref()
                    .expect("Element should provide types according to ProviderMap");
                if !is_old {
                    let contending_writer = provider.register_read(Arc::downgrade(element_context));
                    if let Some(contending_lane) = contending_writer {
                        async_work_needs_restarting
                            .entry(contending_lane)
                            .and_modify(|v| {
                                if v.depth < subscription.depth {
                                    *v = subscription.clone()
                                }
                            })
                            .or_insert_with(|| subscription.clone());
                    }
                }
                provider.read()
            })
            .collect();
        let async_work_needs_restarting: Vec<_> = async_work_needs_restarting.into();
        async_work_needs_restarting.par_for_each(
            &get_current_scheduler().sync_threadpool,
            |(lane_pos, context)| {
                let node = context
                    .element_node
                    .upgrade()
                    .expect("ElementNode should be alive");
                node.restart_async_work(lane_pos, tree_scheduler)
            },
        );
        return consumed_values;
    }
}

pub(crate) mod sync_build_private {
    use super::*;

    pub trait AnyElementSyncTreeWalkExt {
        fn visit_and_work_sync<'a, 'batch>(
            self: Arc<Self>,
            job_ids: &'a SmallSet<JobId>,
            scope: &'a rayon::Scope<'batch>,
            tree_scheduler: &'batch TreeScheduler,
        ) -> SubtreeCommitResult;
    }

    impl<E> AnyElementSyncTreeWalkExt for ElementNode<E>
    where
        E: Element,
    {
        fn visit_and_work_sync<'a, 'batch>(
            self: Arc<Self>,
            job_ids: &'a SmallSet<JobId>,
            scope: &'a rayon::Scope<'batch>,
            tree_scheduler: &'batch TreeScheduler,
        ) -> SubtreeCommitResult {
            self.rebuild_node_sync(None, job_ids, scope, tree_scheduler)
        }
    }
}
