use std::marker::PhantomData;

use epgi_core::{
    foundation::{Arc, ArrayContainer, BuildSuspendedError, InlinableDwsizeVec, Provide, TypeKey},
    template::{ImplByTemplate, TemplateElement, TemplateElementBase},
    tree::{
        ArcChildElementNode, ArcChildWidget, ArcWidget, BuildContext,
        ChildRenderObjectsUpdateCallback, ElementBase, ElementImpl, ElementReconcileItem,
        ImplElement,
    },
};

use crate::BoxProtocol;

pub struct BoxProxyElementTemplate<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool>(
    PhantomData<E>,
);

pub trait BoxProxyElement: Clone + Send + Sync + Sized + 'static {
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
        child: ArcChildElementNode<BoxProtocol>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<BoxProtocol>>,
    ) -> Result<
        ElementReconcileItem<BoxProtocol>,
        (ArcChildElementNode<BoxProtocol>, BuildSuspendedError),
    >;

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(Self, ArcChildWidget<BoxProtocol>), BuildSuspendedError>;
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateElementBase<E>
    for BoxProxyElementTemplate<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: BoxProxyElement,
{
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type ChildContainer = ArrayContainer<1>;

    type ArcWidget = E::ArcWidget;

    fn perform_rebuild_element(
        element: &mut E,
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        [child]: [ArcChildElementNode<BoxProtocol>; 1],
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<BoxProtocol>>,
    ) -> Result<
        (
            [ElementReconcileItem<BoxProtocol>; 1],
            Option<ChildRenderObjectsUpdateCallback<Self::ChildContainer, BoxProtocol>>,
        ),
        ([ArcChildElementNode<BoxProtocol>; 1], BuildSuspendedError),
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
    ) -> Result<(E, [ArcChildWidget<BoxProtocol>; 1]), BuildSuspendedError> {
        E::perform_inflate_element(widget, ctx, provider_values)
            .map(|(element, child_widget)| (element, [child_widget]))
    }
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateElement<E>
    for BoxProxyElementTemplate<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ElementBase,
    ElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>: ImplElement<Element = E>,
{
    type ElementImpl = ElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>;
}
