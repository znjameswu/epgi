mod commit_barrier;
pub use commit_barrier::*;

use hashbrown::HashSet;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    foundation::{Asc, PtrEq},
    scheduler::{BatchConf, BatchId, BatchResult, JobBatcher, LaneMask, LanePos},
    tree::{ArcAnyElementNode, AweakAnyElementNode, AweakElementContextNode},
};

pub struct LaneScheduler {
    sync_lane: Option<SyncLaneData>,
    async_lanes: [Option<AsyncLaneData>; LaneMask::ASYNC_LANE_COUNT],
    queued_batches: Vec<Asc<BatchConf>>,
}

struct SyncLaneData {
    batch: Asc<BatchConf>,
    pub point_rebuilds: HashSet<PtrEq<AweakElementContextNode>>,
}

struct AsyncLaneData {
    batch: Asc<BatchConf>,
    lane_pos: LanePos,
    // top_level_roots:
    // state: LaneState,
    barrier_inner: Asc<CommitBarrierInner>,
    blocked_by: LaneMask,
}

impl AsyncLaneData {
    fn new(lane_pos: LanePos, batch: Asc<BatchConf>) -> Self {
        Self {
            lane_pos,
            batch,
            barrier_inner: Asc::new(CommitBarrierInner::new()),
            blocked_by: LaneMask::new(),
        }
    }
}

impl LaneScheduler {
    pub(crate) fn new() -> Self {
        Self {
            sync_lane: None,
            async_lanes: [(); LaneMask::ASYNC_LANE_COUNT].map(|_| None),
            queued_batches: Default::default(),
        }
    }

    pub(super) fn get_commit_barrier_for(&self, lane_pos: LanePos) -> Option<CommitBarrier> {
        let LanePos::Async(pos) = lane_pos else {
            panic!("Only async lanes have commit barriers");
        };
        let Some(async_lane) = &self.async_lanes[pos as usize] else {
            return None;
        };
        Some(CommitBarrier::from_inner(async_lane.barrier_inner.clone()))
    }

    pub(crate) fn apply_batcher_result(
        &mut self,
        result: BatchResult,
        point_rebuilds: HashSet<PtrEq<AweakElementContextNode>>,
        root_element: &ArcAnyElementNode,
    ) {
        debug_assert!(
            self.sync_lane.is_none(),
            "Batcher should only run after the previous sync batch finishes"
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
                        root_element
                            .clone()
                            .remove_async_work_and_lane_in_subtree(async_lane_data.lane_pos);
                        *async_lane = None;
                    }
                }
            }
        }

        if let Some(sync_batch) = new_sync_batch {
            mark_batch(&sync_batch, LanePos::Sync);
            mark_point_rebuilds(&point_rebuilds);
            self.sync_lane = Some(SyncLaneData {
                batch: sync_batch,
                point_rebuilds,
            });
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
                mark_batch(&new_async_batch, lane_pos);
                *async_lane = Some(AsyncLaneData::new(lane_pos, new_async_batch));
                todo!()
            }
        }
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
        let Some(sync_batch) = self.sync_batch() else {
            return None;
        };
        rayon::scope(|scope| {
            root_element
                .clone()
                .visit_and_work_sync_any(&sync_batch.job_ids, scope, self);
        });
        let batch_id = sync_batch.id;
        self.remove_commited_batch(LanePos::Sync);
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

impl LaneScheduler {
    fn sync_batch(&self) -> Option<&BatchConf> {
        self.sync_lane
            .as_ref()
            .map(|sync_lane| sync_lane.batch.as_ref())
    }

    fn remove_commited_batch(&mut self, lane_pos: LanePos) {
        if lane_pos.is_sync() {
            self.sync_lane = None;
            return;
        }
        todo!()
    }
}

fn mark_batch(batch_conf: &BatchConf, lane_pos: LanePos) {
    if batch_conf.roots.len() <= 100 {
        batch_conf.roots.iter().for_each(|PtrEq(node)| {
            let Some(node) = node.upgrade() else { return };
            if node.is_unmounted() {
                return;
            }
            node.mark_root(lane_pos);
        });
    } else {
        batch_conf.roots.par_iter().for_each(|PtrEq(node)| {
            let Some(node) = node.upgrade() else { return };
            if node.is_unmounted() {
                return;
            }
            node.mark_root(lane_pos);
        })
    }
}

fn mark_point_rebuilds(point_rebuilds: &HashSet<PtrEq<AweakElementContextNode>>) {
    point_rebuilds.iter().for_each(|PtrEq(node)| {
        let Some(node) = node.upgrade() else { return };
        if node.is_unmounted() {
            return;
        }
        node.mark_point_rebuild();
    });
}
