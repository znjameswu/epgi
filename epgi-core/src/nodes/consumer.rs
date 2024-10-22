use std::{any::TypeId, marker::PhantomData};

use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::{
    foundation::{
        Arc, Asc, AscProvideExt, BuildSuspendedError, InlinableDwsizeVec, Key, Protocol, Provide,
        TypeKey,
    },
    template::{ImplByTemplate, SingleChildElement, SingleChildElementTemplate},
    tree::{ArcAnyWidget, ArcChildWidget, ArcWidget, BuildContext, ElementBase, Widget, WidgetExt},
};

pub trait ConsumerWidget<P: Protocol>:
    Widget<Element = ConsumerElement<P>, ParentProtocol = P, ChildProtocol = P> + WidgetExt
{
    #[allow(unused_variables)]
    fn get_consumed_types(&self) -> &[TypeKey];

    fn build(
        &self,
        ctx: &mut BuildContext,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> ArcChildWidget<P>;
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

    fn get_consumed_types(widget: &Self::ArcWidget) -> impl AsRef<[TypeKey]> {
        widget.get_consumed_types()
    }

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<Self::ChildProtocol>, BuildSuspendedError> {
        Ok(widget.build(ctx, provider_values))
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self(PhantomData)
    }
}

#[derive(Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Consumer<T, F, P>>))]
pub struct Consumer<
    T: Provide,
    F: Fn(&mut BuildContext, Asc<T>) -> ArcChildWidget<P> + Send + Sync + 'static,
    P: Protocol,
> {
    pub builder: F,
    #[builder(default, setter(skip))]
    phantom: PhantomData<T>,
    #[builder(default = TypeKey::of::<T>(), setter(skip))]
    type_key: TypeKey,
}

impl<T, F, P> std::fmt::Debug for Consumer<T, F, P>
where
    T: Provide,
    F: Fn(&mut BuildContext, Asc<T>) -> ArcChildWidget<P> + Send + Sync + 'static,
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
    F: Fn(&mut BuildContext, Asc<T>) -> ArcChildWidget<P> + Send + Sync + 'static,
    P: Protocol,
{
    type ParentProtocol = P;
    type ChildProtocol = P;
    type Element = ConsumerElement<P>;

    fn into_arc_widget(self: Asc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

impl<T, F, P> ConsumerWidget<P> for Consumer<T, F, P>
where
    T: Provide,
    F: Fn(&mut BuildContext, Asc<T>) -> ArcChildWidget<P> + Send + Sync + 'static,
    P: Protocol,
{
    fn get_consumed_types(&self) -> &[TypeKey] {
        std::array::from_ref(&self.type_key)
    }

    fn build(
        &self,
        ctx: &mut BuildContext,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> ArcChildWidget<P> {
        assert_eq!(
            provider_values.len(),
            1,
            "Consumer widget should only receive exactly one provider value"
        );
        let value = provider_values
            .into_iter()
            .next()
            .expect("Impossible to fail");
        let value = value
            .downcast::<T>()
            .ok()
            .expect("Received provider value should be of correct type");
        (self.builder)(ctx, value)
    }
}

macro_rules! impl_multi_consumer {
    ($name: ident, $count: literal, $($t:ident),*) => {
        #[derive(Declarative, TypedBuilder)]
        #[builder(build_method(into=Asc<$name<$($t),*, F, P>>))]
        pub struct $name<
            $($t: Provide),*,
            F: Fn(&mut BuildContext, $(Asc<$t>),*) -> ArcChildWidget<P> + Send + Sync + 'static,
            P: Protocol,
        > {
            pub builder: F,
            #[builder(default, setter(skip))]
            phantom: PhantomData<($($t),*)>,
            #[builder(default = [$(TypeKey::of::<$t>()),*], setter(skip))]
            type_keys: [TypeKey; $count],
        }

        impl<$($t),*, F, P> std::fmt::Debug for $name<$($t),*, F, P>
        where
            $($t: Provide),*,
            F: Fn(&mut BuildContext, $(Asc<$t>),*) -> ArcChildWidget<P> + Send + Sync + 'static,
            P: Protocol,
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("Consumer")
                    .field("Types", &self.type_keys)
                    .finish()
            }
        }

        impl<$($t),*, F, P> Widget for $name<$($t),*, F, P>
        where
            $($t: Provide),*,
            F: Fn(&mut BuildContext, $(Asc<$t>),*) -> ArcChildWidget<P> + Send + Sync + 'static,
            P: Protocol,
        {
            type ParentProtocol = P;
            type ChildProtocol = P;
            type Element = ConsumerElement<P>;

            fn into_arc_widget(
                self: Asc<Self>,
            ) -> <Self::Element as ElementBase>::ArcWidget {
                self
            }
        }

        impl<$($t),*, F, P> ConsumerWidget<P> for $name<$($t),*, F, P>
        where
            $($t: Provide),*,
            F: Fn(&mut BuildContext, $(Asc<$t>),*) -> ArcChildWidget<P> + Send + Sync + 'static,
            P: Protocol,
        {
            fn get_consumed_types(&self) -> &[TypeKey] {
                self.type_keys.as_ref().into()
            }

            fn build(
                &self,
                ctx: &mut BuildContext,
                provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
            ) -> ArcChildWidget<P> {
                assert_eq!(
                    provider_values.len(),
                    $count,
                    concat!(stringify!($name), " widget should only receive exactly ", $count, " provider value")
                );
                let mut provider_values = provider_values.into_iter();
                (self.builder)(
                    ctx,
                    $({
                        provider_values
                            .next()
                            .expect("Impossible to fail")
                            .downcast::<$t>()
                            .ok()
                            .expect("Received provider value should be of correct type")
                    }),*
                )
            }
        }
    };
}

impl_multi_consumer!(Consumer2, 2, T1, T2);
impl_multi_consumer!(Consumer3, 3, T1, T2, T3);
impl_multi_consumer!(Consumer4, 4, T1, T2, T3, T4);
