use std::marker::PhantomData;

use crate::{
    foundation::{
        Arc, ArrayContainer, BuildSuspendedError, InlinableDwsizeVec, Protocol, Provide, TypeKey,
    },
    template::{
        ImplByTemplate, TemplateElement, TemplateElementBase, TemplateProvideElement,
        TemplateRenderElement,
    },
    tree::{
        ArcChildElementNode, ArcChildWidget, ArcWidget, BuildContext,
        ChildRenderObjectsUpdateCallback, ElementBase, ElementImpl, ElementReconcileItem,
        ImplElement, Render, RenderAction,
    },
};

pub struct ProxyElementTemplate<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool>(
    PhantomData<E>,
);

pub trait ProxyElement: Clone + Send + Sync + Sized + 'static {
    type Protocol: Protocol;

    type ArcWidget: ArcWidget<Element = Self>;

    // ~~TypeId::of is not constant function so we have to work around like this.~~ Reuse Element for different widget.
    // Boxed slice generates worse code than Vec due to https://github.com/rust-lang/rust/issues/59878
    #[allow(unused_variables)]
    fn get_consumed_types(widget: &Self::ArcWidget) -> &[TypeKey] {
        &[]
    }

    // SAFETY: No async path should poll or await the stashed continuation left behind by the sync build. Awaiting outside the sync build will cause child tasks to be run outside of sync build while still being the sync variant of the task.
    // Rationale for a moving self: Allows users to destructure the self without needing to fill in a placeholder value.
    /// If a hook suspended, then the untouched Self should be returned along with the suspended error
    /// If nothing suspended, then the new Self should be returned.
    fn perform_rebuild_element(
        &mut self,
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        child: ArcChildElementNode<Self::Protocol>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<Self::Protocol>>,
    ) -> Result<
        ElementReconcileItem<Self::Protocol>,
        (ArcChildElementNode<Self::Protocol>, BuildSuspendedError),
    >;

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(Self, ArcChildWidget<Self::Protocol>), BuildSuspendedError>;
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateElementBase<E>
    for ProxyElementTemplate<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: ProxyElement,
{
    type ParentProtocol = E::Protocol;
    type ChildProtocol = E::Protocol;
    type ChildContainer = ArrayContainer<1>;

    type ArcWidget = E::ArcWidget;

    fn perform_rebuild_element(
        element: &mut E,
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        [child]: [ArcChildElementNode<E::Protocol>; 1],
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<E::Protocol>>,
    ) -> Result<
        (
            [ElementReconcileItem<E::Protocol>; 1],
            Option<ChildRenderObjectsUpdateCallback<Self::ChildContainer, E::Protocol>>,
        ),
        ([ArcChildElementNode<E::Protocol>; 1], BuildSuspendedError),
    > {
        E::perform_rebuild_element(
            element,
            widget,
            ctx,
            provider_values,
            child,
            nodes_needing_unmount,
        )
        .map(|item| ([item], None))
        .map_err(|(child, error)| ([child], error))
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(E, [ArcChildWidget<E::Protocol>; 1]), BuildSuspendedError> {
        E::perform_inflate_element(widget, ctx, provider_values)
            .map(|(element, child_widget)| (element, [child_widget]))
    }
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateElement<E>
    for ProxyElementTemplate<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ElementBase,
    ElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>: ImplElement<Element = E>,
{
    type ElementImpl = ElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>;
}

pub trait ProxyRenderElement: ProxyElement {
    type Render: Render<
        ParentProtocol = Self::Protocol,
        ChildProtocol = Self::Protocol,
        ChildContainer = ArrayContainer<1>,
    >;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render;
    /// Update necessary properties of render object given by the widget
    ///
    /// Called during the commit phase, when the widget is updated.
    /// Always called after [RenderElement::try_update_render_object_children].
    /// If that call failed to update children (indicating suspense), then this call will be skipped.
    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> RenderAction;

    /// Whether [Render::update_render_object] is a no-op and always returns None
    ///
    /// When set to true, [Render::update_render_object]'s implementation will be ignored,
    /// Certain optimizations to reduce mutex usages will be applied during the commit phase.
    /// However, if [Render::update_render_object] is actually not no-op, doing this will cause unexpected behaviors.
    ///
    /// Setting to false will always guarantee the correct behavior.
    const NOOP_UPDATE_RENDER_OBJECT: bool = false;
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateRenderElement<E>
    for ProxyElementTemplate<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: ProxyRenderElement,
{
    type Render = E::Render;

    fn create_render(element: &E, widget: &<E as ElementBase>::ArcWidget) -> Self::Render {
        E::create_render(element, widget)
    }

    fn update_render(
        render: &mut Self::Render,
        widget: &<E as ElementBase>::ArcWidget,
    ) -> RenderAction {
        E::update_render(render, widget)
    }

    const NOOP_UPDATE_RENDER_OBJECT: bool = E::NOOP_UPDATE_RENDER_OBJECT;
}

pub trait ProxyProvideElement: ProxyElement {
    type Provided: Provide;
    fn get_provided_value(widget: &Self::ArcWidget) -> Arc<Self::Provided>;
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateProvideElement<E>
    for ProxyElementTemplate<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: ProxyProvideElement,
{
    type Provided = E::Provided;

    fn get_provided_value(widget: &<E as ElementBase>::ArcWidget) -> Arc<Self::Provided> {
        E::get_provided_value(widget)
    }
}