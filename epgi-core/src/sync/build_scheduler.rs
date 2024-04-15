mod commit_barrier;
mod lane_scheduler;

pub use commit_barrier::*;
use lane_scheduler::*;

use crate::{
    foundation::Arc,
    scheduler::{BatchId, BatchResult, JobBatcher, LanePos, SchedulerHandle},
    tree::{
        ArcAnyElementNode, ArcAnyLayerRenderObject, AweakAnyElementNode, AweakElementContextNode,
    },
};

pub struct BuildScheduler {
    lane_scheduler: LaneScheduler,
}

impl BuildScheduler {
    pub fn new() -> Self {
        Self {
            lane_scheduler: LaneScheduler::new(),
        }
    }

    pub(super) fn get_commit_barrier_for(&self, lane_pos: LanePos) -> Option<CommitBarrier> {
        self.lane_scheduler.get_commit_barrier_for(lane_pos)
    }

    pub(crate) fn apply_batcher_result(
        &mut self,
        result: BatchResult,
        root_element: &ArcAnyElementNode,
    ) {
        self.lane_scheduler
            .apply_batcher_result(result, root_element);
    }

    pub(crate) fn commit_completed_async_batches(
        &mut self,
        job_batcher: &mut JobBatcher,
    ) -> Vec<BatchId> {
        todo!()
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

    pub(crate) fn dispatch_sync_batch(
        &mut self,
        root_element: &ArcAnyElementNode,
    ) -> Option<BatchId> {
        let Some(sync_batch) = self.lane_scheduler.sync_batch() else {
            return None;
        };
        rayon::scope(|scope| {
            root_element
                .clone()
                .visit_and_work_sync_any(&sync_batch.job_ids, scope, self);
        });
        let batch_id = sync_batch.id;
        self.lane_scheduler.remove_commited_batch(LanePos::Sync);
        return Some(batch_id);
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
