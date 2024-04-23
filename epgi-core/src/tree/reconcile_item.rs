use crate::{
    foundation::{Arc, Asc, Protocol},
    sync::CommitBarrier,
};

use super::{
    try_convert_if_same_type, ArcChildElementNode, ArcChildWidget, ArcElementContextNode,
    ArcWidget, ElementNode, ElementReconcileItem, FullElement, WorkContext, WorkHandle,
};

pub enum ReconcileItem<CP: Protocol> {
    Rebuild(Box<dyn ChildElementWidgetPair<CP>>),
    Inflate(ArcChildWidget<CP>),
}

impl<CP> ReconcileItem<CP>
where
    CP: Protocol,
{
    pub fn new_rebuild<E: FullElement<ParentProtocol = CP>>(
        element: Arc<ElementNode<E>>,
        widget: E::ArcWidget,
    ) -> Self {
        Self::Rebuild(Box::new(ElementWidgetPair::<E> { element, widget }))
    }

    pub fn new_inflate(widget: ArcChildWidget<CP>) -> Self {
        Self::Inflate(widget)
    }
}

impl<CP> ReconcileItem<CP>
where
    CP: Protocol,
{
    fn into_async_item(
        self,
        work_context: Asc<WorkContext>,
        host_element_context: ArcElementContextNode,
        barrier: CommitBarrier,
    ) -> AsyncReconcileItem<CP> {
        match self {
            ReconcileItem::Rebuild(_) => todo!(),
            ReconcileItem::Inflate(widget) => {
                let handle = WorkHandle::new();
                todo!()
            }
        }
    }
}

struct AsyncReconcileItem<CP: Protocol> {
    inner: AsyncReconcileItemInner<CP>,
    work_context: Asc<WorkContext>,
    parent_handle: WorkHandle,
    barrier: CommitBarrier,
}

enum AsyncReconcileItemInner<CP: Protocol> {
    Rebuild(Box<dyn ChildElementWidgetPair<CP>>),
    Inflate(ArcChildElementNode<CP>),
}

impl<CP> AsyncReconcileItem<CP>
where
    CP: Protocol,
{
    fn element(&self) -> ArcChildElementNode<CP> {
        match &self.inner {
            AsyncReconcileItemInner::Rebuild(pair) => pair.element(),
            AsyncReconcileItemInner::Inflate(element) => element.clone(),
        }
    }
    fn perform_reconcile(self) {
        match self.inner {
            AsyncReconcileItemInner::Rebuild(pair) => {
                pair.rebuild_async_box(self.work_context, self.parent_handle, self.barrier)
            }
            AsyncReconcileItemInner::Inflate(element) => todo!(),
        }
    }
}

impl<E: FullElement> ElementNode<E> {
    pub(crate) fn can_rebuild_with(
        self: Arc<Self>,
        widget: ArcChildWidget<E::ParentProtocol>,
    ) -> Result<
        ElementReconcileItem<E::ParentProtocol>,
        (Arc<Self>, ArcChildWidget<E::ParentProtocol>),
    > {
        let old_widget = self.widget();
        if widget.key() != old_widget.key() {
            return Err((self, widget));
        }
        match try_convert_if_same_type(&old_widget, widget) {
            Ok(widget) => Ok(ElementReconcileItem::new_update::<E>(self, widget)),
            Err(widget) => Err((self, widget)),
        }
    }
}

pub struct ElementWidgetPair<E: FullElement> {
    pub element: Arc<ElementNode<E>>,
    pub widget: E::ArcWidget,
}

impl<E> Clone for ElementWidgetPair<E>
where
    E: FullElement,
{
    fn clone(&self) -> Self {
        Self {
            element: self.element.clone(),
            widget: self.widget.clone(),
        }
    }
}

pub trait ChildElementWidgetPair<P: Protocol>:
    crate::sync::ChildElementWidgetPairSyncBuildExt<P>
    + crate::r#async::ChildElementWidgetPairAsyncBuildExt<P>
    + Send
    + Sync
    + 'static
{
    fn element(&self) -> ArcChildElementNode<P>;
}

impl<E> ChildElementWidgetPair<E::ParentProtocol> for ElementWidgetPair<E>
where
    E: FullElement,
{
    fn element(&self) -> ArcChildElementNode<E::ParentProtocol> {
        todo!()
        // self.element.clone() as _
    }
}

mod reconciler_private {}
