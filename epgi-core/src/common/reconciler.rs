use crate::{
    foundation::{Arc, Asc, HktContainer, InlinableDwsizeVec, Parallel, Protocol},
    sync::CommitBarrier,
};

use super::{
    try_convert_if_same_type, ArcChildElementNode, ArcChildWidget, ArcElementContextNode,
    ArcWidget, BuildContext, Element, ElementNode, Widget, WorkContext, WorkHandle,
};

pub trait Reconciler<CP: Protocol> {
    fn build_context_mut(&mut self) -> BuildContext<'_>;

    fn nodes_needing_unmount_mut(&mut self) -> &mut InlinableDwsizeVec<ArcChildElementNode<CP>>;

    fn into_reconcile<I: Parallel<Item = ReconcileItem<CP>>>(
        self,
        items: I,
    ) -> <I::HktContainer as HktContainer>::Container<ArcChildElementNode<CP>>;
}

pub enum ReconcileItem<CP: Protocol> {
    Rebuild(Box<dyn ChildElementWidgetPair<CP>>),
    Inflate(ArcChildWidget<CP>),
}

impl<CP> ReconcileItem<CP>
where
    CP: Protocol,
{
    pub fn new_rebuild<E: Element<ParentProtocol = CP>>(
        element: Arc<ElementNode<E>>,
        widget: E::ArcWidget,
    ) -> Self {
        Self::Rebuild(Box::new(ElementWidgetPair { element, widget }))
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

impl<E> ElementNode<E>
where
    E: Element,
{
    pub(crate) fn can_rebuild_with(
        self: Arc<Self>,
        widget: ArcChildWidget<E::ParentProtocol>,
    ) -> Result<ReconcileItem<E::ParentProtocol>, (Arc<Self>, ArcChildWidget<E::ParentProtocol>)>
    {
        let old_widget = self.widget();
        if widget.key() != old_widget.key() {
            return Err((self, widget));
        }
        match try_convert_if_same_type(&old_widget, widget) {
            Ok(widget) => Ok(ReconcileItem::new_rebuild(self, widget)),
            Err(widget) => Err((self, widget)),
        }
    }
}

pub struct ElementWidgetPair<E: Element> {
    pub element: Arc<ElementNode<E>>,
    pub widget: E::ArcWidget,
}

impl<E> Clone for ElementWidgetPair<E>
where
    E: Element,
{
    fn clone(&self) -> Self {
        Self {
            element: self.element.clone(),
            widget: self.widget.clone(),
        }
    }
}

pub trait ChildElementWidgetPair<P: Protocol>:
    crate::sync::reconciler_private::ChildElementWidgetPairSyncBuildExt<P>
    + crate::r#async::reconciler_private::ChildElementWidgetPairAsyncBuildExt<P>
    + Send
    + Sync
    + 'static
{
    fn element(&self) -> ArcChildElementNode<P>;
}

impl<E> ChildElementWidgetPair<E::ParentProtocol> for ElementWidgetPair<E>
where
    E: Element,
{
    fn element(&self) -> ArcChildElementNode<E::ParentProtocol> {
        self.element.clone() as _
    }
}

mod reconciler_private {}
