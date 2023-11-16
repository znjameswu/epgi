use crate::{
    foundation::{Arc, Inlinable64Vec, Protocol},
    scheduler::{JobId, TreeScheduler},
    sync::SubtreeRenderObjectChange,
    tree::{
        ArcChildElementNode, ArcElementContextNode, Element, ElementNode, ElementWidgetPair, Widget,
    },
};

pub trait ChildElementWidgetPairSyncBuildExt<P: Protocol> {
    fn rebuild_sync(
        self,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        tree_scheduler: &TreeScheduler,
    ) -> (ArcChildElementNode<P>, SubtreeRenderObjectChange<P>);

    fn rebuild_sync_box(
        self: Box<Self>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        tree_scheduler: &TreeScheduler,
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
        tree_scheduler: &TreeScheduler,
    ) -> (
        ArcChildElementNode<E::ParentProtocol>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        let subtree_results =
            self.element
                .rebuild_node_sync(Some(self.widget), job_ids, scope, tree_scheduler);
        (self.element, subtree_results)
    }

    fn rebuild_sync_box(
        self: Box<Self>,
        job_ids: &Inlinable64Vec<JobId>,
        scope: &rayon::Scope<'_>,
        tree_scheduler: &TreeScheduler,
    ) -> (
        ArcChildElementNode<E::ParentProtocol>,
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        self.rebuild_sync(job_ids, scope, tree_scheduler)
    }
}

pub trait ChildWidgetSyncInflateExt<PP: Protocol> {
    fn inflate_sync(
        self: Arc<Self>,
        parent_context: ArcElementContextNode,
        tree_scheduler: &TreeScheduler,
    ) -> (ArcChildElementNode<PP>, SubtreeRenderObjectChange<PP>);
}

impl<T> ChildWidgetSyncInflateExt<<<T as Widget>::Element as Element>::ParentProtocol> for T
where
    T: Widget,
{
    fn inflate_sync(
        self: Arc<Self>,
        parent_context: ArcElementContextNode,
        tree_scheduler: &TreeScheduler,
    ) -> (
        ArcChildElementNode<<<T as Widget>::Element as Element>::ParentProtocol>,
        SubtreeRenderObjectChange<<<T as Widget>::Element as Element>::ParentProtocol>,
    ) {
        let (node, results) = ElementNode::<<T as Widget>::Element>::inflate_node_sync(
            &self.into_arc_widget(),
            parent_context,
            tree_scheduler,
        );
        (node as _, results)
    }
}
