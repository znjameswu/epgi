use core::sync::atomic::{AtomicBool, Ordering::*};

use crate::{
    scheduler::{AtomicLaneMask, LaneMask, LanePos},
    tree::ElementContextNode,
};

use super::NotUnmountedToken;

pub(crate) struct ElementMark {
    /// Lanes that are present in the mailbox
    pub(super) mailbox_lanes: AtomicLaneMask,

    pub(super) consumer_lanes: AtomicLaneMask,

    // pub(super) async_consumer_deactivated: AtomicBool,

    // pub(super) async_consumer_refcount: SyncMutex<SmallMap<u8, u8>>,
    pub(super) descendant_lanes: AtomicLaneMask,
    /// Indicate whether this node requested for a sync poll
    pub(super) needs_poll: AtomicBool,
}

impl ElementMark {
    pub(crate) fn new() -> Self {
        Self {
            mailbox_lanes: AtomicLaneMask::new(LaneMask::new()),
            consumer_lanes: AtomicLaneMask::new(LaneMask::new()),
            // async_consumer_deactivated: AtomicBool::new(false),
            // async_consumer_refcount: Default::default(),
            descendant_lanes: AtomicLaneMask::new(LaneMask::new()),
            needs_poll: AtomicBool::new(false),
        }
    }
}

pub struct ConsumerWorkSpawnToken(());

impl ElementContextNode {
    pub(crate) fn mailbox_lanes(&self) -> LaneMask {
        self.mark.mailbox_lanes.load(Relaxed)
    }

    pub(crate) fn consumer_lanes(&self) -> LaneMask {
        self.mark.consumer_lanes.load(Relaxed)
    }

    pub(crate) fn descendant_lanes(&self) -> LaneMask {
        self.mark.descendant_lanes.load(Relaxed)
    }

    pub(crate) fn needs_poll(&self) -> bool {
        self.mark.needs_poll.load(Relaxed)
    }

    pub(crate) fn mark_root(&self, lane_pos: LanePos, not_unmounted: NotUnmountedToken) {
        self.mark
            .mailbox_lanes
            .fetch_insert_single(lane_pos, Relaxed);
        self.mark_up(lane_pos, not_unmounted)
    }

    pub(crate) fn mark_point_rebuild(&self, not_unmounted: NotUnmountedToken) {
        self.mark.needs_poll.store(true, Relaxed);
        self.mark_up(LanePos::SYNC, not_unmounted)
    }

    pub(crate) fn mark_consumer(
        &self,
        lane_pos: LanePos,
        not_unmounted: NotUnmountedToken,
    ) -> Option<ConsumerWorkSpawnToken> {
        let old_consumer_lanes = self
            .mark
            .consumer_lanes
            .fetch_insert_single(lane_pos, Relaxed);
        if old_consumer_lanes.contains(lane_pos) {
            return None;
        }
        self.mark_up(lane_pos, not_unmounted);
        return Some(ConsumerWorkSpawnToken(()));
    }

    // Token is to prevent accidental unmark from not the topmost spawner, see reverse-order unsubscription documentation
    pub(crate) fn unmark_consumer(&self, lane_pos: LanePos, spawn_token: ConsumerWorkSpawnToken) {
        debug_assert!(!lane_pos.is_sync(), "Sync lane should not be unmarked");
        drop(spawn_token);
        self.mark
            .consumer_lanes
            .fetch_remove_single(lane_pos, Relaxed);
    }

    // pub(crate) fn mark_consumer_async(
    //     &self,
    //     lane_pos: LanePos,
    //     not_unmounted: NotUnmountedToken,
    // ) -> bool {
    //     let Some(pos) = lane_pos.async_lane_pos() else {
    //         panic!("Expected an async lane but received a sync lane")
    //     };
    //     let mut async_consumer_refcount = self.mark.async_consumer_refcount.lock();
    //     let old_consumer_lanes = self
    //         .mark
    //         .consumer_lanes
    //         .fetch_insert_single(lane_pos, Relaxed);
    //     async_consumer_refcount
    //         .entry(pos)
    //         .or_insert(0)
    //         .add_assign(1);
    //     if old_consumer_lanes.contains(lane_pos) {
    //         return false;
    //     }
    //     // We must hold the refcount lock while mark up
    //     // Example for what may go wrong if we dont:
    //     // The mark_up happens simultaneously with a commit upwalk phase calling update_consumer_lanes
    //     // Then
    //     self.mark_up(lane_pos, not_unmounted);
    //     return true;
    // }

    fn mark_up(&self, lane_pos: LanePos, not_unmounted: NotUnmountedToken) {
        let mut curr = self;
        loop {
            let Some(parent) = curr.parent(not_unmounted) else {
                break;
            };
            curr = parent.as_ref();
            let old_descendant_lanes = curr
                .mark
                .descendant_lanes
                .fetch_insert_single(lane_pos, Relaxed);
            if old_descendant_lanes.contains(lane_pos) {
                break;
            }
        }
    }

    // pub(crate) fn dec_async_consumer_root(&self, pos: u8) {
    //     debug_assert_sync_phase();
    //     {
    //         let mut lock = self.mark.async_consumer_refcount.lock();
    //         let linear_map::Entry::Occupied(mut entry) = lock.entry(pos) else {
    //             panic!("Tried to decrement ref count on non-existing ref count.")
    //         };
    //         let ref_count = entry.get_mut();
    //         ref_count.sub_assign(1);
    //         if *ref_count == 0 {
    //             self.mark.async_consumer_deactivated.store(true, Relaxed);
    //             entry.remove();
    //         }
    //     }
    //     self.mark.async_consumer_deactivated.store(true, Relaxed)
    // }

    // pub(crate) fn update_consumer_lanes(&self) -> SubtreeLanesCommitResult {
    //     debug_assert_sync_phase();
    //     debug_assert!(
    //         !self.consumer_lanes().contains(LanePos::SYNC),
    //         "Sync batch must have already been completed \
    //         before this method is allowed to be called"
    //     );

    //     // Safety of Relaxed load outside a mutex:
    //     // There are only two methods that will modify async_consumer_deactivated: this method and the dec_async_consumer_count
    //     // And they can never be called simultaneously if used correctly.
    //     // And their calling must have already been synchronized by sync phase operations.
    //     let needing_remove = self.mark.async_consumer_deactivated.load(Relaxed);
    //     if needing_remove {
    //         let lock = self.mark.async_consumer_refcount.lock();
    //         let mut new_consumer_lanes = LaneMask::new();
    //         for (&pos, &count) in lock.iter() {
    //             debug_assert!(count != 0);
    //             new_consumer_lanes = new_consumer_lanes | LanePos::new_async(pos)
    //         }
    //         self.mark.consumer_lanes.store(new_consumer_lanes, Relaxed);
    //         SubtreeLanesCommitResult {
    //             remove_has_happened: true,
    //             current: new_consumer_lanes,
    //         }
    //     } else {
    //         SubtreeLanesCommitResult {
    //             remove_has_happened: false,
    //             current: self.consumer_lanes(),
    //         }
    //     }
    // }

    pub(crate) fn purge_lane(&self, lane_pos: LanePos) {
        self.mark
            .mailbox_lanes
            .fetch_remove_single(lane_pos, Relaxed);
        self.mark
            .consumer_lanes
            .fetch_remove_single(lane_pos, Relaxed);
        self.mark
            .descendant_lanes
            .fetch_remove_single(lane_pos, Relaxed);
        if lane_pos.is_sync() {
            self.mark.needs_poll.store(false, Relaxed)
        }
    }
}
