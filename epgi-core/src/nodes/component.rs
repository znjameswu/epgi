use std::{any::TypeId, marker::PhantomData};

use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::{
    foundation::{Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Key, Protocol, Provide},
    template::{ImplByTemplate, SingleChildElement, SingleChildElementTemplate},
    tree::{ArcAnyWidget, ArcChildWidget, ArcWidget, BuildContext, ElementBase, Widget, WidgetExt},
};

// ComponentWidget and Consumer are separated due to the virtual call overhead in get_consumed_types
// ComponentWidget and Provider are separated due to type inconsistencies in Element::Provided
// ComponentWidget are non-suspendable because suspend error in virtual function signature would massively
// ruin the following dead branch elimination of the suspend path
pub trait ComponentWidget<P: Protocol>:
    Widget<Element = ComponentElement<P>, ParentProtocol = P, ChildProtocol = P> + WidgetExt
{
    fn build(&self, ctx: BuildContext<'_>) -> ArcChildWidget<P>;
}

impl<P: Protocol> ArcWidget for Asc<dyn ComponentWidget<P>> {
    type Element = ComponentElement<P>;

    fn into_any_widget(self) -> ArcAnyWidget {
        self.as_arc_any_widget()
    }

    fn into_child_widget(self) -> ArcChildWidget<P> {
        self.as_arc_child_widget()
    }

    fn widget_type_id(&self) -> TypeId {
        WidgetExt::widget_type_id(self.as_ref())
    }

    fn key(&self) -> Option<&dyn Key> {
        Widget::key(self.as_ref())
    }
}

#[derive(Default, Clone)]
pub struct ComponentElement<P: Protocol>(PhantomData<P>);

impl<P: Protocol> ImplByTemplate for ComponentElement<P> {
    type Template = SingleChildElementTemplate<false, false>;
}

impl<P: Protocol> SingleChildElement for ComponentElement<P> {
    type ParentProtocol = P;
    type ChildProtocol = P;
    type ArcWidget = Asc<dyn ComponentWidget<P>>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<P>, BuildSuspendedError> {
        Ok(widget.build(ctx))
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self(PhantomData)
    }
}

pub trait SuspendableComponentWidget<P: Protocol>:
    Widget<Element = SuspendableComponentElement<P>, ParentProtocol = P, ChildProtocol = P> + WidgetExt
{
    fn build(&self, ctx: BuildContext<'_>) -> Result<ArcChildWidget<P>, BuildSuspendedError>;
}

impl<P: Protocol> ArcWidget for Asc<dyn SuspendableComponentWidget<P>> {
    type Element = SuspendableComponentElement<P>;

    fn into_any_widget(self) -> ArcAnyWidget {
        self.as_arc_any_widget()
    }

    fn into_child_widget(self) -> ArcChildWidget<P> {
        self.as_arc_child_widget()
    }

    fn widget_type_id(&self) -> TypeId {
        WidgetExt::widget_type_id(self.as_ref())
    }

    fn key(&self) -> Option<&dyn Key> {
        Widget::key(self.as_ref())
    }
}

#[derive(Default, Clone)]
pub struct SuspendableComponentElement<P: Protocol>(PhantomData<P>);

impl<P: Protocol> ImplByTemplate for SuspendableComponentElement<P> {
    type Template = SingleChildElementTemplate<false, false>;
}

impl<P: Protocol> SingleChildElement for SuspendableComponentElement<P> {
    type ParentProtocol = P;
    type ChildProtocol = P;
    type ArcWidget = Asc<dyn SuspendableComponentWidget<P>>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<P>, BuildSuspendedError> {
        widget.build(ctx)
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self(PhantomData)
    }
}

#[derive(Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Builder<F, P>>))]
pub struct Builder<F: Fn(BuildContext) -> ArcChildWidget<P> + Send + Sync + 'static, P: Protocol> {
    pub builder: F,
}

impl<F, P> std::fmt::Debug for Builder<F, P>
where
    P: Protocol,
    F: Fn(BuildContext) -> ArcChildWidget<P> + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Function").finish()
    }
}

impl<F, P> Widget for Builder<F, P>
where
    P: Protocol,
    F: Fn(BuildContext) -> ArcChildWidget<P> + Send + Sync + 'static,
{
    type ParentProtocol = P;
    type ChildProtocol = P;
    type Element = ComponentElement<P>;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

impl<F, P> ComponentWidget<P> for Builder<F, P>
where
    P: Protocol,
    F: Fn(BuildContext) -> ArcChildWidget<P> + Send + Sync + 'static,
{
    fn build(&self, ctx: BuildContext<'_>) -> ArcChildWidget<P> {
        (self.builder)(ctx)
    }
}

#[derive(Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<SuspendableBuilder<F, P>>))]
pub struct SuspendableBuilder<
    F: Fn(BuildContext) -> Result<ArcChildWidget<P>, BuildSuspendedError> + Send + Sync + 'static,
    P: Protocol,
> {
    pub builder: F,
}

impl<F, P> std::fmt::Debug for SuspendableBuilder<F, P>
where
    P: Protocol,
    F: Fn(BuildContext) -> Result<ArcChildWidget<P>, BuildSuspendedError> + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Function").finish()
    }
}

impl<F, P> Widget for SuspendableBuilder<F, P>
where
    P: Protocol,
    F: Fn(BuildContext) -> Result<ArcChildWidget<P>, BuildSuspendedError> + Send + Sync + 'static,
{
    type ParentProtocol = P;
    type ChildProtocol = P;
    type Element = SuspendableComponentElement<P>;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

impl<F, P> SuspendableComponentWidget<P> for SuspendableBuilder<F, P>
where
    P: Protocol,
    F: Fn(BuildContext) -> Result<ArcChildWidget<P>, BuildSuspendedError> + Send + Sync + 'static,
{
    fn build(&self, ctx: BuildContext<'_>) -> Result<ArcChildWidget<P>, BuildSuspendedError> {
        (self.builder)(ctx)
    }
}
