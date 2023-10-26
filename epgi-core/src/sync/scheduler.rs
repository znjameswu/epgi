use crate::{
    scheduler::{BatchResult, JobBatcher, LanePos},
    tree::{
        ArcAnyElementNode, ArcAnyLayerNode, ArcAnyRenderObject, AweakAnyElementNode,
        AweakElementContextNode,
    },
};

use super::{CommitBarrier, LaneScheduler};
pub struct TreeScheduler {
    lane_scheduler: LaneScheduler,
    pub(super) root_element: ArcAnyElementNode,
    pub(super) root_render_object: ArcAnyRenderObject,
    pub(crate) root_layer: ArcAnyLayerNode,
}

impl TreeScheduler {
    pub fn new(root_element: ArcAnyElementNode) -> Self {
        let root_render_object = root_element.render_object().expect(
            "The render object of the root element should be initilialized and attached manually",
        );
        let root_layer = root_render_object.layer().expect(
            "The layer of the root render object should be initilialized and attached manually",
        );
        Self {
            lane_scheduler: LaneScheduler::new(),
            root_element,
            root_render_object,
            root_layer,
        }
    }

    pub(super) fn get_commit_barrier_for(&self, lane_pos: LanePos) -> Option<CommitBarrier> {
        self.lane_scheduler.get_commit_barrier_for(lane_pos)
    }

    pub(crate) fn apply_batcher_result(&mut self, result: BatchResult) {
        self.lane_scheduler
            .apply_batcher_result(result, &self.root_element);
    }

    pub(crate) fn commit_completed_async_batches(&mut self, job_batcher: &mut JobBatcher) {
        // todo!()
        // for (lane_index, async_lane) in self.async_lanes.iter_mut().enumerate() {
        //     let Some(async_lane) = async_lane else {
        //         continue;
        //     };
        //     if async_lane.barrier_inner.is_empty() {
        //         todo!("Commit async lane");
        //         job_batcher.remove_commited_batch(&async_lane.batch.id);
        //     }
        // }
    }

    pub(crate) fn dispatch_sync_batch(&mut self) {
        if let Some(sync_job_ids) = self.lane_scheduler.get_sync_job_id() {
            rayon::scope(|scope| {
                self.root_element
                    .clone()
                    .visit_and_work_sync(sync_job_ids, scope, self);
            });
        }
    }

    pub(crate) fn dispatch_async_batches(&self) {
        // todo!()
    }

    pub(crate) fn reorder_async_work(&self, node: AweakAnyElementNode) {
        node.upgrade().map(|node| node.reorder_async_work(self));
    }

    pub(crate) fn reorder_provider_reservation(&self, context: AweakElementContextNode) {
        // todo!()
    }
}
