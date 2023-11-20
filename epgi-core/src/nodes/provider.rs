use std::marker::PhantomData;

use crate::{
    foundation::{
        Arc, ArrayContainer, Asc, BuildSuspendedError, InlinableDwsizeVec, Protocol, Provide,
    },
    tree::{
        ArcChildElementNode, ArcChildWidget, BuildContext, ChildRenderObjectsUpdateCallback,
        Element, ElementReconcileItem, Widget,
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
    type ParentProtocol = P;

    type ChildProtocol = P;

    type Element = ProviderElement<T, P>;

    fn into_arc_widget(self: Asc<Self>) -> Asc<Self> {
        self
    }
}

pub struct ProviderElement<T: Provide, P: Protocol>(PhantomData<(T, P)>);

impl<T, P> Clone for ProviderElement<T, P>
where
    T: Provide,
    P: Protocol,
{
    fn clone(&self) -> Self {
        Self(PhantomData)
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

    type ChildContainer = ArrayContainer<1>;

    type Provided = T;
    const GET_PROVIDED_VALUE: Option<fn(&Self::ArcWidget) -> Arc<Self::Provided>> =
        Some(|widget| (widget.init)());

    fn perform_rebuild_element(
        &mut self,
        widget: &Self::ArcWidget,
        _ctx: BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        children: [ArcChildElementNode<P>; 1],
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<P>>,
    ) -> Result<
        (
            [ElementReconcileItem<P>; 1],
            Option<ChildRenderObjectsUpdateCallback<Self>>,
        ),
        ([ArcChildElementNode<P>; 1], BuildSuspendedError),
    > {
        let [child] = children;
        match child.can_rebuild_with(widget.child.clone()) {
            Ok(item) => Ok(([item], None)),
            Err((child, child_widget)) => {
                nodes_needing_unmount.push(child);
                Ok(([ElementReconcileItem::new_inflate(child_widget)], None))
            }
        }
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        _ctx: BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(Self, [ArcChildWidget<P>; 1]), BuildSuspendedError> {
        let child_widget = widget.child.clone();
        Ok((Self(PhantomData), [child_widget]))
    }

    type RenderOrUnit = ();
}
