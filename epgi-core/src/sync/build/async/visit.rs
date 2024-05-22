use lane_scheduler::LaneScheduler;

use crate::{
    foundation::{Arc, Asc, Container, Protocol},
    scheduler::{get_current_scheduler, LanePos},
    sync::lane_scheduler,
    tree::{ElementNode, FullElement, WorkContext},
};

pub trait ChildElementAsyncVisitExt<P: Protocol> {
    fn visit_and_work_async(
        self: Arc<Self>,
        inherited_work_context: Option<&Asc<WorkContext>>,
        executable_lanes: &[LanePos],
        lane_scheduler: &LaneScheduler,
    );
}

impl<E> ChildElementAsyncVisitExt<E::ParentProtocol> for ElementNode<E>
where
    E: FullElement,
{
    fn visit_and_work_async(
        self: Arc<Self>,
        inherited_work_context: Option<&Asc<WorkContext>>,
        executable_lanes: &[LanePos],
        lane_scheduler: &LaneScheduler,
    ) {
        ElementNode::visit_and_work_async(
            self,
            inherited_work_context,
            executable_lanes,
            lane_scheduler,
        )
    }
}

impl<E: FullElement> ElementNode<E> {
    fn visit_and_work_async(
        self: Arc<Self>,
        mut inherited_work_context: Option<&Asc<WorkContext>>,
        mut executable_lanes: &[LanePos],
        lane_scheduler: &LaneScheduler,
    ) {
        let self_lanes = self.context.mailbox_lanes() | self.context.consumer_lanes();
        let descendant_lanes = self.context.descendant_lanes();

        if let Some(work_context) = inherited_work_context {
            let lane_pos = work_context.lane_pos;
            if self_lanes.contains(lane_pos) {
                // Start async work
                return self.spawn_reconcile_node_async(
                    work_context.clone(),
                    lane_scheduler
                        .get_commit_barrier_for(lane_pos)
                        .expect("async lane should exist"),
                );
            } else if descendant_lanes.contains(lane_pos) {
                // Visit children
                return self.visit_and_work_children_async(
                    inherited_work_context,
                    executable_lanes,
                    lane_scheduler,
                );
            } else {
                inherited_work_context = None;
            }
        }

        while let Some((&lane_pos, rest_executable_lanes)) = executable_lanes.split_first() {
            if self_lanes.contains(lane_pos) {
                // Start async work
                let work_context = Asc::new(WorkContext {
                    lane_pos,
                    batch: lane_scheduler
                        .get_batch_conf_for(lane_pos)
                        .expect("async lane should exist"),
                    recorded_provider_values: Default::default(),
                });
                return self.spawn_reconcile_node_async(
                    work_context,
                    lane_scheduler
                        .get_commit_barrier_for(lane_pos)
                        .expect("async lane should exist"),
                );
            } else if descendant_lanes.contains(lane_pos) {
                // Visit children
                return self.visit_and_work_children_async(
                    inherited_work_context,
                    executable_lanes,
                    lane_scheduler,
                );
            } else {
                executable_lanes = rest_executable_lanes
            }
        }
    }

    fn visit_and_work_children_async(
        self: Arc<Self>,
        inherited_work_context: Option<&Asc<WorkContext>>,
        executable_lanes: &[LanePos],
        lane_scheduler: &LaneScheduler,
    ) {
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

        drop(snapshot);

        if let Some(children) = children {
            children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                child.visit_and_work_async(inherited_work_context, executable_lanes, lane_scheduler)
            })
        }
    }
}
