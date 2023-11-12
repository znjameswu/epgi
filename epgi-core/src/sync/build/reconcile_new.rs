use std::marker::PhantomData;

use linear_map::LinearMap;

use crate::{
    foundation::{
        access_node, AccessArcRenderObject, AccessNode, Arc, AsIterator, Asc, BuildSuspendedError,
        HktContainer, Inlinable64Vec, InlinableDwsizeVec, LinearMapEntryExt, NodeAccessor,
        Parallel, Provide, SyncMutex, TypeKey, EMPTY_CONSUMED_TYPES,
    },
    scheduler::{get_current_scheduler, JobId, LanePos},
    sync::{SubtreeRenderObjectChange, SubtreeRenderObjectCommitResultSummary, TreeScheduler},
    tree::{
        is_non_render_element, is_non_suspense_render_element, is_suspense_element,
        render_element_function_table_of, ArcChildElementNode, ArcElementContextNode,
        ArcRenderObjectOf, AsyncWorkQueue, BuildContext, ChildRenderObjectsUpdateCallback,
        ContainerOf, Element, ElementContextNode, ElementNode, ElementReconcileItem,
        ElementSnapshot, ElementSnapshotInner, HookContext, Hooks, Mainline, MainlineState,
        MaybeSuspendChildRenderObject, RenderChildrenOf, RenderElementFunctionTable, RenderObject,
        RenderObjectReconcileItem, RenderOrUnit, RerenderAction, SuspenseElementFunctionTable,
    },
};

use super::{CancelAsync, SyncReconcileContext};

enum VisitAction<E: Element> {
    Rebuild {
        is_poll: bool,
        old_widget: E::ArcWidget,
        new_widget: Option<E::ArcWidget>,
        state: MainlineState<E>,
        cancel_async: Option<CancelAsync<ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>>>,
    },
    /// Visit is needed when the node itself does not need reconcile, but
    /// lane marking has indicated that one of its descendants needs needs reconcile.
    ///
    /// The visit variant will under no circumstance change the mainline state.
    /// Therefore, this variant won't occupy the element node. As a result, exisiting async work won't be interrupted
    /// However, the visit variant WILL have other commit effects, such as createing/updating/detaching render object.
    Visit {
        element: E,
        children: ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
        // This has two variant in case the render object is detached
        // We do not store MaybeSuspendeChildRenderObject, because everytime we need to access it, (update children suspend state)
        // we have to write it into the node anyway. Could just lock the mutex
        // We also do not store the widget, because everytime we need to access it, (create render object)
        // we have to write the render object into the node anyway
        render_object: Option<
            // This field is needed in case a new descendant render object pops up.
            ArcRenderObjectOf<E>,
        >,
    },
    /// End-of-visit is triggered when both the node and its descendants (i.e. entire subtree) does not need reconcile
    EndOfVisit,
}

impl<E> ElementNode<E>
where
    E: Element,
{
    fn visit_inspect<'a, 'batch>(
        self: &Arc<Self>,
        widget: Option<E::ArcWidget>,
        reconcile_context: SyncReconcileContext<'a, 'batch>,
    ) -> VisitAction<E> {
        // Subtree has no work, end of visit
        if !self.context.subtree_lanes().contains(LanePos::Sync) {
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

        // Self has no work, but subtree has work. Visit
        if Self::can_skip_work(&widget, old_widget, LanePos::Sync, &self.context) {
            use MainlineState::*;
            match state {
                Ready {
                    element,
                    children,
                    render_object,
                    ..
                } => VisitAction::Visit {
                    element: element.clone(),
                    children: children.map_ref_collect(Clone::clone),
                    render_object: render_object.as_ref().ok().cloned(),
                },
                RebuildSuspended {
                    element, children, ..
                } => VisitAction::Visit {
                    element: element.clone(),
                    children: children.map_ref_collect(Clone::clone),
                    render_object: None,
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
                reconcile_context.tree_scheduler,
            )
            .ok()
            .expect("Impossible to fail");
            Some(cancel)
        } else {
            None
        };

        // Cannot skip work but can skip rebuild, meaning there is a polling work here.
        if Self::can_skip_rebuild(&widget, old_widget, LanePos::Sync, &self.context) {
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

    fn rebuild<'a, 'batch>(
        self: &Arc<Self>,
        widget: Option<E::ArcWidget>,
        reconcile_context: SyncReconcileContext<'a, 'batch>,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        let visit_action = self.visit_inspect(widget, reconcile_context);
        match visit_action {
            VisitAction::Rebuild {
                is_poll,
                old_widget,
                new_widget,
                state,
                cancel_async,
            } => todo!(),
            VisitAction::Visit {
                element,
                children,
                render_object,
            } => {
                let results = children
                    .par_map_collect(&get_current_scheduler().sync_threadpool, |child| {
                        child.visit_and_work_sync(reconcile_context)
                    });
                let (children, render_object_changes) = results.unzip_collect(|x| x);
                let render_object_change_summary =
                    SubtreeRenderObjectChange::summarize(render_object_changes.as_iter());

                if is_non_render_element::<E>() {
                    let RenderElementFunctionTable::None {
                        as_child,
                        into_subtree_render_object_change,
                    } = render_element_function_table_of::<E>()
                    else {
                        panic!(
                            "Invoked method from non-render render element on other element types"
                        )
                    };
                    return into_subtree_render_object_change(render_object_changes);
                } else if is_non_suspense_render_element::<E>() {
                    
                }
                todo!()
            }
            VisitAction::EndOfVisit => SubtreeRenderObjectChange::new_no_update(),
        }
    }

    fn apply_updates_sync_new<'a, 'batch>(
        element_context: &ElementContextNode,
        job_ids: &'a Inlinable64Vec<JobId>,
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
            todo!()
        }
    }

    fn process_component_subtree(
        updates: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        let RenderElementFunctionTable::None {
            into_subtree_render_object_change: into_subtree_update,
            ..
        } = render_element_function_table_of::<E>()
        else {
            panic!("Invoked method from component element on other element types")
        };

        into_subtree_update(updates)
    }

    fn commit_write_element_sync(self: &Arc<Self>, state: MainlineState<E>) {
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

    fn commit_write_inflate_element_sync(self: &Arc<Self>, state: MainlineState<E>) {
        todo!()
    }

    fn update_provided_value<'a, 'batch>(
        old_widget: &'a E::ArcWidget,
        new_widget: &'a E::ArcWidget,
        element_context: &'a ElementContextNode,
        tree_scheduler: &'batch TreeScheduler,
    ) {
        if let Some(get_provided_value) = E::GET_PROVIDED_VALUE {
            let old_provided_value = get_provided_value(&old_widget);
            let new_provided_value = get_provided_value(new_widget);
            if !Asc::ptr_eq(&old_provided_value, &new_provided_value)
                && !old_provided_value.eq_sized(new_provided_value.as_ref())
            {
                let contending_readers = element_context
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

struct DetachRenderObjectAccessor;

impl<E> NodeAccessor<AccessArcRenderObject<E>> for DetachRenderObjectAccessor
where
    E: Element,
{
    type Probe = ();

    type Return = ();

    fn can_bypass(self, node: &AccessArcRenderObject<E>) -> Result<Self::Return, Self::Probe> {
        Err(())
    }

    fn access(
        inner: &mut <AccessArcRenderObject<E> as AccessNode>::Inner<'_>,
        probe: Self::Probe,
    ) -> Self::Return {
        todo!()
    }
}

// pub(crate) mod sync_build_private {
//     use crate::{
//         foundation::{Inlinable64Vec, Protocol},
//         tree::ArcAnyElementNode,
//     };

//     use super::*;

//     pub trait AnyElementSyncReconcileExt {
//         fn visit_and_work_sync<'a, 'batch>(
//             self: Arc<Self>,
//             reconcile_context: SyncReconcileContext<'a, 'batch>,
//         ) -> ArcAnyElementNode;
//     }

//     impl<E> AnyElementSyncReconcileExt for ElementNode<E>
//     where
//         E: Element,
//     {
//         fn visit_and_work_sync<'a, 'batch>(
//             self: Arc<Self>,
//             reconcile_context: SyncReconcileContext<'a, 'batch>,
//         ) -> ArcAnyElementNode {
//             self.rebuild_node_sync(None, reconcile_context);
//             self
//         }
//     }

//     pub trait ChildElementSyncReconcileExt<PP: Protocol> {
//         fn visit_and_work_sync<'a, 'batch>(
//             self: Arc<Self>,
//             reconcile_context: SyncReconcileContext<'a, 'batch>,
//         ) -> (ArcChildElementNode<PP>, SubtreeRenderObjectChange<PP>);
//     }

//     impl<E> ChildElementSyncReconcileExt<E::ParentProtocol> for ElementNode<E>
//     where
//         E: Element,
//     {
//         fn visit_and_work_sync<'a, 'batch>(
//             self: Arc<Self>,
//             reconcile_context: SyncReconcileContext<'a, 'batch>,
//         ) -> (
//             ArcChildElementNode<E::ParentProtocol>,
//             SubtreeRenderObjectChange<E::ParentProtocol>,
//         ) {
//             let result = self.rebuild_node_sync(None, reconcile_context);
//             (self, result)
//         }
//     }
// }

// #[derive(Clone, Copy)]
// pub(crate) struct SyncReconcileContext<'a, 'batch> {
//     job_ids: &'a Inlinable64Vec<JobId>,
//     scope: &'a rayon::Scope<'batch>,
//     tree_scheduler: &'batch TreeScheduler,
// }
