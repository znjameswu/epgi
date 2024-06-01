use crate::{
    foundation::Container,
    scheduler::{get_current_scheduler, LanePos},
    tree::{ElementNode, FullElement},
};

pub trait AnyElementAsyncPurgeLaneMark {
    fn purge_lane_mark_async(&self, lane_pos: LanePos);
}

impl<E> AnyElementAsyncPurgeLaneMark for ElementNode<E>
where
    E: FullElement,
{
    fn purge_lane_mark_async(&self, lane_pos: LanePos) {
        self.purge_lane_mark_async_impl(lane_pos)
    }
}

impl<E: FullElement> ElementNode<E> {
    fn purge_lane_mark_async_impl(&self, lane_pos: LanePos) {
        debug_assert!(
            !lane_pos.is_sync(),
            "Sync lane should not rely on this method to purge lane mark"
        );
        if self.context.descendant_lanes().contains(lane_pos) {
            let snapshot = self.snapshot.lock();
            let mainline = snapshot
                .inner
                .mainline_ref()
                .expect("Remove lane mark only walk in mainline nodes");
            let children = mainline
                .state
                .as_ref()
                .expect(
                    "A sync task should not encounter another sync task contending over the same node",
                )
                .children_cloned();
            drop(snapshot);
            if let Some(children) = children {
                children.par_for_each(&get_current_scheduler().sync_threadpool, |child| {
                    child.purge_lane_mark_async(lane_pos)
                })
            }
        }
        self.context.purge_lane(lane_pos)
    }
}
