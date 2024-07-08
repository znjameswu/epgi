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

/// Provides an immutable value to the subtree for subscription.
/// 
/// [`Provider`] does NOT track internal mutability in its state. The only way to notify subtree of a value change
/// is to provide a new [`Provider`] with a different [`Provider::value`] during `build` function.
/// State changes should only be handled by [`BuildContext::use_state`] and similar hooks,
/// and [`Provider`] only serves to propagate that change to the subtree.
/// 
/// [`Provider`] corresponds to `Provider.value` in Flutter's `provider` package. 
/// On the contrary, the default Flutter `Provider` has no corresponding construct in EPGI.
/// The rationale is that by tracking internal mutabilities, our functionalities would overlap with [`BuildContext::use_state`],
/// and the user is forced to write thread-safe internal object.
#[derive(Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Provider<T, P>>))]
pub struct Provider<T: Provide, P: Protocol> {
    #[builder(setter(into))]
    pub value: Asc<T>,
    pub child: ArcChildWidget<P>,
}

impl<T: Provide, P: Protocol> std::fmt::Debug for Provider<T, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Provider")
            .field("value", &self.value)
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

    fn get_provided_value(widget: &Self::ArcWidget) -> &Arc<Self::Provided> {
        &widget.value
    }
}
