use std::marker::PhantomData;

use crate::{
    foundation::{Arc, BuildSuspendedError, InlinableDwsizeVec, Provide, TypeKey},
    sync::ImplReconcileCommit,
    tree::{
        ArcChildElementNode, ArcChildWidget, BuildContext, ChildRenderObjectsUpdateCallback,
        ContainerOf, Element, ElementReconcileItem, ImplElement, Render, RenderAction,
    },
};

use super::{HasReconcileImpl, HasRenderElementImpl, ImplElementNode, ImplProvide};

pub struct ElementImpl<E: Element, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool>(
    PhantomData<E>,
);

impl<E: Element, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> ImplElement
    for ElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    Self: ImplElementNode<E>,
    // Self: ImplReconcile<E>,
    Self: ImplProvide<E>,
    Self: ImplReconcileCommit<E>,
{
    type Element = E;
}

pub trait Reconcile: Element {
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
        children: ContainerOf<Self, ArcChildElementNode<Self::ChildProtocol>>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<Self::ChildProtocol>>,
    ) -> Result<
        (
            ContainerOf<Self, ElementReconcileItem<Self::ChildProtocol>>,
            Option<ChildRenderObjectsUpdateCallback<Self>>,
        ),
        (
            ContainerOf<Self, ArcChildElementNode<Self::ChildProtocol>>,
            BuildSuspendedError,
        ),
    >;

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(Self, ContainerOf<Self, ArcChildWidget<Self::ChildProtocol>>), BuildSuspendedError>;
}

impl<E: Element, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> HasReconcileImpl<E>
    for ElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: Reconcile,
{
    fn get_consumed_types(widget: &<E>::ArcWidget) -> &[TypeKey] {
        E::get_consumed_types(widget)
    }

    fn perform_rebuild_element(
        element: &mut E,
        widget: &E::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        children: ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<E::ChildProtocol>>,
    ) -> Result<
        (
            ContainerOf<E, ElementReconcileItem<E::ChildProtocol>>,
            Option<ChildRenderObjectsUpdateCallback<E>>,
        ),
        (
            ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
            BuildSuspendedError,
        ),
    > {
        element.perform_rebuild_element(
            widget,
            ctx,
            provider_values,
            children,
            nodes_needing_unmount,
        )
    }

    fn perform_inflate_element(
        widget: &E::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(E, ContainerOf<E, ArcChildWidget<E::ChildProtocol>>), BuildSuspendedError> {
        E::perform_inflate_element(widget, ctx, provider_values)
    }
}

pub trait RenderElement: Element {
    type Render: Render<
        ParentProtocol = Self::ParentProtocol,
        ChildProtocol = Self::ChildProtocol,
        ChildContainer = Self::ChildContainer,
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

impl<E: Element, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> HasRenderElementImpl<E>
    for ElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: RenderElement,
{
    type Render = E::Render;

    fn create_render(element: &E, widget: &<E>::ArcWidget) -> Self::Render {
        E::create_render(element, widget)
    }

    fn update_render(render: &mut Self::Render, widget: &<E>::ArcWidget) -> RenderAction {
        E::update_render(render, widget)
    }
}

pub trait ProvideElement: Element {
    type Provided: Provide;
    fn get_provided_value(widget: &Self::ArcWidget) -> Arc<Self::Provided>;
}

pub trait HasProvideElementImpl<E: Element> {
    type Provided: Provide;
    fn get_provided_value(widget: &E::ArcWidget) -> Arc<Self::Provided>;
}

impl<E: Element, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> HasProvideElementImpl<E>
    for ElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ProvideElement,
{
    type Provided = E::Provided;

    fn get_provided_value(widget: &E::ArcWidget) -> Arc<Self::Provided> {
        E::get_provided_value(widget)
    }
}
