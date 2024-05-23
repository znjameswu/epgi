use std::borrow::Cow;

use lane_scheduler::{CommitBarrier, LaneScheduler};

use crate::{
    foundation::{Arc, Asc, Container, ContainerOf},
    scheduler::{get_current_scheduler, LanePos},
    sync::lane_scheduler,
    tree::{ArcChildElementNode, ElementNode, FullElement, WorkContext, WorkHandle},
};

pub trait AnyElementAsyncVisitExt {
    fn visit_and_continue_work_async(
        self: Arc<Self>,
        work_to_continue: &(Asc<WorkContext>, WorkHandle, CommitBarrier),
    );

    fn visit_and_start_work_async(
        self: Arc<Self>,
        lanes_to_start: &[LanePos],
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
    ) {
        ElementNode::visit_and_continue_work_async_impl(self, work_to_continue)
    }

    fn visit_and_start_work_async(
        self: Arc<Self>,
        lanes_to_start: &[LanePos],
        lane_scheduler: &LaneScheduler,
    ) {
        ElementNode::visit_and_start_work_async_impl(self, lanes_to_start, lane_scheduler)
    }
}

impl<E: FullElement> ElementNode<E> {
    #[inline]
    fn visit_and_continue_work_async_impl(
        self: Arc<Self>,
        work_to_continue: &(Asc<WorkContext>, WorkHandle, CommitBarrier),
    ) {
        let self_lanes = self.context.mailbox_lanes() | self.context.consumer_lanes();
        let descendant_lanes = self.context.descendant_lanes();

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
            return if let Some(children) = self.get_children() {
                children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                    child.visit_and_continue_work_async(work_to_continue)
                })
            };
        }
    }

    fn visit_and_start_work_async_impl(
        self: Arc<Self>,
        // The lanes yet to be started, from high priority to low priority
        lanes_to_start: &[LanePos],
        lane_scheduler: &LaneScheduler,
    ) {
        // Since we are starting root work, we do not need to look into descendant lanes
        let self_lanes = self.context.mailbox_lanes();
        debug_assert!(
            {
                let descendant_lanes = self.context.consumer_lanes();
                lanes_to_start
                    .iter()
                    .all(|&lane_pos| !descendant_lanes.contains(lane_pos))
            },
            "Lanes to be started cannot have consumer work marked. \
            That would indicate they have already been started"
        );

        let mut works_to_start = Vec::new();
        for &lane_pos in lanes_to_start {
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
                works_to_start.push((work_context, parent_handle, barrier));
            }
        }

        let mut remaining_lanes_to_start = Cow::Borrowed(lanes_to_start);
        if !works_to_start.is_empty() {
            remaining_lanes_to_start.to_mut().retain(|&lane_pos| {
                works_to_start
                    .iter()
                    .any(|(work_context, _, _)| work_context.lane_pos == lane_pos)
            });
            self.clone()
                .spawn_multi_reconcile_node_async(works_to_start);
        }
        if !remaining_lanes_to_start.is_empty() {
            if let Some(children) = self.get_children() {
                children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                    child.visit_and_start_work_async(&remaining_lanes_to_start, lane_scheduler)
                })
            };
        }
    }

    fn get_children(
        &self,
    ) -> Option<ContainerOf<E::ChildContainer, ArcChildElementNode<E::ChildProtocol>>> {
        let snapshot = self.snapshot.lock();

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
