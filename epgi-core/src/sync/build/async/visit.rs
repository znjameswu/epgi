use lane_scheduler::{CommitBarrier, LaneScheduler};

use crate::{
    foundation::{Arc, Asc, Container, ContainerOf},
    scheduler::{get_current_scheduler, LanePos},
    sync::lane_scheduler,
    tree::{ArcChildElementNode, Element, ElementNode, FullElement, WorkContext, WorkHandle},
};

pub trait AnyElementAsyncVisitExt {
    fn visit_and_continue_work_async(
        self: Arc<Self>,
        work_to_continue: &(Asc<WorkContext>, WorkHandle, CommitBarrier),
        executable_lanes: &[LanePos],
        lane_scheduler: &LaneScheduler,
    );

    fn visit_and_work_async(
        self: Arc<Self>,
        executable_lanes: &[LanePos],
        lane_scheduler: &LaneScheduler,
    );
}

impl<E> AnyElementAsyncVisitExt for ElementNode<E>
where
    E: FullElement,
{
    fn visit_and_continue_work_async(
        self: Arc<Self>,
        work_to_continue: &(Asc<WorkContext>, WorkHandle, CommitBarrier),
        executable_lanes: &[LanePos],
        lane_scheduler: &LaneScheduler,
    ) {
        ElementNode::visit_and_work_async(
            self,
            Some(work_to_continue),
            executable_lanes,
            lane_scheduler,
        )
    }

    fn visit_and_work_async(
        self: Arc<Self>,
        executable_lanes: &[LanePos],
        lane_scheduler: &LaneScheduler,
    ) {
        ElementNode::visit_and_work_async(self, None, executable_lanes, lane_scheduler)
    }
}

impl<E: FullElement> ElementNode<E> {
    #[inline]
    fn visit_and_work_async(
        self: Arc<Self>,
        work_to_continue: Option<&(Asc<WorkContext>, WorkHandle, CommitBarrier)>,
        mut executable_lanes: &[LanePos],
        lane_scheduler: &LaneScheduler,
    ) {
        let self_lanes = self.context.mailbox_lanes() | self.context.consumer_lanes();
        let descendant_lanes = self.context.descendant_lanes();

        fn get_children<E: Element>(
            node: &ElementNode<E>,
        ) -> Option<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>> {
            let snapshot = node.snapshot.lock();

            let children = snapshot
                .inner
                .mainline_ref()
                .expect("An unmounted element node should not be reachable by async tree visit!")
                .state
                .as_ref()
                .expect(
                    "A sync task should not encounter another sync task contending over the same node",
                )
                .children_cloned();
            children
        }

        // If we have a work context, it means we are continue a work in this subtree.
        // And the top priority is to check whether that lane exist
        if let Some(work_to_continue) = work_to_continue {
            let (work_context, parent_handle, barrier) = work_to_continue;
            let lane_pos = work_context.lane_pos;
            if self_lanes.contains(lane_pos) {
                // Start async work
                return self.spawn_reconcile_node_async(
                    work_context.clone(),
                    parent_handle.clone(),
                    barrier.clone(),
                );
            } else if descendant_lanes.contains(lane_pos) {
                // Visit children
                return if let Some(children) = get_children(&self) {
                    children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                        child.visit_and_continue_work_async(
                            work_to_continue,
                            executable_lanes,
                            lane_scheduler,
                        )
                    })
                };
            }
        }

        // We have no work to continue, then we search for the lane with highest priority
        while let Some((&lane_pos, rest_executable_lanes)) = executable_lanes.split_first() {
            if self_lanes.contains(lane_pos) {
                // Start async work
                let work_context = Asc::new(WorkContext {
                    lane_pos,
                    batch: lane_scheduler
                        .get_batch_conf_for_async(lane_pos)
                        .expect("async lane should exist")
                        .clone(),
                    recorded_provider_values: Default::default(),
                });
                let parent_handle = WorkHandle::new();
                let barrier = lane_scheduler
                    .get_commit_barrier_for(lane_pos)
                    .expect("async lane should exist");
                return self.spawn_reconcile_node_async(work_context, parent_handle, barrier);
            } else if descendant_lanes.contains(lane_pos) {
                // Visit children
                return if let Some(children) = get_children(&self) {
                    children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                        child.visit_and_work_async(executable_lanes, lane_scheduler)
                    })
                };
            } else {
                executable_lanes = rest_executable_lanes
            }
        }
    }

    // #[inline]
    // fn visit_and_work_children_async(
    //     self: Arc<Self>,
    //     inherited_work_context: Option<&Asc<WorkContext>>,
    //     executable_lanes: &[LanePos],
    //     lane_scheduler: &LaneScheduler,
    // ) {
    //     let snapshot = self.snapshot.lock();

    //     let children = snapshot
    //         .inner
    //         .mainline_ref()
    //         .expect("An unmounted element node should not be reachable by async tree visit!")
    //         .state
    //         .as_ref()
    //         .expect(
    //             "A sync task should not encounter another sync task contending over the same node",
    //         )
    //         .children_cloned();

    //     drop(snapshot);

    //     if let Some(children) = children {
    //         if let Some(inherited_work_context) = inherited_work_context {
    //             children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
    //                 child.visit_and_continue_work_async(
    //                     inherited_work_context,
    //                     executable_lanes,
    //                     lane_scheduler,
    //                 )
    //             })
    //         } else {
    //             children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
    //                 child.visit_and_work_async(executable_lanes, lane_scheduler)
    //             })
    //         }
    //     }
    // }
}
