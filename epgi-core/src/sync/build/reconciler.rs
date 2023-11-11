use crate::{
    foundation::{HktContainer, Inlinable64Vec, InlinableDwsizeVec, Parallel, Protocol},
    scheduler::JobId,
    sync::{SubtreeRenderObjectCommitResult, TreeScheduler},
    tree::{
        ArcChildElementNode, ArcElementContextNode, BuildContext, Element, ElementWidgetPair,
        HookContext, ReconcileItem, Reconciler,
    },
};

pub(super) struct SyncReconciler<'a, 'batch, CP: Protocol> {
    pub(super) job_ids: &'a Inlinable64Vec<JobId>,
    pub(super) scope: &'a rayon::Scope<'batch>,
    pub(super) tree_scheduler: &'batch TreeScheduler,
    // pub(super) subtree_results: &'a mut SubtreeVisitResult,
    pub(super) host_context: &'a ArcElementContextNode, // Remove duplicate field with build_context
    pub(super) hooks: &'a mut HookContext,
    pub(super) nodes_needing_unmount: &'a mut InlinableDwsizeVec<ArcChildElementNode<CP>>,
}

impl<'a, 'batch, CP: Protocol> Reconciler<CP> for SyncReconciler<'a, 'batch, CP> {
    fn build_context(&mut self) -> BuildContext<'_> {
        BuildContext {
            hooks: self.hooks,
            element_context: self.host_context,
        }
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
        foundation::Arc,
        sync::SyncReconcileContext,
        tree::{ArcElementContextNode, ElementNode, Widget},
    };

    use super::*;
    pub trait ChildElementWidgetPairSyncBuildExt<P: Protocol> {
        fn rebuild_sync<'a, 'batch>(
            self,
            reconcile_context: SyncReconcileContext<'a, 'batch>,
        ) -> (ArcChildElementNode<P>, SubtreeRenderObjectCommitResult<P>);

        fn rebuild_sync_box<'a, 'batch>(
            self: Box<Self>,
            reconcile_context: SyncReconcileContext<'a, 'batch>,
        ) -> (ArcChildElementNode<P>, SubtreeRenderObjectCommitResult<P>);
    }

    impl<E> ChildElementWidgetPairSyncBuildExt<E::ParentProtocol> for ElementWidgetPair<E>
    where
        E: Element,
    {
        fn rebuild_sync<'a, 'batch>(
            self,
            reconcile_context: SyncReconcileContext<'a, 'batch>,
        ) -> (
            ArcChildElementNode<E::ParentProtocol>,
            SubtreeRenderObjectCommitResult<E::ParentProtocol>,
        ) {
            let subtree_results = self
                .element
                .rebuild_node_sync(Some(self.widget), reconcile_context);
            (self.element, subtree_results)
        }

        fn rebuild_sync_box<'a, 'batch>(
            self: Box<Self>,
            reconcile_context: SyncReconcileContext<'a, 'batch>,
        ) -> (
            ArcChildElementNode<E::ParentProtocol>,
            SubtreeRenderObjectCommitResult<E::ParentProtocol>,
        ) {
            self.rebuild_sync(reconcile_context)
        }
    }

    pub trait ChildWidgetSyncInflateExt<PP: Protocol> {
        fn inflate_sync<'a, 'batch>(
            self: Arc<Self>,
            parent_context: ArcElementContextNode,
            reconcile_context: SyncReconcileContext<'a, 'batch>,
        ) -> (ArcChildElementNode<PP>, SubtreeRenderObjectCommitResult<PP>);
    }

    impl<T> ChildWidgetSyncInflateExt<<<T as Widget>::Element as Element>::ParentProtocol> for T
    where
        T: Widget,
    {
        fn inflate_sync<'a, 'batch>(
            self: Arc<Self>,
            parent_context: ArcElementContextNode,
            reconcile_context: SyncReconcileContext<'a, 'batch>,
        ) -> (
            ArcChildElementNode<<<T as Widget>::Element as Element>::ParentProtocol>,
            SubtreeRenderObjectCommitResult<<<T as Widget>::Element as Element>::ParentProtocol>,
        ) {
            let (node, results) = ElementNode::<<T as Widget>::Element>::inflate_node_sync(
                &self.into_arc_widget(),
                parent_context,
                reconcile_context,
            );
            (node as _, results)
        }
    }
}
