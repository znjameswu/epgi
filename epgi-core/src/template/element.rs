use crate::{
    foundation::{
        Arc, BuildSuspendedError, ContainerOf, HktContainer, InlinableDwsizeVec, Protocol, Provide,
        TypeKey,
    },
    tree::{
        ArcChildElementNode, ArcChildWidget, ArcWidget, BuildContext,
        ChildRenderObjectsUpdateCallback, Element, ElementBase, ElementReconcileItem, FullRender,
        ImplElement, ProvideElement, RenderAction, RenderElement,
    },
};

use super::ImplByTemplate;

pub trait TemplateElementBase<E> {
    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol;
    type ChildContainer: HktContainer;

    type ArcWidget: ArcWidget<Element = E>;

    #[allow(unused_variables)]
    fn get_consumed_types(widget: &Self::ArcWidget) -> &[TypeKey] {
        &[]
    }
    fn perform_rebuild_element(
        element: &mut E,
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        children: ContainerOf<Self::ChildContainer, ArcChildElementNode<Self::ChildProtocol>>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<Self::ChildProtocol>>,
    ) -> Result<
        (
            ContainerOf<Self::ChildContainer, ElementReconcileItem<Self::ChildProtocol>>,
            Option<ChildRenderObjectsUpdateCallback<Self::ChildContainer, Self::ChildProtocol>>,
        ),
        (
            ContainerOf<Self::ChildContainer, ArcChildElementNode<Self::ChildProtocol>>,
            BuildSuspendedError,
        ),
    >;

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<
        (
            E,
            ContainerOf<Self::ChildContainer, ArcChildWidget<Self::ChildProtocol>>,
        ),
        BuildSuspendedError,
    >;
}

impl<E> ElementBase for E
where
    E: ImplByTemplate,
    E::Template: TemplateElementBase<E>,
    E: Clone + Send + Sync + Sized + 'static,
{
    type ParentProtocol = <E::Template as TemplateElementBase<E>>::ParentProtocol;
    type ChildProtocol = <E::Template as TemplateElementBase<E>>::ChildProtocol;
    type ChildContainer = <E::Template as TemplateElementBase<E>>::ChildContainer;

    type ArcWidget = <E::Template as TemplateElementBase<E>>::ArcWidget;

    // type ElementImpl = <E::Template as TemplateElement<E>>::ElementImpl;

    fn perform_rebuild_element(
        &mut self,
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        children: ContainerOf<Self::ChildContainer, ArcChildElementNode<Self::ChildProtocol>>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<Self::ChildProtocol>>,
    ) -> Result<
        (
            ContainerOf<Self::ChildContainer, ElementReconcileItem<Self::ChildProtocol>>,
            Option<ChildRenderObjectsUpdateCallback<Self::ChildContainer, Self::ChildProtocol>>,
        ),
        (
            ContainerOf<Self::ChildContainer, ArcChildElementNode<Self::ChildProtocol>>,
            BuildSuspendedError,
        ),
    > {
        E::Template::perform_rebuild_element(
            self,
            widget,
            ctx,
            provider_values,
            children,
            nodes_needing_unmount,
        )
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<
        (
            Self,
            ContainerOf<Self::ChildContainer, ArcChildWidget<Self::ChildProtocol>>,
        ),
        BuildSuspendedError,
    > {
        E::Template::perform_inflate_element(widget, ctx, provider_values)
    }
}

pub trait TemplateElement<E: ElementBase> {
    type Impl: ImplElement<E>;
}

impl<E> Element for E
where
    E: ImplByTemplate,
    E::Template: TemplateElement<E>,
    E: ElementBase,
{
    type Impl = <E::Template as TemplateElement<E>>::Impl;
}

pub trait TemplateRenderElement<E: ElementBase> {
    type Render: FullRender<
        ParentProtocol = E::ParentProtocol,
        ChildProtocol = E::ChildProtocol,
        ChildContainer = E::ChildContainer,
    >;

    fn create_render(element: &E, widget: &E::ArcWidget) -> Self::Render;
    /// Update necessary properties of render object given by the widget
    ///
    /// Called during the commit phase, when the widget is updated.
    /// Always called after [RenderElement::try_update_render_object_children].
    /// If that call failed to update children (indicating suspense), then this call will be skipped.
    fn update_render(render: &mut Self::Render, widget: &E::ArcWidget) -> Option<RenderAction>;

    /// Whether [Render::update_render_object] is a no-op and always returns None
    ///
    /// When set to true, [Render::update_render_object]'s implementation will be ignored,
    /// Certain optimizations to reduce mutex usages will be applied during the commit phase.
    /// However, if [Render::update_render_object] is actually not no-op, doing this will cause unexpected behaviors.
    ///
    /// Setting to false will always guarantee the correct behavior.
    const NOOP_UPDATE_RENDER_OBJECT: bool;
}

impl<E> RenderElement for E
where
    E: ImplByTemplate,
    E::Template: TemplateRenderElement<E>,
    E: ElementBase,
{
    type Render = <E::Template as TemplateRenderElement<E>>::Render;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        E::Template::create_render(self, widget)
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        E::Template::update_render(render, widget)
    }
}

pub trait TemplateProvideElement<E: ElementBase> {
    type Provided: Provide;
    fn get_provided_value(widget: &E::ArcWidget) -> Arc<Self::Provided>;
}

impl<E> ProvideElement for E
where
    E: ImplByTemplate,
    E::Template: TemplateProvideElement<E>,
    E: ElementBase,
{
    type Provided = <E::Template as TemplateProvideElement<E>>::Provided;

    fn get_provided_value(widget: &Self::ArcWidget) -> Arc<Self::Provided> {
        E::Template::get_provided_value(widget)
    }
}
