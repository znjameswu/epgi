use crate::{
    common::{
        AweakAnyElementNode, AweakElementContextNode, ElementNode, RenderObject,
        RootViewElement,
    },
    foundation::{Arc, Asc},
    scheduler::{BatchConf, BatchResult, JobBatcher, LaneMask, LanePos}, integrations::RenderRootView,
};

use super::{CommitBarrier, CommitBarrierInner};

struct LaneData {
    batch: Asc<BatchConf>,
    lane_pos: LanePos,
    // top_level_roots:
    // state: LaneState,
    barrier_inner: Asc<CommitBarrierInner>,
    blocked_by: LaneMask,
}

impl LaneData {
    fn new(lane_pos: LanePos, batch: Asc<BatchConf>) -> Self {
        Self {
            lane_pos,
            batch,
            barrier_inner: Asc::new(CommitBarrierInner::new()),
            blocked_by: LaneMask::new(),
        }
    }
}

pub struct TreeScheduler {
    sync_lane: Option<LaneData>,
    async_lanes: [Option<LaneData>; LaneMask::ASYNC_LANE_COUNT],
    queued_batches: Vec<Asc<BatchConf>>,
    root_element: Arc<ElementNode<RootViewElement>>,
    root_render_object: Arc<RenderObject<RenderRootView>>,
}

impl TreeScheduler {
    pub(super) fn get_commit_barrier_for(&self, lane_pos: LanePos) -> Option<CommitBarrier> {
        let LanePos::Async(pos) = lane_pos else {
            panic!("Only async lanes have commit barriers");
        };
        let Some(async_lane) = &self.async_lanes[pos as usize] else {
            return None;
        };
        Some(CommitBarrier::from_inner(async_lane.barrier_inner.clone()))
    }

    pub(crate) fn apply_batcher_result(&mut self, result: BatchResult) {
        debug_assert!(
            self.sync_lane.is_none(),
            "Batcher should only be run after the previous sync batch finishes"
        );
        let BatchResult {
            expired_batches,
            new_async_batches,
            new_sync_batch,
        } = result;
        if !expired_batches.is_empty() {
            self.queued_batches
                .retain(|batch| !expired_batches.contains(&batch.id));
            for async_lane in self.async_lanes.iter_mut() {
                if let Some(async_lane_data) = async_lane {
                    if expired_batches.contains(&async_lane_data.batch.id) {
                        self.root_element
                            .remove_async_work_and_lane_in_subtree(async_lane_data.lane_pos);
                        *async_lane = None;
                    }
                }
            }
        }
        if let Some(sync_batch) = new_sync_batch {
            self.sync_lane = Some(LaneData::new(LanePos::Sync, sync_batch));
            todo!()
        }

        if !new_async_batches.is_empty() {
            self.queued_batches.extend(new_async_batches);
            self.queued_batches
                .sort_unstable_by_key(|batch| std::cmp::Reverse(batch.priority));
        }

        if !self.queued_batches.is_empty() {
            for (lane_index, async_lane) in self.async_lanes.iter_mut().enumerate() {
                if async_lane.is_some() {
                    continue;
                }
                let Some(new_async_batch) = self.queued_batches.pop() else {
                    break;
                };
                let lane_pos = LanePos::Async(lane_index as u8);
                *async_lane = Some(LaneData::new(lane_pos, new_async_batch));
                todo!()
            }
        }
    }

    pub(crate) fn commit_completed_async_batches(&mut self, job_batcher: &mut JobBatcher) {
        for (lane_index, async_lane) in self.async_lanes.iter_mut().enumerate() {
            let Some(async_lane) = async_lane else {
                continue;
            };
            if async_lane.barrier_inner.is_empty() {
                todo!("Commit async lane");
                job_batcher.remove_commited_batch(&async_lane.batch.id);
            }
        }
    }

    pub(crate) fn dispatch_sync_batch(&mut self) {
        todo!()
    }

    pub(crate) fn dispatch_async_batches(&self) {
        todo!()
    }

    pub(crate) fn reorder_async_work(&self, node: AweakAnyElementNode) {
        node.upgrade().map(|node| node.reorder_async_work(self));
    }

    pub(crate) fn reorder_provider_reservation(&self, context: AweakElementContextNode) {
        todo!()
    }
}
