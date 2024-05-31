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

    pub(crate) fn get_commit_barrier_for(&self, lane_pos: LanePos) -> Option<CommitBarrier> {
        let pos = lane_pos
            .async_lane_pos()
            .expect("Only async lanes have commit barriers");
        let async_lane = self.async_lanes[pos as usize].as_ref()?;
        Some(CommitBarrier::from_inner(async_lane.barrier_inner.clone()))
    }

    pub(crate) fn get_batch_conf_for_async(&self, lane_pos: LanePos) -> Option<&Asc<BatchConf>> {
        let pos = lane_pos.async_lane_pos().expect("Async lane is expected");
        let async_lane = self.async_lanes[pos as usize].as_ref()?;
        Some(&async_lane.batch)
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
                            .remove_async_work(async_lane_data.lane_pos, true);
                        todo!("Remove lane mark");
                        *async_lane = None;
                    }
                }
            }
        }

        if let Some(sync_batch) = new_sync_batch {
            mark_batch(&sync_batch, LanePos::SYNC);
            mark_point_rebuilds(&point_rebuilds);
            self.sync_lane = Some(SyncLaneData {
                batch: sync_batch,
                point_rebuilds,
            });
        }

        if !new_async_batches.is_empty() {
            self.queued_batches.extend(new_async_batches);
            // The top priority batch is sorted to the rear
            self.queued_batches
                .sort_unstable_by_key(|batch| std::cmp::Reverse(batch.priority));
        }
    }

    pub(crate) fn commit_completed_async_batches(
        &mut self,
        root_element: &ArcAnyElementNode,
        job_batcher: &mut JobBatcher,
    ) {
        let mut finished_lanes = LaneMask::new();
        for (lane_index, async_lane) in self.async_lanes.iter_mut().enumerate() {
            let Some(async_lane) = async_lane else {
                continue;
            };
            if async_lane.barrier_inner.is_empty() {
                finished_lanes = finished_lanes | LanePos::new_async(lane_index as u8);
                job_batcher.remove_commited_batch(&async_lane.batch.id);
            }
        }
        if !finished_lanes.is_empty() {
            let root_element = root_element.clone();
            rayon::scope(move |scope| {
                root_element.visit_and_commit_async_any(finished_lanes, scope, self);
            })
        }
    }

    pub(crate) fn dispatch_sync_batch(
        &mut self,
        root_element: &ArcAnyElementNode,
    ) -> Option<BatchId> {
        let Some(sync_batch) = self.sync_batch() else {
            return None;
        };
        let root_element = root_element.clone();
        rayon::scope(|scope| {
            root_element.visit_and_work_sync_any(&sync_batch.job_ids, scope, self);
        });
        let batch_id = sync_batch.id;
        self.remove_commited_batch(LanePos::SYNC);
        return Some(batch_id);
    }

    pub(crate) fn dispatch_async_batches(&mut self, root_element: &ArcAnyElementNode) {
        let mut lanes_to_start = Vec::new();
        if !self.queued_batches.is_empty() {
            for (lane_index, async_lane) in self.async_lanes.iter_mut().enumerate() {
                if async_lane.is_some() {
                    continue;
                }
                // The top priority batch is sorted to the rear, so the executable_lanes is sorted
                let Some(new_async_batch) = self.queued_batches.pop() else {
                    break;
                };
                let lane_pos = LanePos::new_async(lane_index as u8);
                mark_batch(&new_async_batch, lane_pos);
                *async_lane = Some(AsyncLaneData::new(lane_pos, new_async_batch));
                lanes_to_start.push(lane_pos)
            }
        }

        // In theory, instead of visiting top-down, we can also filter out top-level roots during lane marking,
        // and directly start working on individual top-level roots.
        // It would be faster, but this would rely on a weak pointer from ElementContextNode back to ElementNode
        // (Lane marking needs parent pointers, which are provided by ElementContextNode)
        // We DO have this weak pointer, but its implications on our parallel algorithm remains unclear.
        // (For example, mountedness and lifecycle concerns)
        // We choose the more traditional visit and start work
        if !lanes_to_start.is_empty() {
            root_element
                .clone()
                .visit_and_start_work_async(lanes_to_start.as_slice(), self)
        }
    }

    pub(crate) fn reorder_async_work(&self, node: AweakAnyElementNode) {
        node.upgrade().map(|node| node.reorder_async_work(self));
    }

    pub(crate) fn reorder_provider_reservation(&self, context: AweakElementContextNode) {
        let Some(context) = context.upgrade() else {
            return;
        };
        context.reorder_reservation(self)
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
    let mark_root = |PtrEq(node): &PtrEq<AweakElementContextNode>| {
        let Some(node) = node.upgrade() else { return };
        if let Err(not_unmounted) = node.is_unmounted() {
            node.mark_root(lane_pos, not_unmounted);
        }
        return;
    };

    if batch_conf.roots.len() <= 100 {
        batch_conf.roots.iter().for_each(mark_root);
    } else {
        batch_conf.roots.par_iter().for_each(mark_root)
    }
}

fn mark_point_rebuilds(point_rebuilds: &HashSet<PtrEq<AweakElementContextNode>>) {
    point_rebuilds.iter().for_each(|PtrEq(node)| {
        let Some(node) = node.upgrade() else { return };
        if let Err(not_unmounted) = node.is_unmounted() {
            node.mark_point_rebuild(not_unmounted);
        }
        return;
    });
}
