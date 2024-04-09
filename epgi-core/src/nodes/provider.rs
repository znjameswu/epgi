use std::marker::PhantomData;

use crate::{
    foundation::{Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Protocol, Provide},
    template::{ImplByTemplate, ProxyElement, ProxyElementTemplate, ProxyProvideElement},
    tree::{ArcChildElementNode, ArcChildWidget, BuildContext, ElementReconcileItem, Widget},
};

pub struct Provider<T: Provide, P: Protocol> {
    pub init: Box<dyn Fn() -> Asc<T> + Send + Sync>,
    pub child: ArcChildWidget<P>,
}
impl<T: Provide, P: Protocol> Provider<T, P> {
    pub fn init<F: Fn() -> Asc<T> + Send + Sync + 'static>(
        init: F,
        child: ArcChildWidget<P>,
    ) -> Arc<Self> {
        Arc::new(Self {
            init: Box::new(init),
            child,
        })
    }

    pub fn value(value: Asc<T>, child: ArcChildWidget<P>) -> Arc<Self> {
        Self::init(move || value.clone(), child)
    }

    pub fn value_inner(value: T, child: ArcChildWidget<P>) -> Arc<Self> {
        Self::value(Asc::new(value), child)
    }
}

impl<T: Provide, P: Protocol> std::fmt::Debug for Provider<T, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Provider")
            .field("Type", &std::any::type_name::<T>())
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl<T: Provide, P: Protocol> Widget for Provider<T, P> {
    type ParentProtocol = P;

    type ChildProtocol = P;

    type Element = ProviderElement<T, P>;

    fn into_arc_widget(self: Asc<Self>) -> Asc<Self> {
        self
    }
}

pub struct ProviderElement<T: Provide, P: Protocol>(PhantomData<(T, P)>);

impl<T: Provide, P: Protocol> Clone for ProviderElement<T, P> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<T: Provide, P: Protocol> ImplByTemplate for ProviderElement<T, P> {
    type Template = ProxyElementTemplate<Self, false, true>;
}

impl<T: Provide, P: Protocol> ProxyElement for ProviderElement<T, P> {
    type Protocol = P;

    type ArcWidget = Asc<Provider<T, P>>;

    fn perform_rebuild_element(
        &mut self,
        widget: &Self::ArcWidget,
        _ctx: BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        child: ArcChildElementNode<Self::Protocol>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<Self::Protocol>>,
    ) -> Result<
        ElementReconcileItem<Self::Protocol>,
        (ArcChildElementNode<Self::Protocol>, BuildSuspendedError),
    > {
        let item = match child.can_rebuild_with(widget.child.clone()) {
            Ok(item) => item,
            Err((child, child_widget)) => {
                nodes_needing_unmount.push(child);
                ElementReconcileItem::new_inflate(child_widget)
            }
        };
        Ok(item)
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        _ctx: BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(Self, ArcChildWidget<Self::Protocol>), BuildSuspendedError> {
        let child_widget = widget.child.clone();
        Ok((Self(PhantomData), child_widget))
    }
}

impl<T: Provide, P: Protocol> ProxyProvideElement for ProviderElement<T, P> {
    type Provided = T;

    fn get_provided_value(widget: &Self::ArcWidget) -> Arc<Self::Provided> {
        (widget.init)()
    }
}
