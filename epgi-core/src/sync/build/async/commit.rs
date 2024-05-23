use crate::{
    foundation::{Arc, Container, ContainerOf, Protocol},
    scheduler::{get_current_scheduler, LaneMask, LanePos},
    sync::{
        build::provider::AsyncWorkNeedsRestarting, ImplCommitRenderObject, LaneScheduler,
        RenderObjectCommitResult,
    },
    tree::{
        ArcAnyElementNode, ArcChildElementNode, AsyncInflating, AsyncOutput,
        AsyncQueueCurrentEntry, AsyncStash, AsyncWorkQueue, BuildResults, Element, ElementNode,
        ElementSnapshotInner, FullElement, HookContextMode, HooksWithTearDowns, ImplElementNode,
        ImplProvide, Mainline, MainlineState, SubscriptionDiff,
    },
};

pub trait AnyElementAsyncCommitExt {
    fn visit_and_commit_async_any<'batch>(
        self: Arc<Self>,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> ArcAnyElementNode;
}

impl<E: FullElement> AnyElementAsyncCommitExt for ElementNode<E> {
    fn visit_and_commit_async_any<'batch>(
        self: Arc<Self>,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> ArcAnyElementNode {
        self.visit_and_commit_async_impl(finished_lanes, scope, lane_scheduler);
        self
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
        &self,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> RenderObjectCommitResult<E::ParentProtocol> {
        let prepare_result = self.prepare_visit_and_commit_async(finished_lanes);
        use PrepareCommitAsyncResult::*;
        match prepare_result {
            VisitChildrenAnd { children, next } => {
                let render_object_changes = children
                    .par_map_collect(&get_current_scheduler().sync_threadpool, |child| {
                        child.visit_and_commit_async(finished_lanes, scope, lane_scheduler)
                    });
                match next {
                    NextAction::Return {
                        render_object,
                        self_rebuild_suspended,
                    } => <E as Element>::Impl::visit_commit_render_object(
                        &self,
                        render_object,
                        render_object_changes,
                        lane_scheduler,
                        scope,
                        self_rebuild_suspended,
                    ),
                    NextAction::Commit => self.execute_commit_async(
                        render_object_changes,
                        finished_lanes,
                        scope,
                        lane_scheduler,
                    ),
                }
            }
            InflateSuspended => RenderObjectCommitResult::Suspend,
            SkipAndReturn => RenderObjectCommitResult::new_no_update(),
        }
    }
}

enum PrepareCommitAsyncResult<E: Element> {
    VisitChildrenAnd {
        children: ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
        next: NextAction<E>,
    },
    InflateSuspended,
    SkipAndReturn,
}

enum NextAction<E: Element> {
    Return {
        render_object: <<E as Element>::Impl as ImplElementNode<E>>::OptionArcRenderObject,
        // Optimization parameter carried over from sync commit (Because the infrasturcture is build around sync commit)
        self_rebuild_suspended: bool,
    },
    Commit,
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
                        return VisitChildrenAnd {
                            children: results.children.map_ref_collect(Clone::clone),
                            next: NextAction::Commit,
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
                        return VisitChildrenAnd {
                            children: state.children_cloned().expect(
                                "Async commit walk should not walk into a \
                                inflate suspended node that it has no work on. \
                                Inflate suspended node has no children \
                                and therefore impossible to have work in its descendants",
                            ),
                            next: NextAction::Commit,
                        }
                    }
                    _ => {
                        // No work in this node, check descendant
                        let no_descendant_lanes =
                            !self.context.descendant_lanes().overlaps(finished_lanes);
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
                        return VisitChildrenAnd {
                            children: children.map_ref_collect(Clone::clone),
                            next: NextAction::Return {
                                render_object,
                                self_rebuild_suspended,
                            },
                        };
                    }
                }
            }
        }
    }

    fn execute_commit_async<'batch>(
        &self,
        render_object_changes: ContainerOf<
            E::ChildContainer,
            RenderObjectCommitResult<E::ChildProtocol>,
        >,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> RenderObjectCommitResult<E::ParentProtocol> {
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

                let (render_object, subtree_change) =
                    <E as Element>::Impl::inflate_success_commit_render_object(
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
                    .remove_current_if(|current| {
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
                    work_context,
                    stash,
                } = current;

                let AsyncStash {
                    handle: _,
                    subscription_diff,
                    spawned_consumers,
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

                self.apply_subscription_registrations(
                    subscription_diff,
                    work_context.lane_pos,
                    lane_scheduler,
                );
                if <E as Element>::Impl::PROVIDE_ELEMENT {
                    if let Some(_spawned_consumers) = spawned_consumers {
                        let provider = self.context.provider_object.as_ref().expect(
                            "Provider element should have a provider in its element context node",
                        );
                        provider.commit_async_write(work_context.lane_pos, work_context.batch.id);
                    }
                } else {
                    debug_assert!(spawned_consumers.is_none());
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
                                    <E as Element>::Impl::inflate_success_commit_render_object(
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

    fn perform_commit_rebuild_success_async<'batch>(
        &self,
        mut results: BuildResults<E>,
        render_object_changes: ContainerOf<
            E::ChildContainer,
            RenderObjectCommitResult<E::ChildProtocol>,
        >,
        widget: &E::ArcWidget,
        mut hooks: HooksWithTearDowns,
        mut render_object: <<E as Element>::Impl as ImplElementNode<E>>::OptionArcRenderObject,
        mainline: &mut Mainline<E>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
        is_new_widget: bool,
    ) -> RenderObjectCommitResult<E::ParentProtocol> {
        let rebuild_state = results
            .rebuild_state
            .expect("Rebuild commit should see rebuild results");
        hooks.merge_with(results.hooks, false, HookContextMode::Rebuild);

        // We mimic the order in sync rebuild: first apply hook update, then unmount nodes, then commit into render object
        let mut unmounted_consumer_lanes = LaneMask::new();
        for node_needing_unmount in rebuild_state.nodes_needing_unmount {
            unmounted_consumer_lanes =
                unmounted_consumer_lanes | node_needing_unmount.context_ref().consumer_lanes();
            scope.spawn(|scope| node_needing_unmount.unmount(scope, lane_scheduler))
        }

        let change = <E as Element>::Impl::rebuild_success_commit_render_object(
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

    fn apply_subscription_registrations(
        &self,
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

        let weak_element_context = Arc::downgrade(&self.context);

        let mut async_work_needs_restarting = AsyncWorkNeedsRestarting::new();

        for providing_element_context in reserve {
            // providing_element_context.register
            let provider = providing_element_context
                .provider_object
                .as_ref()
                .expect("Recorded providers should exist");
            let contending_writer =
                provider.register_reserved_read(weak_element_context.clone(), lane_pos);
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
