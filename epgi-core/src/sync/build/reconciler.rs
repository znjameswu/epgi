use crate::{
    common::{
        ArcChildElementNode, ArcElementContextNode, BuildContext, Element, ElementWidgetPair,
        ReconcileItem, Reconciler,
    },
    foundation::{HktContainer, InlinableDwsizeVec, Parallel, Protocol, SmallSet},
    scheduler::JobId,
    sync::{SubtreeCommitResult, TreeScheduler},
};

pub(super) struct SyncReconciler<'a, 'batch, CP: Protocol> {
    pub(super) job_ids: &'a SmallSet<JobId>,
    pub(super) scope: &'a rayon::Scope<'batch>,
    pub(super) tree_scheduler: &'batch TreeScheduler,
    pub(super) subtree_results: &'a mut SubtreeCommitResult,
    pub(super) host_context: &'a ArcElementContextNode,
    pub(super) build_context: &'a mut BuildContext,
    pub(super) nodes_needing_unmount: &'a mut InlinableDwsizeVec<ArcChildElementNode<CP>>,
}

impl<'a, 'batch, CP: Protocol> Reconciler<CP> for SyncReconciler<'a, 'batch, CP> {
    fn build_context_mut(&mut self) -> &mut BuildContext {
        self.build_context
    }

    fn nodes_needing_unmount_mut(&mut self) -> &mut InlinableDwsizeVec<ArcChildElementNode<CP>> {
        self.nodes_needing_unmount
    }

    fn into_reconcile<I: Parallel<Item = ReconcileItem<CP>>>(
        self,
        items: I,
    ) -> <I::HktContainer as HktContainer>::Container<ArcChildElementNode<CP>> {
        todo!()
    }
}

pub(crate) mod reconciler_private {
    use crate::{
        common::{ArcElementContextNode, ElementNode, Widget},
        foundation::Arc,
    };

    use super::*;
    pub trait ChildElementWidgetPairSyncBuildExt<P: Protocol> {
        fn rebuild_sync<'a, 'batch>(
            self,
            job_ids: &'a SmallSet<JobId>,
            scope: &'a rayon::Scope<'batch>,
            tree_scheduler: &'batch TreeScheduler,
        ) -> (ArcChildElementNode<P>, SubtreeCommitResult);

        fn rebuild_sync_box<'a, 'batch>(
            self: Box<Self>,
            job_ids: &'a SmallSet<JobId>,
            scope: &'a rayon::Scope<'batch>,
            tree_scheduler: &'batch TreeScheduler,
        ) -> (ArcChildElementNode<P>, SubtreeCommitResult);
    }

    impl<E> ChildElementWidgetPairSyncBuildExt<E::SelfProtocol> for ElementWidgetPair<E>
    where
        E: Element,
    {
        fn rebuild_sync<'a, 'batch>(
            self,
            job_ids: &'a SmallSet<JobId>,
            scope: &'a rayon::Scope<'batch>,
            tree_scheduler: &'batch TreeScheduler,
        ) -> (ArcChildElementNode<E::SelfProtocol>, SubtreeCommitResult) {
            let subtree_results =
                self.element
                    .rebuild_node_sync(Some(self.widget), job_ids, scope, tree_scheduler);
            (self.element, subtree_results)
        }

        fn rebuild_sync_box<'a, 'batch>(
            self: Box<Self>,
            job_ids: &'a SmallSet<JobId>,
            scope: &'a rayon::Scope<'batch>,
            tree_scheduler: &'batch TreeScheduler,
        ) -> (ArcChildElementNode<E::SelfProtocol>, SubtreeCommitResult) {
            self.rebuild_sync(job_ids, scope, tree_scheduler)
        }
    }

    pub trait ChildWidgetSyncInflateExt<SP: Protocol> {
        fn inflate_sync<'a, 'batch>(
            self: Arc<Self>,
            parent_context: &ArcElementContextNode,
            job_ids: &'a SmallSet<JobId>,
            scope: &'a rayon::Scope<'batch>,
            tree_scheduler: &'batch TreeScheduler,
        ) -> (ArcChildElementNode<SP>, SubtreeCommitResult);
    }

    impl<T> ChildWidgetSyncInflateExt<<<T as Widget>::Element as Element>::SelfProtocol> for T
    where
        T: Widget,
    {
        fn inflate_sync<'a, 'batch>(
            self: Arc<Self>,
            parent_context: &ArcElementContextNode,
            job_ids: &'a SmallSet<JobId>,
            scope: &'a rayon::Scope<'batch>,
            tree_scheduler: &'batch TreeScheduler,
        ) -> (
            ArcChildElementNode<<<T as Widget>::Element as Element>::SelfProtocol>,
            SubtreeCommitResult,
        ) {
            let (node, results) = ElementNode::<<T as Widget>::Element>::inflate_node_sync(
                &self.into_arc_widget(),
                parent_context,
                job_ids,
                scope,
                tree_scheduler,
            );
            (node as _, results)
        }
    }
}
