use crate::{
    foundation::{Arc, Inlinable64Vec, Protocol},
    scheduler::JobId,
    sync::LaneScheduler,
    tree::{ArcAnyElementNode, ArcChildElementNode, ElementNode, FullElement},
};

use super::SubtreeRenderObjectChange;

pub trait AnyElementSyncReconcileExt {
    fn visit_and_work_sync_any(
        self: Arc<Self>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        lane_scheduler: &LaneScheduler,
    ) -> ArcAnyElementNode;
}

impl<E: FullElement> AnyElementSyncReconcileExt for ElementNode<E> {
    fn visit_and_work_sync_any(
        self: Arc<Self>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        lane_scheduler: &LaneScheduler,
    ) -> ArcAnyElementNode {
        self.reconcile_node_sync(None, job_ids, scope, lane_scheduler);
        self
    }
}

pub trait ChildElementSyncReconcileExt<PP: Protocol> {
    fn visit_and_work_sync(
        self: Arc<Self>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        lane_scheduler: &LaneScheduler,
    ) -> (ArcChildElementNode<PP>, SubtreeRenderObjectChange<PP>);
}

impl<E: FullElement> ChildElementSyncReconcileExt<E::ParentProtocol> for ElementNode<E> {
    fn visit_and_work_sync(
        self: Arc<Self>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        lane_scheduler: &LaneScheduler,
    ) -> (
        ArcChildElementNode<E::ParentProtocol>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        let result = self.reconcile_node_sync(None, job_ids, scope, lane_scheduler);
        (self, result)
    }
}
