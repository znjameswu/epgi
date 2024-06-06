use std::{any::TypeId, marker::PhantomData};

use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::{
    foundation::{
        Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Key, Protocol, Provide, TypeKey,
    },
    template::{ImplByTemplate, SingleChildElement, SingleChildElementTemplate},
    tree::{ArcAnyWidget, ArcChildWidget, ArcWidget, BuildContext, ElementBase, Widget, WidgetExt},
};

pub trait ConsumerWidget<P: Protocol>:
    Widget<Element = ConsumerElement<P>, ParentProtocol = P, ChildProtocol = P> + WidgetExt
{
    #[allow(unused_variables)]
    fn get_consumed_types(&self) -> &[TypeKey];

    fn build(&self, provider_values: InlinableDwsizeVec<Arc<dyn Provide>>) -> ArcChildWidget<P>;
}

impl<P: Protocol> ArcWidget for Asc<dyn ConsumerWidget<P>> {
    type Element = ConsumerElement<P>;

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
pub struct ConsumerElement<P: Protocol>(PhantomData<P>);

impl<P: Protocol> ImplByTemplate for ConsumerElement<P> {
    type Template = SingleChildElementTemplate<false, false>;
}

impl<P: Protocol> SingleChildElement for ConsumerElement<P> {
    type ParentProtocol = P;
    type ChildProtocol = P;
    type ArcWidget = Asc<dyn ConsumerWidget<P>>;

    fn get_consumed_types(widget: &Self::ArcWidget) -> &[TypeKey] {
        widget.get_consumed_types()
    }

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<Self::ChildProtocol>, BuildSuspendedError> {
        Ok(widget.build(provider_values))
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self(PhantomData)
    }
}

#[derive(Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Consumer<T, F, P>>))]
pub struct Consumer<
    T: Provide,
    F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Send + Sync + 'static,
    P: Protocol,
> {
    pub builder: F,
    #[builder(default)]
    pub phantom: PhantomData<T>,
}

impl<T, F, P> std::fmt::Debug for Consumer<T, F, P>
where
    T: Provide,
    F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Send + Sync + 'static,
    P: Protocol,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Consumer")
            .field("Type", &TypeKey::of::<T>())
            .finish()
    }
}

impl<T, F, P> Widget for Consumer<T, F, P>
where
    T: Provide,
    F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Send + Sync + 'static,
    P: Protocol,
{
    type ParentProtocol = P;
    type ChildProtocol = P;
    type Element = ConsumerElement<P>;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

impl<T, F, P> ConsumerWidget<P> for Consumer<T, F, P>
where
    T: Provide,
    F: Fn(&mut BuildContext, T) -> ArcChildWidget<P> + Send + Sync + 'static,
    P: Protocol,
{
    fn get_consumed_types(&self) -> &[TypeKey] {
        &[TypeKey::of::<T>()]
    }

    fn build(&self, provider_values: InlinableDwsizeVec<Arc<dyn Provide>>) -> ArcChildWidget<P> {
        todo!()
    }
}
