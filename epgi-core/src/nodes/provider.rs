use std::marker::PhantomData;

use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::{
    foundation::{Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Protocol, Provide},
    template::{
        ImplByTemplate, SingleChildElement, SingleChildElementTemplate, SingleChildProvideElement,
    },
    tree::{ArcChildWidget, BuildContext, Widget},
};

#[derive(Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Provider<T, P>>))]
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
    type Template = SingleChildElementTemplate<false, true>;
}

impl<T: Provide, P: Protocol> SingleChildElement for ProviderElement<T, P> {
    type ParentProtocol = P;
    type ChildProtocol = P;
    type ArcWidget = Asc<Provider<T, P>>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<P>, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self(PhantomData)
    }
}

impl<T: Provide, P: Protocol> SingleChildProvideElement for ProviderElement<T, P> {
    type Provided = T;

    fn get_provided_value(widget: &Self::ArcWidget) -> Arc<Self::Provided> {
        (widget.init)()
    }
}
