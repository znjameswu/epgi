pub(crate) mod component_element;
pub(crate) mod render_element;
// pub(crate) mod suspense_element;

use linear_map::LinearMap;

use crate::{
    foundation::{
        Arc, AsIterator, Container, Inlinable64Vec, InlinableDwsizeVec, LinearMapEntryExt, Provide,
        SyncMutex, TypeKey, EMPTY_CONSUMED_TYPES,
    },
    scheduler::{get_current_scheduler, JobId, LanePos},
    sync::{BuildScheduler, SubtreeRenderObjectChange},
    tree::{
        no_widget_update, ArcChildElementNode, ArcElementContextNode, AsyncWorkQueue, BuildContext,
        ChildRenderObjectsUpdateCallback, ContainerOf, Element, ElementContextNode, ElementNode,
        ElementReconcileItem, ElementSnapshot, ElementSnapshotInner, HookContext, Hooks, Mainline,
        MainlineState, SelectArcRenderObject, SelectProvideElement,
    },
};

use super::CancelAsync;

pub trait ImplElementNodeSyncReconcile<E: Element<ElementNode = Self>>: Sized {
    fn rebuild_node_sync(
        self: &Arc<Self>,
        widget: Option<E::ArcWidget>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        build_scheduler: &BuildScheduler,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol>;

    fn inflate_node_sync(
        widget: &E::ArcWidget,
        parent_context: ArcElementContextNode,
        build_scheduler: &BuildScheduler,
    ) -> (
        Arc<E::ElementNode>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    );
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> ImplElementNodeSyncReconcile<E>
    for ElementNode<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: Element<ElementNode = Self>
        + SelectReconcileImpl<RENDER_ELEMENT, PROVIDE_ELEMENT>
        + SelectProvideElement<PROVIDE_ELEMENT>,
{
    fn rebuild_node_sync(
        self: &Arc<Self>,
        widget: Option<E::ArcWidget>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        build_scheduler: &BuildScheduler,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        let visit_action = self.visit_inspect(widget, build_scheduler);
        self.execute_visit(visit_action, job_ids, scope, build_scheduler)
    }

    fn inflate_node_sync(
        widget: &E::ArcWidget,
        parent_context: ArcElementContextNode,
        build_scheduler: &BuildScheduler,
    ) -> (
        Arc<E::ElementNode>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        let node = Arc::new_cyclic(|weak| ElementNode {
            context: Arc::new(ElementContextNode::new_for::<E, PROVIDE_ELEMENT>(
                weak.clone() as _,
                parent_context,
                widget,
            )),
            snapshot: SyncMutex::new(ElementSnapshot {
                widget: widget.clone(),
                inner: ElementSnapshotInner::Mainline(Mainline {
                    state: None,
                    async_queue: AsyncWorkQueue::new_empty(),
                }),
            }),
        });

        let consumed_values = Self::read_and_update_subscriptions_sync(
            E::get_consumed_types(widget),
            EMPTY_CONSUMED_TYPES,
            &node.context,
            build_scheduler,
        );

        let subtree_results = Self::perform_inflate_node_sync::<true>(
            &node,
            widget,
            HookContext::new_inflate(),
            consumed_values,
            build_scheduler,
        );
        return (node, subtree_results);
    }
}

enum VisitAction<E: Element, R> {
    Rebuild {
        is_poll: bool,
        old_widget: E::ArcWidget,
        new_widget: Option<E::ArcWidget>,
        state: MainlineState<E, R>,
        cancel_async: Option<CancelAsync<ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>>>,
    },
    /// Visit is needed when the node itself does not need reconcile, but
    /// lane marking has indicated that one of its descendants needs needs reconcile.
    ///
    /// The visit variant will under no circumstance change the mainline state.
    /// Therefore, this variant won't occupy the element node. As a result, exisiting async work won't be interrupted
    /// However, the visit variant WILL have other commit effects, such as createing/updating/detaching render object.
    Visit {
        children: ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
        // This has two variant in case the render object is detached
        // We do not store MaybeSuspendeChildRenderObject, because everytime we need to access it, (update children suspend state)
        // we have to write it into the node anyway. Could just lock the mutex
        // We also do not store the widget, because everytime we need to access it, (create render object)
        // we have to write the render object into the node anyway
        render_object: R,
        // render_object: Option<
        //     // This field is needed in case a new descendant render object pops up.
        //     ArcRenderObjectOf<E>,
        // >,
        self_rebuild_suspended: bool,
    },
    /// End-of-visit is triggered when both the node and its descendants (i.e. entire subtree) does not need reconcile
    EndOfVisit,
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool>
    ElementNode<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: Element<ElementNode = Self>
        + SelectArcRenderObject<RENDER_ELEMENT>
        + SelectReconcileImpl<RENDER_ELEMENT, PROVIDE_ELEMENT>
        + SelectProvideElement<PROVIDE_ELEMENT>,
{
    fn visit_inspect(
        self: &Arc<Self>,
        widget: Option<E::ArcWidget>,
        build_scheduler: &BuildScheduler,
    ) -> VisitAction<E, E::OptionArcRenderObject> {
        let no_new_widget = widget.is_none();
        let no_mailbox_update = !self.context.mailbox_lanes().contains(LanePos::Sync);
        let no_consumer_root = !self.context.consumer_root_lanes().contains(LanePos::Sync);
        let no_poll = !self.context.needs_poll();
        let no_descendant_lanes = !self.context.descendant_lanes().contains(LanePos::Sync);

        // Subtree has no work, end of visit
        if no_new_widget && no_mailbox_update && no_consumer_root && no_poll && no_descendant_lanes
        {
            return VisitAction::EndOfVisit;
        }

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

        let no_widget_update = no_widget_update::<E>(widget.as_ref(), old_widget);
        // Self has no work, but subtree has work. Visit
        if no_widget_update && no_mailbox_update && no_poll {
            use MainlineState::*;
            match state {
                Ready {
                    children,
                    render_object,
                    ..
                } => VisitAction::Visit::<E, E::OptionArcRenderObject> {
                    children: children.map_ref_collect(Clone::clone),
                    render_object: render_object.clone(),
                    self_rebuild_suspended: false,
                },
                RebuildSuspended { children, .. } => VisitAction::Visit {
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
                    VisitAction::EndOfVisit
                }
            };
        }

        let state = (&mut mainline.state).take().expect("Impossible to fail"); // rust-analyzer#14933
                                                                               // Not able to use `Option::map` due to closure lifetime problem.
        let cancel_async = if let Some(entry) = mainline.async_queue.current() {
            let cancel = Self::prepare_cancel_async_work(
                mainline,
                entry.work.context.lane_pos,
                build_scheduler,
            )
            .ok()
            .expect("Impossible to fail");
            Some(cancel)
        } else {
            None
        };

        // Cannot skip work but can skip rebuild, meaning there is a polling work here.
        if no_widget_update && no_mailbox_update {
            return VisitAction::Rebuild {
                is_poll: true,
                old_widget: old_widget.clone(),
                new_widget: widget,
                state,
                cancel_async,
            };
        }
        let old_widget = if let Some(widget) = &widget {
            std::mem::replace(old_widget, widget.clone())
        } else {
            old_widget.clone()
        };
        return VisitAction::Rebuild {
            is_poll: false,
            old_widget,
            new_widget: widget,
            state,
            cancel_async,
        };
    }

    #[inline(always)]
    fn execute_visit(
        self: &Arc<Self>,
        visit_action: VisitAction<E, E::OptionArcRenderObject>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        build_scheduler: &BuildScheduler,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        let result = match visit_action {
            VisitAction::Visit {
                children,
                render_object,
                self_rebuild_suspended,
            } => {
                let results = children
                    .par_map_collect(&get_current_scheduler().sync_threadpool, |child| {
                        child.visit_and_work_sync(job_ids, scope, build_scheduler)
                    });
                let (_children, render_object_changes) = results.unzip_collect(|x| x);

                return E::visit_commit(
                    &self,
                    render_object,
                    render_object_changes,
                    self_rebuild_suspended,
                    scope,
                    build_scheduler,
                );
            }
            VisitAction::Rebuild {
                is_poll,
                old_widget,
                new_widget,
                state,
                cancel_async,
            } => {
                if let Some(cancel_async) = cancel_async {
                    self.perform_cancel_async_work(cancel_async)
                }
                let new_widget_ref = new_widget.as_ref().unwrap_or(&old_widget);
                let consumed_values = Self::read_and_update_subscriptions_sync(
                    E::get_consumed_types(new_widget_ref),
                    E::get_consumed_types(&old_widget),
                    &self.context,
                    build_scheduler,
                );
                if let Some(widget) = new_widget.as_ref() {
                    Self::update_provided_value(&old_widget, widget, &self.context, build_scheduler)
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
                        Self::apply_updates_sync_new(&self.context, job_ids, &mut hooks);
                        self.perform_rebuild_node_sync_new(
                            new_widget,
                            element,
                            children,
                            HookContext::new_rebuild(hooks),
                            render_object,
                            consumed_values,
                            job_ids,
                            scope,
                            build_scheduler,
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
                        // If it is not poll, then it means a new job occurred on this previously suspended node
                        if !is_poll {
                            Self::apply_updates_sync_new(
                                &self.context,
                                job_ids,
                                &mut suspended_hooks,
                            );
                        }
                        self.perform_rebuild_node_sync_new(
                            new_widget,
                            element,
                            children,
                            HookContext::new_rebuild(suspended_hooks),
                            Default::default(),
                            consumed_values,
                            job_ids,
                            scope,
                            build_scheduler,
                            is_new_widget,
                        )
                    }
                    MainlineState::InflateSuspended {
                        suspended_hooks,
                        waker,
                    } => {
                        waker.abort();
                        self.perform_inflate_node_sync::<false>(
                            new_widget,
                            if !is_poll {
                                HookContext::new_inflate()
                            } else {
                                HookContext::new_poll_inflate(suspended_hooks)
                            },
                            consumed_values,
                            build_scheduler,
                        )
                    }
                }
            }
            VisitAction::EndOfVisit => SubtreeRenderObjectChange::new_no_update(),
        };
        self.context.purge_lane(LanePos::Sync);
        return result;
    }

    fn perform_rebuild_node_sync_new(
        self: &Arc<Self>,
        widget: &E::ArcWidget,
        mut element: E,
        children: ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
        mut hook_context: HookContext,
        old_render_object: E::OptionArcRenderObject,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        build_scheduler: &BuildScheduler,
        is_new_widget: bool,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        let mut nodes_needing_unmount = Default::default();
        let results = element.perform_rebuild_element(
            &widget,
            BuildContext {
                hooks: &mut hook_context,
                element_context: &self.context,
            },
            provider_values,
            children,
            &mut nodes_needing_unmount,
        );

        let (state, change) = match results {
            Ok((items, shuffle)) => {
                // Starting the unmounting as early as possible.
                // Unmount before updating render object can cause render object to hold reference to detached children,
                // Therfore, we need to ensure we do not read into render objects before the batch commit is done
                for node_needing_unmount in nodes_needing_unmount {
                    scope.spawn(|scope| node_needing_unmount.unmount(scope))
                }

                let results =
                    items.par_map_collect(&get_current_scheduler().sync_threadpool, |item| {
                        use ElementReconcileItem::*;
                        match item {
                            Keep(node) => node.visit_and_work_sync(job_ids, scope, build_scheduler),
                            Update(pair) => pair.rebuild_sync_box(job_ids, scope, build_scheduler),
                            Inflate(widget) => {
                                widget.inflate_sync(self.context.clone(), build_scheduler)
                            }
                        }
                    });
                let (children, changes) = results.unzip_collect(|x| x);

                let (render_object, change) = E::rebuild_success_commit(
                    &element,
                    widget,
                    shuffle,
                    &children,
                    old_render_object,
                    changes,
                    &self.context,
                    is_new_widget,
                );
                (
                    MainlineState::Ready {
                        element,
                        hooks: hook_context.hooks,
                        children,
                        render_object,
                    },
                    change,
                )
            }
            Err((children, err)) => {
                debug_assert!(
                    nodes_needing_unmount.is_empty(),
                    "An element that suspends itself should not request unmounting any child nodes"
                );

                (
                    MainlineState::RebuildSuspended {
                        suspended_hooks: hook_context.hooks,
                        element,
                        children,
                        waker: err.waker,
                    },
                    E::rebuild_suspend_commit(old_render_object),
                )
            }
        };
        self.commit_write_element(state);
        return change;
    }

    fn perform_inflate_node_sync<const FIRST_INFLATE: bool>(
        self: &Arc<Self>,
        widget: &E::ArcWidget,
        mut hook_context: HookContext,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        build_scheduler: &BuildScheduler,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        let result = E::perform_inflate_element(
            &widget,
            BuildContext {
                hooks: &mut hook_context,
                element_context: &self.context,
            },
            provider_values,
        );

        let (state, change) = match result {
            Ok((element, child_widgets)) => {
                let results = child_widgets
                    .par_map_collect(&get_current_scheduler().sync_threadpool, |child_widget| {
                        child_widget.inflate_sync(self.context.clone(), build_scheduler)
                    });
                let (children, changes) = results.unzip_collect(|x| x);

                debug_assert!(
                    !changes
                        .as_iter()
                        .any(SubtreeRenderObjectChange::is_keep_render_object),
                    "Fatal logic bug in epgi-core reconcile logic. Please file issue report."
                );

                let (render_object, change) =
                    E::inflate_success_commit(&element, widget, &self.context, changes);

                (
                    MainlineState::Ready {
                        element,
                        hooks: hook_context.hooks,
                        children,
                        render_object,
                    },
                    change,
                )
            }
            Err(err) => (
                MainlineState::InflateSuspended {
                    suspended_hooks: hook_context.hooks,
                    waker: err.waker,
                },
                SubtreeRenderObjectChange::Suspend,
            ),
        };
        if FIRST_INFLATE {
            self.commit_write_element_first_inflate(state);
        } else {
            self.commit_write_element(state)
        }
        return change;
    }

    fn apply_updates_sync_new(
        element_context: &ElementContextNode,
        job_ids: &Inlinable64Vec<JobId>,
        hooks: &mut Hooks,
    ) {
        let mut jobs = {
            element_context
                .mailbox
                .lock()
                .extract_if(|job_id, _| job_ids.contains(job_id))
                .collect::<Vec<_>>()
        };
        jobs.sort_by_key(|(job_id, ..)| *job_id);

        let updates = jobs
            .into_iter()
            .flat_map(|(_, updates)| updates)
            .collect::<Vec<_>>();

        for update in updates {
            (update.op)(hooks.array_hooks[update.hook_index].as_mut())
                .ok()
                .expect("We currently do not handle hook failure") //TODO
        }
    }

    fn commit_write_element(self: &Arc<Self>, state: MainlineState<E, E::OptionArcRenderObject>) {
        // Collecting async work is necessary, even if we are inflating!
        // Since it could be an InflateSuspended node and an async batch spawned a secondary root on this node.
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

        if let Some(async_work_needing_start) = async_work_needing_start {
            let node = self.clone();
            node.execute_rebuild_node_async_detached(async_work_needing_start);
        }
    }

    fn commit_write_element_first_inflate(
        self: &Arc<Self>,
        state: MainlineState<E, E::OptionArcRenderObject>,
    ) {
        let mut snapshot = self.snapshot.lock();
        let snapshot_reborrow = &mut *snapshot;
        let mainline = snapshot_reborrow
            .inner
            .mainline_mut()
            .expect("An unmounted element node should not be reachable by a rebuild!");
        debug_assert!(
            mainline.async_queue.is_empty(),
            "The first-time inflate should not see have any other async work"
        );
        mainline.state = Some(state);
    }

    fn update_provided_value(
        old_widget: &E::ArcWidget,
        new_widget: &E::ArcWidget,
        element_context: &ElementContextNode,
        build_scheduler: &BuildScheduler,
    ) {
        if let Some(new_provided_value) = E::diff_provided_value(old_widget, new_widget) {
            let contending_readers = element_context
                .provider
                .as_ref()
                .expect("Element with a provided value should have a provider")
                .write_sync(new_provided_value);

            contending_readers.non_mainline.par_for_each(
                &get_current_scheduler().sync_threadpool,
                |(lane_pos, node)| {
                    let node = node.upgrade().expect("ElementNode should be alive");
                    node.restart_async_work(lane_pos, build_scheduler)
                },
            );

            // This is the a operation, we do not fear any inconsistencies caused by cancellation.
            for reader in contending_readers.mainline {
                reader
                    .upgrade()
                    .expect("Readers should be alive")
                    .mark_consumer_root(LanePos::Sync);
            }
        }
    }

    fn read_and_update_subscriptions_sync(
        new_consumed_types: &[TypeKey],
        old_consumed_types: &[TypeKey],
        element_context: &ArcElementContextNode,
        build_scheduler: &BuildScheduler,
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
                node.restart_async_work(lane_pos, build_scheduler)
            },
        );
        return consumed_values;
    }
}

pub(crate) mod sync_build_private {
    use crate::{foundation::Protocol, tree::ArcAnyElementNode};

    use super::*;

    pub trait AnyElementSyncReconcileExt {
        fn visit_and_work_sync_any(
            self: Arc<Self>,
            job_ids: &Inlinable64Vec<JobId>,
            scope: &rayon::Scope<'_>,
            build_scheduler: &BuildScheduler,
        ) -> ArcAnyElementNode;
    }

    impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> AnyElementSyncReconcileExt
        for ElementNode<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
    where
        E: Element<ElementNode = Self>
            + SelectArcRenderObject<RENDER_ELEMENT>
            + SelectReconcileImpl<RENDER_ELEMENT, PROVIDE_ELEMENT>
            + SelectProvideElement<PROVIDE_ELEMENT>,
    {
        fn visit_and_work_sync_any(
            self: Arc<Self>,
            job_ids: &Inlinable64Vec<JobId>,
            scope: &rayon::Scope<'_>,
            build_scheduler: &BuildScheduler,
        ) -> ArcAnyElementNode {
            self.rebuild_node_sync(None, job_ids, scope, build_scheduler);
            self
        }
    }

    pub trait ChildElementSyncReconcileExt<PP: Protocol> {
        fn visit_and_work_sync(
            self: Arc<Self>,
            job_ids: &Inlinable64Vec<JobId>,
            scope: &rayon::Scope<'_>,
            build_scheduler: &BuildScheduler,
        ) -> (ArcChildElementNode<PP>, SubtreeRenderObjectChange<PP>);
    }

    impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool>
        ChildElementSyncReconcileExt<E::ParentProtocol>
        for ElementNode<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
    where
        E: Element<ElementNode = Self>
            + SelectArcRenderObject<RENDER_ELEMENT>
            + SelectReconcileImpl<RENDER_ELEMENT, PROVIDE_ELEMENT>
            + SelectProvideElement<PROVIDE_ELEMENT>,
    {
        fn visit_and_work_sync(
            self: Arc<Self>,
            job_ids: &Inlinable64Vec<JobId>,
            scope: &rayon::Scope<'_>,
            build_scheduler: &BuildScheduler,
        ) -> (
            ArcChildElementNode<E::ParentProtocol>,
            SubtreeRenderObjectChange<E::ParentProtocol>,
        ) {
            let result = self.rebuild_node_sync(None, job_ids, scope, build_scheduler);
            (self, result)
        }
    }
}

pub trait SelectReconcileImpl<const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool>:
    Element + SelectArcRenderObject<RENDER_ELEMENT>
{
    fn visit_commit(
        element_node: &Self::ElementNode,
        render_object: Self::OptionArcRenderObject,
        render_object_changes: ContainerOf<Self, SubtreeRenderObjectChange<Self::ChildProtocol>>,
        self_rebuild_suspended: bool,
        scope: &rayon::Scope<'_>,
        build_scheduler: &BuildScheduler,
    ) -> SubtreeRenderObjectChange<Self::ParentProtocol>;

    fn rebuild_success_commit(
        element: &Self,
        widget: &Self::ArcWidget,
        shuffle: Option<ChildRenderObjectsUpdateCallback<Self>>,
        children: &ContainerOf<Self, ArcChildElementNode<Self::ChildProtocol>>,
        render_object: Self::OptionArcRenderObject,
        render_object_changes: ContainerOf<Self, SubtreeRenderObjectChange<Self::ChildProtocol>>,
        element_context: &ArcElementContextNode,
        is_new_widget: bool,
    ) -> (
        Self::OptionArcRenderObject,
        SubtreeRenderObjectChange<Self::ParentProtocol>,
    );

    fn rebuild_suspend_commit(
        render_object: Self::OptionArcRenderObject,
    ) -> SubtreeRenderObjectChange<Self::ParentProtocol>;

    fn inflate_success_commit(
        element: &Self,
        widget: &Self::ArcWidget,
        element_context: &ArcElementContextNode,
        render_object_changes: ContainerOf<Self, SubtreeRenderObjectChange<Self::ChildProtocol>>,
    ) -> (
        Self::OptionArcRenderObject,
        SubtreeRenderObjectChange<Self::ParentProtocol>,
    );
}
