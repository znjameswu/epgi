use std::any::TypeId;
use std::marker::PhantomData;

use crate::foundation::{
    AnyRawPointer, Arc, ArrayContainer, Asc, BuildSuspendedError, Canvas, InlinableDwsizeVec,
    Never, PaintContext, Protocol, Provide, TypeKey,
};

use crate::tree::{
    ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, BuildContext,
    ChildRenderObjectsUpdateCallback, ContainerOf, Element, ElementImpl, ElementReconcileItem,
    HasReconcileImpl, HitTestBehavior, HitTestResults, ImplElement, ImplElementBySuper, Render,
    RenderAction, RenderElement, Widget,
};

pub struct SingleChildElementImpl<
    E: Element,
    const RENDER_ELEMENT: bool,
    const PROVIDE_ELEMENT: bool,
>(PhantomData<E>);

impl<E: Element, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> ImplElementBySuper
    for SingleChildElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    ElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>: ImplElement<Element = E>,
{
    type Super = ElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>;
}

pub trait SingleChildReconcile: Element<ChildContainer = ArrayContainer<1>> {
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
        child: ArcChildElementNode<Self::ChildProtocol>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<Self::ChildProtocol>>,
    ) -> Result<
        ElementReconcileItem<Self::ChildProtocol>,
        (
            ArcChildElementNode<Self::ChildProtocol>,
            BuildSuspendedError,
        ),
    >;

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(Self, ArcChildWidget<Self::ChildProtocol>), BuildSuspendedError>;
}

impl<E: Element, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> HasReconcileImpl<E>
    for SingleChildElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: SingleChildReconcile,
{
    fn get_consumed_types(widget: &E::ArcWidget) -> &[TypeKey] {
        E::get_consumed_types(widget)
    }

    fn perform_rebuild_element(
        element: &mut E,
        widget: &E::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        [child]: [ArcChildElementNode<E::ChildProtocol>; 1],
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
        widget: &E::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(E, ContainerOf<E, ArcChildWidget<E::ChildProtocol>>), BuildSuspendedError> {
        E::perform_inflate_element(widget, ctx, provider_values)
            .map(|(element, child_widget)| (element, [child_widget]))
    }
}


pub trait SingleRenderElement: RenderElement<ChildContainer = ArrayContainer<1>> {}

