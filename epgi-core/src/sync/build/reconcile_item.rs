use crate::{
    foundation::{Arc, Inlinable64Vec, Protocol},
    scheduler::{BuildScheduler, JobId},
    sync::SubtreeRenderObjectChange,
    tree::{
        ArcChildElementNode, ArcElementContextNode, ElementBase, ElementNode, ElementWidgetPair,
        FullElement, Widget,
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
    E: FullElement,
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

impl<T> ChildWidgetSyncInflateExt<<<T as Widget>::Element as ElementBase>::ParentProtocol> for T
where
    T: Widget,
{
    fn inflate_sync(
        self: Arc<Self>,
        parent_context: ArcElementContextNode,
        build_scheduler: &BuildScheduler,
    ) -> (
        ArcChildElementNode<<<T as Widget>::Element as ElementBase>::ParentProtocol>,
        SubtreeRenderObjectChange<<<T as Widget>::Element as ElementBase>::ParentProtocol>,
    ) {
        let (node, results) = ElementNode::<T::Element>::inflate_node_sync(
            &self.into_arc_widget(),
            parent_context,
            build_scheduler,
        );
        (node as _, results)
    }
}
