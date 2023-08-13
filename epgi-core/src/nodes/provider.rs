use std::marker::PhantomData;

use crate::{
    foundation::{Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Never, Protocol, Provide},
    tree::{
        ArcChildElementNode, ArcChildWidget, Element, ReconcileItem, Reconciler,
        SingleChildElement, Widget,
    },
};

pub struct Provider<T: Provide, P: Protocol> {
    pub init: Box<dyn Fn() -> Asc<T> + Send + Sync>,
    pub child: ArcChildWidget<P>,
}
impl<T, P> Provider<T, P>
where
    T: Provide,
    P: Protocol,
{
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

impl<T, P> std::fmt::Debug for Provider<T, P>
where
    T: Provide,
    P: Protocol,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Provider")
            .field("Type", &std::any::type_name::<T>())
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl<T, P> Widget for Provider<T, P>
where
    T: Provide,
    P: Protocol,
{
    type Element = ProviderElement<T, P>;

    fn into_arc_widget(self: Arc<Self>) -> <Self::Element as Element>::ArcWidget {
        self
    }
}

pub struct ProviderElement<T: Provide, P: Protocol> {
    pub child: ArcChildElementNode<P>,
    phantom: PhantomData<T>,
}

impl<T, P> Clone for ProviderElement<T, P>
where
    T: Provide,
    P: Protocol,
{
    fn clone(&self) -> Self {
        Self {
            child: self.child.clone(),
            phantom: self.phantom.clone(),
        }
    }
}

impl<T, P> Element for ProviderElement<T, P>
where
    T: Provide,
    P: Protocol,
{
    type ArcWidget = Asc<Provider<T, P>>;

    type ParentProtocol = P;

    type ChildProtocol = P;

    type Provided = T;
    const GET_PROVIDED_VALUE: Option<fn(&Self::ArcWidget) -> Arc<Self::Provided>> =
        Some(|widget| (widget.init)());

    fn perform_rebuild_element(
        self,
        widget: &Self::ArcWidget,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        mut reconciler: impl Reconciler<Self::ChildProtocol>,
    ) -> Result<Self, (Self, BuildSuspendedError)> {
        match self.child.can_rebuild_with(widget.child.clone()) {
            Ok(item) => {
                let [child] = reconciler.into_reconcile([item]);
                Ok(Self {
                    child,
                    phantom: PhantomData,
                })
            }
            Err((child, child_widget)) => {
                reconciler.nodes_needing_unmount_mut().push(child);
                let [child] = reconciler.into_reconcile([ReconcileItem::new_inflate(child_widget)]);
                Ok(Self {
                    child,
                    phantom: PhantomData,
                })
            }
        }
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        reconciler: impl Reconciler<Self::ChildProtocol>, // TODO: A specialized reconciler for inflate, to save passing &JobIds
    ) -> Result<Self, BuildSuspendedError> {
        let child_widget = widget.child.clone();
        let [child] = reconciler.into_reconcile([ReconcileItem::new_inflate(child_widget)]);
        Ok(Self {
            child,
            phantom: PhantomData,
        })
    }

    type ChildIter = [ArcChildElementNode<P>; 1];

    fn children(&self) -> Self::ChildIter {
        [self.child.clone()]
    }

    type ArcRenderObject = Never;
}

impl<T, P> SingleChildElement for ProviderElement<T, P>
where
    T: Provide,
    P: Protocol,
{
    fn child(&self) -> &ArcChildElementNode<Self::ParentProtocol> {
        &self.child
    }
}
