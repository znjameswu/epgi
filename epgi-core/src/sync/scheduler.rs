use std::any::Any;

use crate::{
    foundation::{Arc, Asc},
    scheduler::{BatchId, BatchResult, JobBatcher, LanePos, SchedulerHandle},
    tree::{
        ArcAnyElementNode, ArcAnyLayerRenderObject, AweakAnyElementNode, AweakElementContextNode,
    },
};

use super::{CommitBarrier, LaneScheduler};
pub struct TreeScheduler {
    lane_scheduler: LaneScheduler,
    pub(super) root_element: ArcAnyElementNode,
    pub(super) root_render_object: ArcAnyLayerRenderObject,
}

impl TreeScheduler {
    pub fn new(root_element: ArcAnyElementNode, scheduler_handle: &SchedulerHandle) -> Self {
        let root_render_object = root_element
            .render_object()
            .expect("The root render object should be initilialized and attached manually")
            .downcast_arc_any_layer_render_object()
            .expect("Root render object should have a layer");
        scheduler_handle
            .push_layer_render_objects_needing_paint(Arc::downgrade(&root_render_object));
        Self {
            lane_scheduler: LaneScheduler::new(),
            root_element,
            root_render_object,
        }
    }

    pub(crate) fn perform_composite(&self) -> Asc<dyn Any + Send + Sync> {
        self.root_render_object.recomposite_into_cache()
    }

    pub(super) fn get_commit_barrier_for(&self, lane_pos: LanePos) -> Option<CommitBarrier> {
        self.lane_scheduler.get_commit_barrier_for(lane_pos)
    }

    pub(crate) fn apply_batcher_result(&mut self, result: BatchResult) {
        self.lane_scheduler
            .apply_batcher_result(result, &self.root_element);
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

    pub(crate) fn dispatch_sync_batch(&mut self) -> Option<BatchId> {
        let Some(sync_batch) = self.lane_scheduler.sync_batch() else {
            return None;
        };
        rayon::scope(|scope| {
            self.root_element
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
