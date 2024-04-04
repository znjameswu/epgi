use crate::{
    foundation::{Arc, Inlinable64Vec, Protocol},
    scheduler::{BuildScheduler, JobId},
    sync::build::reconcile::ImplElementNodeSyncReconcile,
    sync::SubtreeRenderObjectChange,
    tree::{
        ArcChildElementNode, ArcElementContextNode, Element, ElementWidgetPair, TreeNode, Widget,
    },
};

pub trait ChildElementWidgetPairSyncBuildExt<P: Protocol> {
    fn rebuild_sync(
        self,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        build_scheduler: &BuildScheduler,
    ) -> (ArcChildElementNode<P>, SubtreeRenderObjectChange<P>);

    fn rebuild_sync_box(
        self: Box<Self>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        build_scheduler: &BuildScheduler,
    ) -> (ArcChildElementNode<P>, SubtreeRenderObjectChange<P>);
}

impl<E> ChildElementWidgetPairSyncBuildExt<E::ParentProtocol> for ElementWidgetPair<E>
where
    E: Element,
{
    fn rebuild_sync(
        self,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        build_scheduler: &BuildScheduler,
    ) -> (
        ArcChildElementNode<E::ParentProtocol>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        let subtree_results =
            self.element
                .rebuild_node_sync(Some(self.widget), job_ids, scope, build_scheduler);
        (self.element, subtree_results)
    }

    fn rebuild_sync_box(
        self: Box<Self>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        build_scheduler: &BuildScheduler,
    ) -> (
        ArcChildElementNode<E::ParentProtocol>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        self.rebuild_sync(job_ids, scope, build_scheduler)
    }
}

pub trait ChildWidgetSyncInflateExt<PP: Protocol> {
    fn inflate_sync(
        self: Arc<Self>,
        parent_context: ArcElementContextNode,
        build_scheduler: &BuildScheduler,
    ) -> (ArcChildElementNode<PP>, SubtreeRenderObjectChange<PP>);
}

impl<T> ChildWidgetSyncInflateExt<<<T as Widget>::Element as TreeNode>::ParentProtocol> for T
where
    T: Widget,
{
    fn inflate_sync(
        self: Arc<Self>,
        parent_context: ArcElementContextNode,
        build_scheduler: &BuildScheduler,
    ) -> (
        ArcChildElementNode<<<T as Widget>::Element as TreeNode>::ParentProtocol>,
        SubtreeRenderObjectChange<<<T as Widget>::Element as TreeNode>::ParentProtocol>,
    ) {
        let (node, results) = <<T as Widget>::Element as Element>::ElementNode::inflate_node_sync(
            &self.into_arc_widget(),
            parent_context,
            build_scheduler,
        );
        (node as _, results)
    }
}
