use crate::{
    foundation::{Arc, Container, ContainerOf, Protocol},
    scheduler::{get_current_scheduler, LaneMask},
    sync::{ImplReconcileCommit, LaneScheduler, SubtreeRenderObjectChange},
    tree::{
        ArcAnyElementNode, ArcChildElementNode, AsyncInflating, AsyncOutput,
        AsyncQueueCurrentEntry, AsyncStash, AsyncWorkQueue, BuildResults, Element, ElementNode,
        ElementSnapshotInner, FullElement, HookContextMode, HooksWithTearDowns, ImplElementNode,
        Mainline, MainlineState, SubscriptionDiff,
    },
};

pub trait AnyElementAsyncCommitExt {
    fn visit_and_commit_async_any(
        self: Arc<Self>,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'_>,
        lane_scheduler: &LaneScheduler,
    ) -> ArcAnyElementNode;
}

impl<E: FullElement> AnyElementAsyncCommitExt for ElementNode<E> {
    fn visit_and_commit_async_any(
        self: Arc<Self>,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'_>,
        lane_scheduler: &LaneScheduler,
    ) -> ArcAnyElementNode {
        self.visit_and_commit_async_impl(finished_lanes, scope, lane_scheduler);
        self
    }
}

pub trait ChildElementAsyncCommitExt<PP: Protocol> {
    fn visit_and_commit_async(
        self: Arc<Self>,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'_>,
        lane_scheduler: &LaneScheduler,
    ) -> SubtreeRenderObjectChange<PP>;
}

impl<E: FullElement> ChildElementAsyncCommitExt<E::ParentProtocol> for ElementNode<E> {
    fn visit_and_commit_async(
        self: Arc<Self>,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'_>,
        lane_scheduler: &LaneScheduler,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        self.visit_and_commit_async_impl(finished_lanes, scope, lane_scheduler)
    }
}

impl<E> ElementNode<E>
where
    E: FullElement,
{
    fn visit_and_commit_async_impl(
        &self,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'_>,
        lane_scheduler: &LaneScheduler,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        let prepare_result = self.prepare_visit_and_commit_async(finished_lanes);
        use PrepareCommitAsyncResult::*;
        match prepare_result {
            SkipAndVisitChildren {
                children,
                render_object,
                self_rebuild_suspended,
            } => {
                let render_object_changes = children
                    .par_map_collect(&get_current_scheduler().async_threadpool, |child| {
                        child.visit_and_commit_async(finished_lanes, scope, lane_scheduler)
                    });
                return <E as Element>::Impl::visit_commit(
                    &self,
                    render_object,
                    render_object_changes,
                    lane_scheduler,
                    scope,
                    self_rebuild_suspended,
                );
            }
            VisitChildrenAndCommit { children } => {
                let render_object_changes = children
                    .par_map_collect(&get_current_scheduler().async_threadpool, |child| {
                        child.visit_and_commit_async(finished_lanes, scope, lane_scheduler)
                    });
                self.execute_commit_async(
                    render_object_changes,
                    finished_lanes,
                    scope,
                    lane_scheduler,
                )
            }
            InflateSuspended => SubtreeRenderObjectChange::Suspend,
            SkipAndReturn => SubtreeRenderObjectChange::new_no_update(),
        }
    }
}

enum PrepareCommitAsyncResult<E: Element> {
    VisitChildrenAndCommit {
        children: ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
    },
    InflateSuspended,
    SkipAndVisitChildren {
        children: ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
        render_object: <<E as Element>::Impl as ImplElementNode<E>>::OptionArcRenderObject,
        // Optimization parameter carried over from sync commit (Because the infrasturcture is build around sync commit)
        self_rebuild_suspended: bool,
    },
    SkipAndReturn,
}

impl<E> ElementNode<E>
where
    E: FullElement,
{
    fn prepare_visit_and_commit_async(
        &self,
        finished_lanes: LaneMask,
    ) -> PrepareCommitAsyncResult<E> {
        use PrepareCommitAsyncResult::*;
        let mut snapshot = self.snapshot.lock();
        // https://bevy-cheatbook.github.io/pitfalls/split-borrows.html
        let snapshot_reborrow = &mut *snapshot;

        match &mut snapshot_reborrow.inner {
            ElementSnapshotInner::AsyncInflating(AsyncInflating {
                work_context,
                stash,
            }) => {
                assert!(
                    finished_lanes.contains(work_context.lane_pos),
                    "Async commit should not visit into non-mainline nodes from other lanes"
                );
                match &mut stash.output {
                    AsyncOutput::Completed(results) => {
                        debug_assert!(
                            results.rebuild_state.is_none(),
                            "Async inflate node should not have a rebuild results"
                        );
                        return VisitChildrenAndCommit {
                            children: results.children.map_ref_collect(Clone::clone),
                        };
                    }
                    AsyncOutput::Suspended {
                        suspend,
                        barrier: None,
                    } => {
                        let result = suspend.take().expect("Async build should fill back the results before commit ever took place");
                        snapshot_reborrow.inner = ElementSnapshotInner::Mainline(Mainline {
                            state: Some(MainlineState::InflateSuspended {
                                suspended_hooks: result.hooks.fire_effects(),
                                waker: todo!(), // Prevent fired async wakes, establish sync wakes
                            }),
                            async_queue: AsyncWorkQueue::new_empty(),
                        });
                        return InflateSuspended;
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
                let (current, backqueue) = mainline.async_queue.current_and_backqueue_mut();

                debug_assert!(
                    !backqueue.is_some_and(|backqueue| backqueue
                        .iter()
                        .any(|entry| finished_lanes.contains(entry.work_context.lane_pos))),
                    "Finished lanes should not show up in backqueue during commit!"
                );

                // We do not occupy this node
                // Because until we remove the current entry, no other async work will start executing
                // Those trying to occupy this node will fail and request the scheduler (which we now hold) to reorder work.
                // After we are finished with the commit, the scheduler will proceed to reorder work.
                let state = mainline.state.as_ref().expect(
                    "Async commit walk should not witness a node occupied by another sync walk",
                );

                // let Some(current) = current else {
                //
                // };
                match current {
                    Some(current) if finished_lanes.contains(current.work_context.lane_pos) => {
                        return VisitChildrenAndCommit {
                            children: state.children_cloned().expect(
                                "Async commit walk should not walk into a \
                                inflate suspended node that it has no work on. \
                                Inflate suspended node has no children \
                                and therefore impossible to have work in its descendants",
                            ),
                        }
                    }
                    _ => {
                        // No work in this node, skip and visit children instead
                        use MainlineState::*;
                        let (children, render_object, self_rebuild_suspended) = match state {
                            Ready {
                                children,
                                render_object,
                                ..
                            } => (children, render_object.clone(), false),
                            RebuildSuspended { children, .. } => {
                                (children, Default::default(), true)
                            }
                            InflateSuspended { .. } => panic!(
                                "Async commit walk should not walk into a \
                                inflate suspended node that it has no work on. \
                                Inflate suspended node has no children \
                                and therefore impossible to have work in its descendants"
                            ),
                        };
                        return SkipAndVisitChildren {
                            children: children.map_ref_collect(Clone::clone),
                            render_object,
                            self_rebuild_suspended,
                        };
                    }
                }
            }
        }
    }

    fn execute_commit_async(
        &self,
        render_object_changes: ContainerOf<
            E::ChildContainer,
            SubtreeRenderObjectChange<E::ChildProtocol>,
        >,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'_>,
        lane_scheduler: &LaneScheduler,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        let mut snapshot = self.snapshot.lock();
        // https://bevy-cheatbook.github.io/pitfalls/split-borrows.html
        let snapshot_reborrow = &mut *snapshot;

        match &mut snapshot_reborrow.inner {
            ElementSnapshotInner::AsyncInflating(AsyncInflating { stash, .. }) => {
                let output = std::mem::replace(&mut stash.output, AsyncOutput::Gone);
                let AsyncOutput::Completed(results) = output else {
                    panic!(
                        "Previously the commit visit determined this node needs commit and is not suspended during inflating. \
                        However, when it visit again it has been in a non-commitable state or suspended state, \
                        indicating a possible state corruption"
                    );
                };

                let BuildResults {
                    hooks,
                    element,
                    mut children,
                    rebuild_state,
                } = results;

                debug_assert!(
                    rebuild_state.is_none(),
                    "Inflate commit should not see rebuild results"
                );

                // Fire the hooks before commit into render object
                let hooks = hooks.fire_effects();

                let (render_object, subtree_change) = <E as Element>::Impl::inflate_success_commit(
                    &element,
                    &snapshot_reborrow.widget,
                    &mut children,
                    render_object_changes,
                    &self.context,
                    lane_scheduler,
                );

                snapshot_reborrow.inner = ElementSnapshotInner::Mainline(Mainline {
                    state: Some(MainlineState::Ready {
                        element,
                        hooks,
                        children,
                        render_object,
                    }),
                    async_queue: AsyncWorkQueue::new_empty(),
                });
                return subtree_change;
            }
            ElementSnapshotInner::Mainline(mainline) => {
                // We do not occupy this node
                // Because until we release this node, no other async work will start executing
                // Those trying to occupy this node will fail and request the scheduler (which we now hold) to reorder work.
                // After we are finished with the commit, the scheduler will proceed to reorder work.
                let current = mainline
                    .async_queue
                    .try_pop_front_if(|current| {
                        finished_lanes.contains(current.work_context.lane_pos)
                    })
                    .expect(
                        "This node should have a work that can be committed. \
                        Previously the visit deteremined there is committable work inside,\
                        But when we returned, we found no committable work,\
                        indicating a state corruption",
                    );

                let AsyncQueueCurrentEntry {
                    widget,
                    work_context: _,
                    stash,
                } = current;

                let AsyncStash {
                    handle: _,
                    subscription_diff,
                    reserved_provider_write,
                    output,
                } = stash;

                let state = (&mut mainline.state).take().expect(
                    "Async commit walk should not witness a node occupied by another sync walk",
                );

                let mut is_new_widget = false;
                if let Some(widget) = widget {
                    snapshot_reborrow.widget = widget;
                    is_new_widget = true;
                }

                self.apply_subscription_registrations(subscription_diff);
                if reserved_provider_write {
                    todo!()
                }

                match output {
                    AsyncOutput::Suspended {
                        suspend: Some(suspend),
                        barrier: None,
                    } => todo!(),
                    AsyncOutput::Completed(mut results) => {
                        use MainlineState::*;
                        match state {
                            Ready {
                                hooks,
                                render_object,
                                ..
                            } => {
                                self.perform_commit_rebuild_success_async(
                                    results,
                                    render_object_changes,
                                    &snapshot_reborrow.widget,
                                    hooks,
                                    render_object,
                                    mainline,
                                    scope,
                                    lane_scheduler,
                                    is_new_widget,
                                );
                            }
                            RebuildSuspended {
                                suspended_hooks,
                                waker,
                                ..
                            } => {
                                waker.set_completed();
                                self.perform_commit_rebuild_success_async(
                                    results,
                                    render_object_changes,
                                    &snapshot_reborrow.widget,
                                    suspended_hooks,
                                    Default::default(),
                                    mainline,
                                    scope,
                                    lane_scheduler,
                                    is_new_widget,
                                );
                            }
                            InflateSuspended {
                                mut suspended_hooks,
                                waker,
                            } => {
                                waker.set_completed();
                                suspended_hooks.merge_with(
                                    results.hooks,
                                    false,
                                    HookContextMode::PollInflate,
                                );

                                let (render_object, change) =
                                    <E as Element>::Impl::inflate_success_commit(
                                        &results.element,
                                        &snapshot_reborrow.widget,
                                        &mut results.children,
                                        render_object_changes,
                                        &self.context,
                                        lane_scheduler,
                                    );
                                mainline.state = Some(MainlineState::Ready {
                                    element: results.element,
                                    hooks: suspended_hooks,
                                    children: results.children,
                                    render_object,
                                });
                                return change;
                            }
                        };
                    }
                    AsyncOutput::Uninitiated { .. }
                    | AsyncOutput::Suspended {
                        barrier: Some(_), ..
                    } => panic!("CommitBarrier should not be encountered during commit"),
                    AsyncOutput::Gone | AsyncOutput::Suspended { suspend: None, .. } => {
                        panic!("Async results are gone before commit")
                    }
                }
            }
        }

        todo!()
    }

    fn perform_commit_rebuild_success_async(
        &self,
        mut results: BuildResults<E>,
        render_object_changes: ContainerOf<
            E::ChildContainer,
            SubtreeRenderObjectChange<E::ChildProtocol>,
        >,
        widget: &E::ArcWidget,
        mut hooks: HooksWithTearDowns,
        mut render_object: <<E as Element>::Impl as ImplElementNode<E>>::OptionArcRenderObject,
        mainline: &mut Mainline<E>,
        scope: &rayon::Scope<'_>,
        lane_scheduler: &LaneScheduler,
        is_new_widget: bool,
    ) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        let rebuild_state = results
            .rebuild_state
            .expect("Rebuild commit should see rebuild results");
        hooks.merge_with(results.hooks, false, HookContextMode::Rebuild);

        // We mimic the order in sync rebuild: first apply hook update, then unmount nodes, then commit into render object
        for node_needing_unmount in rebuild_state.nodes_needing_unmount {
            scope.spawn(|scope| node_needing_unmount.unmount(scope))
        }

        let change = <E as Element>::Impl::rebuild_success_commit(
            &results.element,
            &widget,
            rebuild_state.shuffle,
            &mut results.children,
            &mut render_object,
            render_object_changes,
            &self.context,
            lane_scheduler,
            scope,
            is_new_widget,
        );

        mainline.state = Some(MainlineState::Ready {
            element: results.element,
            hooks,
            children: results.children,
            render_object,
        });

        return change;
    }

    fn apply_subscription_registrations(&self, subscription_diff: SubscriptionDiff) {
        let SubscriptionDiff {
            register,
            reserve,
            remove,
        } = subscription_diff;

        if register.is_empty() && reserve.is_empty() && remove.is_empty() {
            return;
        }

        let weak_element_context = Arc::downgrade(&self.context);

        for providing_element_context in reserve {
            // providing_element_context.register
            let provider = providing_element_context
                .provider
                .as_ref()
                .expect("Recorded providers should exist");
            provider.register_read(weak_element_context.clone());
        }

        for providing_element_context in register {
            let provider = providing_element_context
                .provider
                .as_ref()
                .expect("Recorded providers should exist");
            provider.register_read(weak_element_context.clone());
        }

        for providing_element_context in remove {
            let provider = providing_element_context
                .provider
                .as_ref()
                .expect("Recorded providers should exist");
            provider.unregister_read(&weak_element_context);
        }
    }

    // fn commit_async_inflating(
    //     async_inflating: AsyncInflating<E>,
    //     finished_lanes: LaneMask,
    // ) -> MainlineState<E, HooksWithTearDowns> {
    //     let AsyncInflating {
    //         work_context,
    //         stash,
    //     } = async_inflating;
    //     assert!(
    //         finished_lanes.contains(work_context.lane_pos),
    //         "Async commit should not visit into non-mainline nodes from other lanes"
    //     );

    //     match stash.output {
    //         AsyncOutput::Completed(results) => {
    //             debug_assert!(
    //                 results.rebuild_state.is_none(),
    //                 "Async inflate node should not have a rebuild results"
    //             );
    //             MainlineState::Ready {
    //                 element: results.element,
    //                 hooks: results.hooks.fire_effects(),
    //                 children: results.children,
    //                 render_object: todo!(),
    //             }
    //         }
    //         AsyncOutput::Suspended {
    //             suspend,
    //             barrier: None,
    //         } => MainlineState::InflateSuspended {
    //             suspended_hooks: todo!(),
    //             waker: todo!(),
    //         },
    //         AsyncOutput::Uninitiated { barrier }
    //         | AsyncOutput::Suspended {
    //             barrier: Some(barrier),
    //             ..
    //         } => panic!("Async commit initiated when there is still commit barrier alive"),
    //         AsyncOutput::Gone => todo!(),
    //     }
    // }
}
