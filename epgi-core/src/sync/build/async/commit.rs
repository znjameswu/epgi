use crate::{
    foundation::{Arc, Container, ContainerOf, Protocol},
    scheduler::{get_current_scheduler, LaneMask},
    sync::{ImplReconcileCommit, LaneScheduler, SubtreeRenderObjectChange},
    tree::{
        ArcAnyElementNode, ArcChildElementNode, AsyncInflating, AsyncOutput, AsyncWorkQueue,
        Element, ElementNode, ElementSnapshotInner, FullElement, HooksWithTearDowns,
        ImplElementNode, Mainline, MainlineState, SubscriptionDiff,
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
    ) -> (ArcChildElementNode<PP>, SubtreeRenderObjectChange<PP>);
}

impl<E: FullElement> ChildElementAsyncCommitExt<E::ParentProtocol> for ElementNode<E> {
    fn visit_and_commit_async(
        self: Arc<Self>,
        finished_lanes: LaneMask,
        scope: &rayon::Scope<'_>,
        lane_scheduler: &LaneScheduler,
    ) -> (
        ArcChildElementNode<E::ParentProtocol>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        let result = self.visit_and_commit_async_impl(finished_lanes, scope, lane_scheduler);
        (self, result)
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
                let results = children
                    .par_map_collect(&get_current_scheduler().async_threadpool, |child| {
                        child.visit_and_commit_async(finished_lanes, scope, lane_scheduler)
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
            VisitChildrenAndCommit { children } => todo!(),
            ReturnWithCommitted { subtree_change } => subtree_change,
            SkipAndReturn => SubtreeRenderObjectChange::new_no_update(),
        }
    }
}

enum PrepareCommitAsyncResult<E: Element> {
    VisitChildrenAndCommit {
        children: ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>,
    },
    ReturnWithCommitted {
        subtree_change: SubtreeRenderObjectChange<E::ParentProtocol>,
    },
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
                return match &mut stash.output {
                    AsyncOutput::Completed(results) => {
                        debug_assert!(
                            results.rebuild_state.is_none(),
                            "Async inflate node should not have a rebuild results"
                        );
                        VisitChildrenAndCommit {
                            children: results.children.map_ref_collect(Clone::clone),
                        }
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
                        ReturnWithCommitted {
                            subtree_change: SubtreeRenderObjectChange::Suspend,
                        }
                    }
                    AsyncOutput::Uninitiated { barrier }
                    | AsyncOutput::Suspended {
                        barrier: Some(barrier),
                        ..
                    } => panic!("Async commit initiated when there is still commit barrier alive"),
                };
            }
            ElementSnapshotInner::Mainline(_) => todo!(),
        }
        let mainline = snapshot_reborrow
            .inner
            .mainline_mut()
            .expect("An unmounted element node should not be reachable by a rebuild!");
    }

    fn apply_subscription_registrations(self: &Arc<Self>, subscription_diff: SubscriptionDiff) {
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

    fn commit_async_inflating(
        async_inflating: AsyncInflating<E>,
        finished_lanes: LaneMask,
    ) -> MainlineState<E, HooksWithTearDowns> {
        let AsyncInflating {
            work_context,
            stash,
        } = async_inflating;
        assert!(
            finished_lanes.contains(work_context.lane_pos),
            "Async commit should not visit into non-mainline nodes from other lanes"
        );

        match stash.output {
            AsyncOutput::Completed(results) => {
                debug_assert!(
                    results.rebuild_state.is_none(),
                    "Async inflate node should not have a rebuild results"
                );
                MainlineState::Ready {
                    element: results.element,
                    hooks: results.hooks.fire_effects(),
                    children: results.children,
                    render_object: todo!(),
                }
            }
            AsyncOutput::Suspended {
                suspend,
                barrier: None,
            } => MainlineState::InflateSuspended {
                suspended_hooks: todo!(),
                waker: todo!(),
            },
            AsyncOutput::Uninitiated { barrier }
            | AsyncOutput::Suspended {
                barrier: Some(barrier),
                ..
            } => panic!("Async commit initiated when there is still commit barrier alive"),
        }
    }

    fn execute_commit_async(&self) -> SubtreeRenderObjectChange<E::ParentProtocol> {
        todo!()
    }
}
