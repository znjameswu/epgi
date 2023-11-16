use crate::{
    foundation::{Arc, Protocol},
    sync::SubtreeRenderObjectChange,
    tree::{
        ArcChildElementNode, ArcElementContextNode, Element, ElementNode, ElementWidgetPair, Widget,
    },
};

use super::SyncReconcileContext;

pub trait ChildElementWidgetPairSyncBuildExt<P: Protocol> {
    fn rebuild_sync<'a, 'batch>(
        self,
        reconcile_context: SyncReconcileContext<'a, 'batch>,
    ) -> (ArcChildElementNode<P>, SubtreeRenderObjectChange<P>);

    fn rebuild_sync_box<'a, 'batch>(
        self: Box<Self>,
        reconcile_context: SyncReconcileContext<'a, 'batch>,
    ) -> (ArcChildElementNode<P>, SubtreeRenderObjectChange<P>);
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
        SubtreeRenderObjectChange<E::ParentProtocol>,
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
        SubtreeRenderObjectChange<E::ParentProtocol>,
    ) {
        self.rebuild_sync(reconcile_context)
    }
}

pub trait ChildWidgetSyncInflateExt<PP: Protocol> {
    fn inflate_sync<'a, 'batch>(
        self: Arc<Self>,
        parent_context: ArcElementContextNode,
    ) -> (ArcChildElementNode<PP>, SubtreeRenderObjectChange<PP>);
}

impl<T> ChildWidgetSyncInflateExt<<<T as Widget>::Element as Element>::ParentProtocol> for T
where
    T: Widget,
{
    fn inflate_sync<'a, 'batch>(
        self: Arc<Self>,
        parent_context: ArcElementContextNode,
    ) -> (
        ArcChildElementNode<<<T as Widget>::Element as Element>::ParentProtocol>,
        SubtreeRenderObjectChange<<<T as Widget>::Element as Element>::ParentProtocol>,
    ) {
        let (node, results) = ElementNode::<<T as Widget>::Element>::inflate_node_sync(
            &self.into_arc_widget(),
            parent_context,
        );
        (node as _, results)
    }
}
