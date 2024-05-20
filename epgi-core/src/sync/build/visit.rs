use crate::{
    foundation::{Arc, Inlinable64Vec, Protocol},
    scheduler::JobId,
    sync::LaneScheduler,
    tree::{ArcAnyElementNode, ArcChildElementNode, ElementNode, FullElement},
};

use super::{CommitResult, RenderObjectCommitResult};

pub trait AnyElementSyncReconcileExt {
    fn visit_and_work_sync_any<'batch>(
        self: Arc<Self>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> ArcAnyElementNode;
}

impl<E: FullElement> AnyElementSyncReconcileExt for ElementNode<E> {
    fn visit_and_work_sync_any<'batch>(
        self: Arc<Self>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> ArcAnyElementNode {
        self.reconcile_node_sync(None, job_ids, scope, lane_scheduler);
        self
    }
}

pub trait ChildElementSyncReconcileExt<PP: Protocol> {
    fn visit_and_work_sync<'batch>(
        self: Arc<Self>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> (ArcChildElementNode<PP>, CommitResult<PP>);
}

impl<E: FullElement> ChildElementSyncReconcileExt<E::ParentProtocol> for ElementNode<E> {
    fn visit_and_work_sync<'batch>(
        self: Arc<Self>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'batch>,
        lane_scheduler: &'batch LaneScheduler,
    ) -> (
        ArcChildElementNode<E::ParentProtocol>,
        CommitResult<E::ParentProtocol>,
    ) {
        let result = self.reconcile_node_sync(None, job_ids, scope, lane_scheduler);
        (self, result)
    }
}
