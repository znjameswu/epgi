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
        self.visit_and_continue_work_async_impl(work_to_continue)
    }

    fn visit_and_start_work_async(
        self: Arc<Self>,
        lanes_to_start: &[LanePos],
        lane_scheduler: &LaneScheduler,
    ) {
        self.visit_and_start_work_async_impl(lanes_to_start, lane_scheduler)
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

        let mut unblocked_works = Vec::new();
        let mut remaining_lanes_to_start = lanes_to_start;

        let init_work_for_lane_pos = |lane_pos: LanePos| {
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
            (work_context, parent_handle, barrier)
        };
        
        while let Some((&lane_pos, rest_lanes)) = remaining_lanes_to_start.split_first() {
            if !self_lanes.contains(lane_pos) {
                break;
            }
            remaining_lanes_to_start = rest_lanes;
            unblocked_works.push(init_work_for_lane_pos(lane_pos));
        }

        if !unblocked_works.is_empty() {
            self.clone()
                .spawn_multi_reconcile_node_async(unblocked_works);
        }

        let mut blocked_works = Vec::new();
        for &lane_pos in remaining_lanes_to_start {
            if !self_lanes.contains(lane_pos) {
                break;
            }
            blocked_works.push(init_work_for_lane_pos(lane_pos));
        }

        let mut remaining_lanes_to_start_for_descendant = Cow::Borrowed(remaining_lanes_to_start);
        if !blocked_works.is_empty() {
            // We need to prevent our descendants from starting those lanes that we already have.
            // We have already removed the lanes of the unblocked works.
            // Now we need to remove the lanes of the blocked works.
            // Instead of checking inside the vec, we check inside our lane mask. It is an overkill but faster.
            remaining_lanes_to_start_for_descendant
                .to_mut()
                .retain(|&lane_pos| !self_lanes.contains(lane_pos));
        }

        if !remaining_lanes_to_start_for_descendant.is_empty() {
            if let Some(children) = self.get_children() {
                children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                    child.visit_and_start_work_async(
                        &remaining_lanes_to_start_for_descendant,
                        lane_scheduler,
                    )
                })
            };
        }

        if !blocked_works.is_empty() {
            self.clone().spawn_multi_reconcile_node_async(blocked_works);
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
