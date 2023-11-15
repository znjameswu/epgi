use crate::{
    foundation::{Arc, Inlinable64Vec},
    scheduler::JobId,
    sync::{SubtreeRenderObjectChange, TreeScheduler},
    tree::{ArcChildElementNode, ArcElementContextNode, Element, ElementNode},
};

// enum VisitAction<E: Element> {
//     Rebuild {
//         is_poll: bool,
//         old_widget: E::ArcWidget,
//         state: MainlineState<E>,
//         cancel_async: Option<CancelAsync<ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>>>,
//     },
//     Visit {
//         element: E,
//         children: ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
//         // This field is needed in case a new descendant render object pops up.
//         render_object: ArcRenderObjectOf<E>,
//     },
//     VisitSuspended {
//         element: E,
//         // This field is needed in case the subtree is not suspended anymore and a new render object needs to be created
//         widget: E::ArcWidget,
//         children: ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
//     },
//     Stop,
// }

impl<E> ElementNode<E>
where
    E: Element,
{
    // Reconciler needs this
    pub(in super::super) fn rebuild_node_sync<'a, 'batch>(
        self: &Arc<Self>,
        widget: Option<E::ArcWidget>,
        reconcile_context: SyncReconcileContext<'a, 'batch>,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        todo!()
    }

    //         if !self.context.subtree_lanes().contains(LanePos::Sync) {
    //             return SubtreeRenderObjectChange::Keep {
    //                 child_render_action: RerenderAction::None,
    //                 subtree_has_action: RerenderAction::None,
    //             };
    //         }

    //         // Skip variant will not occupy the node (i.e., Option::take() from the shared states)
    //         struct SyncRebuild<E: Element> {
    //             is_poll: bool,
    //             old_widget: E::ArcWidget,
    //             state: MainlineState<E>,
    //             cancel_async:
    //                 Option<CancelAsync<ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>>>,
    //         }
    //         let rebuild: Result<
    //             SyncRebuild<E>,
    //             Option<ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>>,
    //         > = {
    //             let mut snapshot = self.snapshot.lock();
    //             // https://bevy-cheatbook.github.io/pitfalls/split-borrows.html
    //             let snapshot_reborrow = &mut *snapshot;
    //             let old_widget = &mut snapshot_reborrow.widget;

    //             let mainline = snapshot_reborrow
    //                 .inner
    //                 .mainline_mut()
    //                 .expect("An unmounted element node should not be reachable by a rebuild!");

    //             let state = mainline.state.as_ref().expect(
    //                 "A sync task should not encounter another sync task contending over the same node",
    //             );
    //             if Self::can_skip_work(&widget, old_widget, LanePos::Sync, &self.context) {
    //                 Err(state.children_cloned())
    //             } else {
    //                 let state = (&mut mainline.state).take().expect("Impossible to fail"); // rust-analyzer#14933
    //                                                                                        // Not able to use `Option::map` due to closure lifetime problem.
    //                 let cancel_async = if let Some(entry) = mainline.async_queue.current() {
    //                     let cancel = Self::prepare_cancel_async_work(
    //                         mainline,
    //                         entry.work.context.lane_pos,
    //                         reconcile_context.tree_scheduler,
    //                     )
    //                     .ok()
    //                     .expect("Impossible to fail");
    //                     Some(cancel)
    //                 } else {
    //                     None
    //                 };
    //                 if Self::can_skip_rebuild(&widget, old_widget, LanePos::Sync, &self.context) {
    //                     // Cannot skip work but can skip rebuild, meaning there is a polling work here.
    //                     Ok(SyncRebuild {
    //                         is_poll: true,
    //                         old_widget: old_widget.clone(),
    //                         state,
    //                         cancel_async,
    //                     })
    //                 } else {
    //                     let old_widget = if let Some(widget) = &widget {
    //                         std::mem::replace(old_widget, widget.clone())
    //                     } else {
    //                         old_widget.clone()
    //                     };
    //                     Ok(SyncRebuild {
    //                         is_poll: false,
    //                         old_widget,
    //                         state,
    //                         cancel_async,
    //                     })
    //                 }
    //             }
    //         };

    //         match rebuild {
    //             Ok(SyncRebuild {
    //                 is_poll,
    //                 old_widget,
    //                 state,
    //                 cancel_async,
    //             }) => {
    //                 if let Some(cancel_async) = cancel_async {
    //                     self.perform_cancel_async_work(cancel_async)
    //                 }
    //                 let new_widget_ref = widget.as_ref().unwrap_or(&old_widget);
    //                 let consumed_values = Self::read_and_update_subscriptions_sync(
    //                     E::get_consumed_types(new_widget_ref),
    //                     E::get_consumed_types(&old_widget),
    //                     &self.context,
    //                     reconcile_context.tree_scheduler,
    //                 );
    //                 if let Some(widget) = widget.as_ref() {
    //                     Self::update_provided_value(
    //                         &old_widget,
    //                         widget,
    //                         &self.context,
    //                         reconcile_context.tree_scheduler,
    //                     )
    //                 }
    //                 let new_widget = &widget.unwrap_or(old_widget);
    //                 match state {
    //                     MainlineState::Ready {
    //                         element,
    //                         children,
    //                         hooks,
    //                         render_object,
    //                     } => {
    //                         assert!(!is_poll, "A non-suspended node should not be polled");
    //                         Self::apply_updates_sync(
    //                             &self.context,
    //                             reconcile_context.job_ids,
    //                             &mut hooks,
    //                         );
    //                         self.perform_rebuild_node_sync_new(
    //                             new_widget,
    //                             element,
    //                             children,
    //                             HookContext::new_rebuild(hooks),
    //                             render_object,
    //                             consumed_values,
    //                             reconcile_context,
    //                         )
    //                     }
    //                     MainlineState::RebuildSuspended {
    //                         element,
    //                         children,
    //                         suspended_hooks,
    //                         waker,
    //                     } => {
    //                         waker.abort();
    //                         // If a new job occurred on this previously suspended node
    //                         if !is_poll {
    //                             Self::apply_updates_sync(
    //                                 &self.context,
    //                                 reconcile_context.job_ids,
    //                                 &mut suspended_hooks,
    //                             );
    //                         }
    //                         self.perform_rebuild_node_sync_new(
    //                             new_widget,
    //                             element,
    //                             children,
    //                             HookContext::new_rebuild(suspended_hooks),
    //                             None,
    //                             consumed_values,
    //                             reconcile_context,
    //                         )
    //                     }
    //                     MainlineState::InflateSuspended {
    //                         suspended_hooks,
    //                         waker,
    //                     } => {
    //                         waker.abort();
    //                         self.perform_inflate_node_sync_new(
    //                             new_widget,
    //                             if !is_poll {
    //                                 HookContext::new_inflate()
    //                             } else {
    //                                 HookContext::new_poll_inflate(suspended_hooks)
    //                             },
    //                             consumed_values,
    //                             reconcile_context,
    //                         )
    //                     }
    //                 }
    //             }

    //             Err((element, widget, children, old_render_object)) => {
    //                 let results = children
    //                     .par_map_collect(&get_current_scheduler().sync_threadpool, |child| {
    //                         child.visit_and_work_sync(reconcile_context)
    //                     });
    //                 let (children, updates) = results.unzip_collect(|x| x);
    //                 if is_non_suspense_render_element::<E>() {
    //                     let (new_render_object, suspend_changed) =
    //                         if let Some(old_render_object) = old_render_object {
    //                             let render_object = Self::commit_update_render_object(
    //                                 old_render_object,
    //                                 widget,
    //                                 None,
    //                                 updates,
    //                             );
    //                             (render_object, render_object.is_none())
    //                         } else {
    //                             let render_object = Self::commit_create_render_object(
    //                                 element,
    //                                 widget,
    //                                 &children,
    //                                 updates,
    //                                 &self.context,
    //                             );
    //                             (render_object, render_object.is_some())
    //                         };
    //                     if suspend_changed {
    //                         {
    //                             let mut snapshot = self.snapshot.lock();
    //                             let snapshot_reborrow = &mut *snapshot;
    //                             let state = snapshot_reborrow
    //                                 .inner
    //                                 .mainline_mut()
    //                                 .expect("An unmounted element node should not be reachable by a rebuild!")
    //                                 .state
    //                                 .as_mut()
    //                                 .expect("");
    //                             let MainlineState::Ready {
    //                                 element,
    //                                 hooks,
    //                                 children,
    //                                 render_object,
    //                             } = state
    //                             else {
    //                                 panic!(
    //                                     "Node state corrupted during sync tree visit! \
    //                                     This sync tree visit has previously found this node to be in the ready state \
    //                                     and did not occupy this node. \
    //                                     However, when the visit returned to this node again, \
    //                                     it found the node state has changed from the ready state. \
    //                                     This indicates a write was commited into the node, \
    //                                     which is strictly forbidden during a sync tree visit"
    //                                 )
    //                             };
    //                             *render_object = new_render_object;
    //                         }
    //                     }
    //                 } else if is_suspense_element::<E>() {
    //                     Self::commit_suspense_updated();
    //                 } else if true {
    //                 }

    //                 // TODO: Absorb new renderobject from subtree by updating the children of this renderobject
    //                 return todo!();
    //             }

    //             Err(None) => SubtreeRenderObjectChange::Keep {
    //                 child_render_action: RerenderAction::None,
    //                 subtree_has_action: RerenderAction::None,
    //             },
    //         }
    //     }

    // Reconciler needs this
    pub(in super::super) fn inflate_node_sync<'a, 'batch>(
        widget: &E::ArcWidget,
        parent_context: ArcElementContextNode,
        reconcile_context: SyncReconcileContext<'a, 'batch>,
    ) -> (
        Arc<ElementNode<E>>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        todo!()
    }
}
//         let node = Arc::new_cyclic(|weak| ElementNode {
//             context: Arc::new(ElementContextNode::new_for::<E>(
//                 weak.clone() as _,
//                 parent_context,
//                 widget,
//             )),
//             snapshot: SyncMutex::new(ElementSnapshot {
//                 widget: widget.clone(),
//                 inner: ElementSnapshotInner::Mainline(Mainline {
//                     state: None,
//                     async_queue: AsyncWorkQueue::new_empty(),
//                 }),
//             }),
//         });

//         // let weak_node: AweakAnyElementNode = Arc::downgrade(&node) as _;
//         let consumed_values = Self::read_and_update_subscriptions_sync(
//             E::get_consumed_types(widget),
//             EMPTY_CONSUMED_TYPES,
//             &node.context,
//             reconcile_context.tree_scheduler,
//         );

//         let subtree_results = Self::perform_inflate_node_sync_new(
//             &node,
//             widget,
//             HookContext::new_inflate(),
//             consumed_values,
//             reconcile_context,
//         );
//         // node.snapshot.lock().inner = snapshot_inner;
//         return (node, subtree_results);
//     }

//     fn apply_updates_sync<'a, 'batch>(
//         element_context: &ElementContextNode,
//         job_ids: &'a Inlinable64Vec<JobId>,
//         hooks: &mut Hooks,
//     ) {
//         let mut jobs = {
//             element_context
//                 .mailbox
//                 .lock()
//                 .extract_if(|job_id, _| job_ids.contains(job_id))
//                 .collect::<Vec<_>>()
//         };
//         jobs.sort_by_key(|(job_id, ..)| *job_id);

//         let updates = jobs
//             .into_iter()
//             .flat_map(|(_, updates)| updates)
//             .collect::<Vec<_>>();

//         for update in updates {
//             todo!()
//         }
//     }

//     fn perform_rebuild_node_sync_new<'a, 'batch>(
//         self: &'a Arc<Self>,
//         widget: &'a E::ArcWidget,
//         element: E,
//         children: ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
//         mut hook_context: HookContext,
//         old_render_object: Option<ArcRenderObjectOf<E>>,
//         provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
//         reconcile_context: SyncReconcileContext<'a, 'batch>,
//     ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
//         let mut nodes_needing_unmount = Default::default();
//         let results = element.perform_rebuild_element(
//             &widget,
//             BuildContext {
//                 hooks: &mut hook_context,
//                 element_context: &self.context,
//             },
//             provider_values,
//             children,
//             &mut nodes_needing_unmount,
//         );

//         match results {
//             Err((children, err)) => {
//                 if is_non_suspense_render_element::<E>() {
//                     if let Some(old_render_object) = old_render_object.as_ref() {
//                         Self::commit_suspend_render_object(old_render_object)
//                     }
//                 } else if is_suspense_element::<E>() {
//                     panic!("Suspense element should never suspend by itself")
//                 }

//                 debug_assert!(
//                     nodes_needing_unmount.is_empty(),
//                     "An element that suspends itself should not request unmounting any child nodes"
//                 );

//                 self.commit_write_element_sync(MainlineState::RebuildSuspended {
//                     suspended_hooks: hook_context.hooks,
//                     element,
//                     children,
//                     waker: err.waker,
//                 });

//                 return SubtreeRenderObjectChange::Suspended;
//             }
//             Ok((items, callback)) => {
//                 // Starting the unmounting as early as possible.
//                 // Unmount before updating render object can cause render object to hold reference to detached children,
//                 // Therfore, we need to ensure we do not read into render objects before the batch commit is done
//                 for node_needing_unmount in nodes_needing_unmount {
//                     reconcile_context.scope.spawn(|scope| {
//                         // node_needing_unmount.unmount()
//                         todo!()
//                     })
//                 }

//                 let results =
//                     items.par_map_collect(&get_current_scheduler().sync_threadpool, |item| {
//                         use ElementReconcileItem::*;
//                         match item {
//                             Keep(node) => node.visit_and_work_sync(reconcile_context),
//                             Update(pair) => pair.rebuild_sync(reconcile_context),
//                             Inflate(widget) => {
//                                 widget.inflate_sync(self.context.clone(), reconcile_context)
//                             }
//                         }
//                     });
//                 let (children, updates) = results.unzip_collect(|x| x);

//                 let child_commit_summary =
//                     SubtreeRenderObjectChange::summarize(updates.as_iter());

//                 use SubtreeRenderObjectCommitResultSummary::*;
//                 match child_commit_summary {
//                     Suspended => {
//                         access_node(
//                             AccessArcRenderObject(old_render_object),
//                             DetachRenderObjectAccessor,
//                         );
//                     }
//                     KeepRenderObject {
//                         child_render_action,
//                         subtree_has_action,
//                     } => todo!(),
//                     NewRenderObject => {
//                         access_node(AccessArcRenderObject(old_render_object), UpdateRenderAccessor)
//                     },
//                 };
//                 let (render_object, subtree_update) = if is_non_suspense_render_element::<E>() {
//                     if let Some(old_render_object) = old_render_object {
//                         Self::commit_update_render_object(
//                             old_render_object,
//                             widget,
//                             callback,
//                             updates,
//                         )
//                     } else {
//                         Self::commit_create_render_object(
//                             &element,
//                             widget,
//                             &children,
//                             updates,
//                             &self.context,
//                         )
//                     }
//                 } else if is_suspense_element::<E>() {
//                     Self::commit_suspense_updated(
//                         old_render_object.expect(
//                             "An exisiting Suspense should always have an attached render object",
//                         ),
//                         widget,
//                         callback,
//                         updates,
//                     )
//                 };

//                 self.commit_write_element_sync(MainlineState::Ready {
//                     element,
//                     children,
//                     hooks: hook_context.hooks,
//                     render_object,
//                 });

//                 return subtree_update;
//             }
//         }
//     }

//     fn perform_inflate_node_sync_new<'a, 'batch>(
//         self: &'a Arc<Self>,
//         widget: &'a E::ArcWidget,
//         mut hook_context: HookContext,
//         provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
//         reconcile_context: SyncReconcileContext<'a, 'batch>,
//     ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
//         let result = E::perform_inflate_element(
//             &widget,
//             BuildContext {
//                 hooks: &mut hook_context,
//                 element_context: &self.context,
//             },
//             provider_values,
//         );

//         match result {
//             Err(err) => {
//                 if is_suspense_element::<E>() {
//                     panic!("Suspense element should never suspend by itself")
//                 }
//                 self.commit_write_inflate_element_sync(MainlineState::InflateSuspended {
//                     suspended_hooks: hook_context.hooks,
//                     waker: err.waker,
//                 });

//                 return SubtreeRenderObjectChange::Suspended;
//             }
//             Ok((element, child_widgets)) => {
//                 let results = child_widgets.par_map_collect(
//                     &get_current_scheduler().sync_threadpool,
//                     |child_widget| {
//                         child_widget.inflate_sync(self.context.clone(), reconcile_context)
//                     },
//                 );
//                 let (children, updates) = results.unzip_collect(|x| x);

//                 debug_assert!(
//                     !updates.any(SubtreeRenderObjectChange::is_suspended),
//                     "Fatal logic bug in epgi-core reconcile logic. Please file issue report."
//                 );

//                 let (render_object, subtree_update) = if is_non_suspense_render_element::<E>() {
//                     Self::commit_create_render_object(
//                         &element,
//                         widget,
//                         &children,
//                         updates,
//                         &self.context,
//                     )
//                 } else if is_suspense_element::<E>() {
//                     todo!()
//                 } else {
//                     (None, Self::process_component_subtree(updates))
//                 };

//                 self.commit_write_inflate_element_sync(MainlineState::Ready {
//                     element,
//                     children,
//                     hooks: hook_context.hooks,
//                     render_object,
//                 });

//                 return subtree_update;
//             }
//         }
//     }

//     fn commit_update_render_object(
//         old_render_object: ArcRenderObjectOf<E>,
//         widget: &E::ArcWidget,
//         callback: Option<ChildRenderObjectsUpdateCallback<E>>,
//         subtree_updates: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
//     ) -> (
//         Option<ArcRenderObjectOf<E>>,
//         SubtreeRenderObjectChange<E::ParentProtocol>,
//     ) {
//         let RenderElementFunctionTable::RenderObject {
//             into_arc_child_render_object,
//             update_render,
//             suspense: None,
//             ..
//         } = render_element_function_table_of::<E>()
//         else {
//             panic!("Invoked method from non-suspense render element on other element types")
//         };

//         let subtree_suspended = subtree_updates.any(SubtreeRenderObjectChange::is_suspended);
//         let subtree_no_update =
//             subtree_updates.all(SubtreeRenderObjectChange::is_keep_render_object);

//         if subtree_suspended {
//             Self::commit_suspend_render_object(old_render_object);
//             return (None, SubtreeRenderObjectChange::Suspended);
//         }
//         if !subtree_no_update || update_render.is_some() || callback.is_some() {
//             E::RenderOrUnit::with_inner(&old_render_object, move |render, children, context| {
//                 if let Some(update_render) = update_render {
//                     let update_results = (update_render)(render, widget);
//                     match update_results {
//                         RerenderAction::None => {}
//                         RerenderAction::Repaint => context.mark_needs_paint(),
//                         RerenderAction::Relayout => context.mark_needs_layout(),
//                     }
//                 }
//                 if let Some(callback) = callback {
//                     replace_with::replace_with_or_abort(children, move |children| {
//                         let items = (callback)(children);
//                         items.zip_collect(subtree_updates, |item, update| {
//                             use RenderObjectReconcileItem::*;
//                             use SubtreeRenderObjectChange::*;
//                             match (item, update) {
//                                 (New, New(render_object)) => render_object,
//                                 (Keep(render_object), Keep) => render_object,
//                                 (Keep(_), New(render_object)) => render_object,
//                                 (New, Keep) => panic!("Render object update callback bug: Requested for new render object while the corresponding slot is not producing one"),
//                                 (_, Suspended) => panic!("Fatal logic bug in epgi-core reconcile logic. Please file issue report.")
//                             }
//                         })
//                     })
//                 } else if !subtree_no_update {
//                     replace_with::replace_with_or_abort(children, move |children| {
//                         children.zip_collect(subtree_updates, |child, update| {
//                             use SubtreeRenderObjectChange::*;
//                             match update {
//                                 Keep => child,
//                                 New(render_object) => render_object,
//                                 Suspended => panic!("Fatal logic bug in epgi-core reconcile logic. Please file issue report."),
//                             }
//                         })
//                     })
//                 }
//             });
//         }
//         return (
//             Some(old_render_object),
//             SubtreeRenderObjectChange::Keep,
//         );
//     }

//     fn commit_suspend_render_object(old_render_object: ArcRenderObjectOf<E>) {
//         let RenderElementFunctionTable::RenderObject {
//             detach_render,
//             suspense: None,
//             ..
//         } = render_element_function_table_of::<E>()
//         else {
//             panic!("Invoked method from non-suspense render element on other element types")
//         };

//         if let Some(detach_render) = detach_render {
//             E::RenderOrUnit::with_inner(&old_render_object, detach_render)
//         }
//     }

//     fn commit_create_render_object(
//         element: &E,
//         widget: &E::ArcWidget,
//         element_children: &ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
//         updates: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
//         element_context: &ArcElementContextNode,
//     ) -> (
//         Option<ArcRenderObjectOf<E>>,
//         SubtreeRenderObjectChange<E::ParentProtocol>,
//     ) {
//         let RenderElementFunctionTable::RenderObject {
//             create_render,
//             create_render_object,
//             suspense: None,
//             into_arc_child_render_object,
//             ..
//         } = render_element_function_table_of::<E>()
//         else {
//             panic!("Invoked method from non-suspense render element on other element types")
//         };

//         let suspended = updates.any(SubtreeRenderObjectChange::is_suspended);
//         if suspended {
//             return (None, SubtreeRenderObjectChange::Suspended);
//         }
//         let render = create_render(element, widget);
//         let children = element_children.zip_ref_collect(updates, |element_child, update| {
//             use SubtreeRenderObjectChange::*;
//             match update {
//                 Keep => element_child.get_current_subtree_render_object(),
//                 New(render_object) => Some(render_object),
//                 Suspended => panic!(
//                     "Fatal logic bug in epgi-core reconcile logic. Please file issue report."
//                 ),
//             }
//         });

//         let render_object = create_render_object(render, children, element_context.clone());

//         (
//             Some(render_object),
//             SubtreeRenderObjectChange::New(into_arc_child_render_object(
//                 render_object.clone(),
//             )),
//         )
//     }

//     fn commit_suspend_suspense<'a, 'batch>(
//         old_render_object: ArcRenderObjectOf<E>,
//         mut element: E,
//         element_context: &ArcElementContextNode,
//         reconcile_context: SyncReconcileContext<'a, 'batch>,
//     ) {
//         match render_element_function_table_of::<E>() {
//             RenderElementFunctionTable::RenderObject {
//                 detach_render: detach_render_object,
//                 suspense:
//                     Some(SuspenseElementFunctionTable {
//                         get_suspense_element_mut,
//                         get_suspense_widget_ref,
//                         get_suspense_render_object,
//                         into_arc_render_object,
//                     }),
//                 ..
//             } => {
//                 let suspense_element = get_suspense_element_mut(&mut element);
//                 if suspense_element.suspended {
//                     return;
//                 }
//                 let old_render_object = old_render_object
//                     .expect("Suspense should always have a non-suspended render object");
//                 let (node, subtree_results) = rayon::scope(|scope| {
//                     suspense_element
//                         .fallback_widget
//                         .clone()
//                         .inflate_sync(element_context.clone(), reconcile_context)
//                 });
//                 let SubtreeRenderObjectChange::New(render_object) =
//                     subtree_results
//                 else {
//                     panic!("Fallback widget must not suspend and its subtree must always provide an attached renderobject")
//                 };
//                 todo!()
//             }
//             _ => {
//                 todo!()
//             }
//         }
//     }

//     fn process_component_subtree(
//         updates: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
//     ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
//         let RenderElementFunctionTable::None {
//             into_subtree_update,
//             ..
//         } = render_element_function_table_of::<E>()
//         else {
//             panic!("Invoked method from component element on other element types")
//         };

//         into_subtree_update(updates)
//     }

//     fn commit_write_element_sync(self: &Arc<Self>, state: MainlineState<E>) {
//         let async_work_needing_start = {
//             let mut snapshot = self.snapshot.lock();
//             let snapshot_reborrow = &mut *snapshot;
//             let mainline = snapshot_reborrow
//                 .inner
//                 .mainline_mut()
//                 .expect("An unmounted element node should not be reachable by a rebuild!");
//             debug_assert!(
//                 mainline.async_queue.current().is_none(),
//                 "An async work should not be executing alongside a sync work"
//             );
//             mainline.state = Some(state);
//             self.prepare_execute_backqueue(mainline, &snapshot_reborrow.widget)
//         };

//         if let Some(async_work_needing_start) = async_work_needing_start {
//             let node = self.clone();
//             node.execute_rebuild_node_async_detached(async_work_needing_start);
//         }
//     }

//     fn commit_write_inflate_element_sync(self: &Arc<Self>, state: MainlineState<E>) {
//         todo!()
//     }

//     fn update_provided_value<'a, 'batch>(
//         old_widget: &'a E::ArcWidget,
//         new_widget: &'a E::ArcWidget,
//         element_context: &'a ElementContextNode,
//         tree_scheduler: &'batch TreeScheduler,
//     ) {
//         if let Some(get_provided_value) = E::GET_PROVIDED_VALUE {
//             let old_provided_value = get_provided_value(&old_widget);
//             let new_provided_value = get_provided_value(new_widget);
//             if !Asc::ptr_eq(&old_provided_value, &new_provided_value)
//                 && !old_provided_value.eq_sized(new_provided_value.as_ref())
//             {
//                 let contending_readers = element_context
//                     .provider
//                     .as_ref()
//                     .expect("Element with a provided value should have a provider")
//                     .write_sync(new_provided_value);

//                 contending_readers.non_mainline.par_for_each(
//                     &get_current_scheduler().sync_threadpool,
//                     |(lane_pos, node)| {
//                         let node = node.upgrade().expect("ElementNode should be alive");
//                         node.restart_async_work(lane_pos, tree_scheduler)
//                     },
//                 );

//                 // This is the a operation, we do not fear any inconsistencies caused by cancellation.
//                 for reader in contending_readers.mainline {
//                     reader
//                         .upgrade()
//                         .expect("Readers should be alive")
//                         .mark_secondary_root(LanePos::Sync)
//                 }
//             }
//         }
//     }

//     fn read_and_update_subscriptions_sync(
//         new_consumed_types: &[TypeKey],
//         old_consumed_types: &[TypeKey],
//         element_context: &ArcElementContextNode,
//         tree_scheduler: &TreeScheduler,
//     ) -> InlinableDwsizeVec<Arc<dyn Provide>> {
//         let is_old_consumed_types = std::ptr::eq(new_consumed_types, old_consumed_types);

//         // Unregister
//         for consumed in old_consumed_types.iter() {
//             if !new_consumed_types.contains(consumed) {
//                 let removed = element_context
//                     .provider_map
//                     .get(consumed)
//                     .expect("ProviderMap should be consistent")
//                     .provider
//                     .as_ref()
//                     .expect("Element should provide types according to ProviderMap")
//                     .unregister_read(&Arc::downgrade(element_context));
//                 debug_assert!(removed)
//             }
//         }

//         // Why do we need to restart contending async writers at all?
//         // Because if we are registering a new read, they will be unaware of us as a secondary root.

//         // We only need to cancel contending async writers only if this is a new subscription.
//         // Because a contending async writer on an old subsciption will naturally find this node as a secondary root.

//         // We only need to cancel the topmost contending writes from a single lane. Because all its subtree will be purged.
//         let mut async_work_needs_restarting = LinearMap::<LanePos, ArcElementContextNode>::new();

//         let consumed_values = new_consumed_types
//             .iter()
//             .map(|consumed| {
//                 let is_old = is_old_consumed_types || old_consumed_types.contains(consumed);
//                 let subscription = element_context
//                     .provider_map
//                     .get(consumed)
//                     .expect("Requested provider should exist");
//                 let provider = subscription
//                     .provider
//                     .as_ref()
//                     .expect("Element should provide types according to ProviderMap");
//                 if !is_old {
//                     let contending_writer = provider.register_read(Arc::downgrade(element_context));
//                     if let Some(contending_lane) = contending_writer {
//                         async_work_needs_restarting
//                             .entry(contending_lane)
//                             .and_modify(|v| {
//                                 if v.depth < subscription.depth {
//                                     *v = subscription.clone()
//                                 }
//                             })
//                             .or_insert_with(|| subscription.clone());
//                     }
//                 }
//                 provider.read()
//             })
//             .collect();
//         let async_work_needs_restarting: Vec<_> = async_work_needs_restarting.into();
//         async_work_needs_restarting.par_for_each(
//             &get_current_scheduler().sync_threadpool,
//             |(lane_pos, context)| {
//                 let node = context
//                     .element_node
//                     .upgrade()
//                     .expect("ElementNode should be alive");
//                 node.restart_async_work(lane_pos, tree_scheduler)
//             },
//         );
//         return consumed_values;
//     }
// }

// struct UpdateRenderChildrenAccessor<E: Element> {
//     shuffle: Option<ChildRenderObjectsUpdateCallback<E>>,
//     child_commits: ContainerOf<E, SubtreeRenderObjectChange<E::ChildProtocol>>,
//     child_commit_summary: SubtreeRenderObjectCommitResultSummary
// }

// struct DetachRenderObjectAccessor;

// impl<E> NodeAccessor<AccessArcRenderObject<E>> for DetachRenderObjectAccessor
// where
//     E: Element,
// {
//     type Probe = ();

//     type Return = ();

//     fn can_bypass(self, node: &AccessArcRenderObject<E>) -> Result<Self::Return, Self::Probe> {
//         Err(())
//     }

//     fn access(
//         inner: <AccessArcRenderObject<E> as AccessNode>::Inner<'_>,
//         probe: Self::Probe,
//     ) -> Self::Return {
//         todo!()
//     }
// }

pub(crate) mod sync_build_private {
    use crate::{foundation::Protocol, tree::ArcAnyElementNode};

    use super::*;

    pub trait AnyElementSyncReconcileExt {
        fn visit_and_work_sync<'a, 'batch>(
            self: Arc<Self>,
            reconcile_context: SyncReconcileContext<'a, 'batch>,
        ) -> ArcAnyElementNode;
    }

    impl<E> AnyElementSyncReconcileExt for ElementNode<E>
    where
        E: Element,
    {
        fn visit_and_work_sync<'a, 'batch>(
            self: Arc<Self>,
            reconcile_context: SyncReconcileContext<'a, 'batch>,
        ) -> ArcAnyElementNode {
            self.rebuild_node_sync(None, reconcile_context);
            self
        }
    }

    pub trait ChildElementSyncReconcileExt<PP: Protocol> {
        fn visit_and_work_sync<'a, 'batch>(
            self: Arc<Self>,
            reconcile_context: SyncReconcileContext<'a, 'batch>,
        ) -> (ArcChildElementNode<PP>, SubtreeRenderObjectChange<PP>);
    }

    impl<E> ChildElementSyncReconcileExt<E::ParentProtocol> for ElementNode<E>
    where
        E: Element,
    {
        fn visit_and_work_sync<'a, 'batch>(
            self: Arc<Self>,
            reconcile_context: SyncReconcileContext<'a, 'batch>,
        ) -> (
            ArcChildElementNode<E::ParentProtocol>,
            SubtreeRenderObjectChange<E::ParentProtocol>,
        ) {
            let result = self.rebuild_node_sync(None, reconcile_context);
            (self, result)
        }
    }
}

#[derive(Clone, Copy)]
pub struct SyncReconcileContext<'a, 'batch> {
    pub(crate) job_ids: &'a Inlinable64Vec<JobId>,
    pub(crate) scope: &'a rayon::Scope<'batch>,
    pub(crate) tree_scheduler: &'batch TreeScheduler,
}
