use crate::{
    foundation::{Arc, Protocol, SyncMutex},
    scheduler::JobId,
};

use super::{
    ArcAnyElementNode, ArcAnyRenderObject, ArcChildElementNode, ArcChildRenderObject,
    ArcChildWidget, ArcElementContextNode, Element, ElementContextNode, ElementReconcileItem,
    ElementSnapshot, ElementSnapshotInner, FullElement, ImplElementNode, Mainline, MainlineState,
};

pub struct ElementNode<E: Element> {
    pub context: ArcElementContextNode,
    pub(crate) snapshot: SyncMutex<ElementSnapshot<E>>,
}

impl<E: Element> ElementNode<E> {
    pub(super) fn new(
        context: ArcElementContextNode,
        widget: E::ArcWidget,
        inner: ElementSnapshotInner<E>,
    ) -> Self {
        Self {
            context,
            snapshot: SyncMutex::new(ElementSnapshot {
                widget,
                // subtree_suspended: true,
                inner,
            }),
        }
    }
    pub fn widget(&self) -> E::ArcWidget {
        self.snapshot.lock().widget.clone()
    }
}

pub trait AnyElementNode:
    crate::sync::cancel_private::AnyElementNodeAsyncCancelExt
    + crate::sync::sync_build_private::AnyElementSyncReconcileExt
    + crate::sync::restart_private::AnyElementNodeRestartAsyncExt
    + crate::sync::reorder_work_private::AnyElementNodeReorderAsyncWorkExt
    + crate::sync::unmount::AnyElementNodeUnmountExt
    + Send
    + Sync
    + 'static
{
    fn as_any_arc(self: Arc<Self>) -> ArcAnyElementNode;
    fn push_job(&self, job_id: JobId);
    fn render_object(&self) -> Result<ArcAnyRenderObject, &str>;
    // fn context(&self) -> &ArcElementContextNode;
}

pub trait ChildElementNode<PP: Protocol>:
    AnyElementNode
    + crate::sync::sync_build_private::ChildElementSyncReconcileExt<PP>
    + Send
    + Sync
    + 'static
{
    fn context(&self) -> &ElementContextNode;

    fn as_arc_any(self: Arc<Self>) -> ArcAnyElementNode;

    // Due to the limitation of both arbitrary_self_type and downcasting, we have to consume both Arc pointers
    // Which may not be a bad thing after all, considering how a fat &Arc would look like in memory layout.
    fn can_rebuild_with(
        self: Arc<Self>,
        widget: ArcChildWidget<PP>,
    ) -> Result<ElementReconcileItem<PP>, (ArcChildElementNode<PP>, ArcChildWidget<PP>)>;

    fn get_current_subtree_render_object(&self) -> Option<ArcChildRenderObject<PP>>;
}

impl<E: FullElement> ChildElementNode<E::ParentProtocol> for ElementNode<E> {
    fn context(&self) -> &ElementContextNode {
        self.context.as_ref()
    }

    fn as_arc_any(self: Arc<Self>) -> ArcAnyElementNode {
        self
    }

    fn can_rebuild_with(
        self: Arc<Self>,
        widget: ArcChildWidget<E::ParentProtocol>,
    ) -> Result<
        ElementReconcileItem<E::ParentProtocol>,
        (
            ArcChildElementNode<E::ParentProtocol>,
            ArcChildWidget<E::ParentProtocol>,
        ),
    > {
        Self::can_rebuild_with(self, widget).map_err(|(element, widget)| (element as _, widget))
    }

    fn get_current_subtree_render_object(&self) -> Option<ArcChildRenderObject<E::ParentProtocol>> {
        let snapshot = self.snapshot.lock();

        let MainlineState::Ready {
            children,
            render_object,
            ..
        } = snapshot.inner.mainline_ref()?.state.as_ref()?
        else {
            return None;
        };

        <<E as Element>::Impl as ImplElementNode<E>>::get_current_subtree_render_object(
            render_object,
            children,
        )
    }
}

impl<E: FullElement> AnyElementNode for ElementNode<E> {
    fn as_any_arc(self: Arc<Self>) -> ArcAnyElementNode {
        self
    }

    fn push_job(&self, job_id: JobId) {
        todo!()
    }

    fn render_object(&self) -> Result<ArcAnyRenderObject, &str> {
        if !<E as Element>::Impl::HAS_RENDER {
            return Err("Render object call should not be called on a non-RenderElement");
        };
        let snapshot = self.snapshot.lock();
        let Some(Mainline {
            state: Some(MainlineState::Ready { render_object, .. }),
            ..
        }) = snapshot.inner.mainline_ref()
        else {
            return Err("Render object call should only be called on element nodes that are ready and attached");
        };
        <E as Element>::Impl::get_render_object(render_object)
            .ok_or("Render object call should only be called on after render object is attached")
    }
}
