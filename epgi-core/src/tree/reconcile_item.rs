use crate::foundation::{Arc, Protocol};

use super::{
    try_convert_if_same_type, ArcChildElementNode, ArcChildWidget, ArcWidget, ElementNode,
    ElementReconcileItem, FullElement,
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
        self.element.clone() as _
    }
}

mod reconciler_private {}
