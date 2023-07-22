use crate::{
    common::{
        ArcChildElementNode, ArcElementContextNode, BuildContext, Element, ElementWidgetPair,
        ReconcileItem, Reconciler2,
    },
    foundation::{HktContainer, Parallel, Protocol, SmallSet},
    scheduler::JobId,
    sync::{SubtreeCommitResult, TreeScheduler},
};

struct SyncReconciler<'a, 'batch> {
    job_ids: &'a SmallSet<JobId>,
    scope: &'a rayon::Scope<'batch>,
    tree_scheduler: &'batch TreeScheduler,
    subtree_results: &'a mut SubtreeCommitResult,
    host_context: &'a ArcElementContextNode,
    build_context: &'a mut BuildContext,
}

impl<'a, 'batch> Reconciler2 for SyncReconciler<'a, 'batch> {
    fn build_context_mut(&mut self) -> &mut BuildContext {
        self.build_context
    }

    fn into_reconcile<CP: crate::foundation::Protocol, I: Parallel<Item = ReconcileItem<CP>>>(
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
