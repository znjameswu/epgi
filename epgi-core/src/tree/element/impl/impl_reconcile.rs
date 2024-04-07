use crate::{
    foundation::{Arc, BuildSuspendedError, InlinableDwsizeVec, Provide, TypeKey},
    tree::{
        ArcChildElementNode, ArcChildWidget, BuildContext, ChildRenderObjectsUpdateCallback,
        ContainerOf, Element, ElementReconcileItem,
    },
};

pub trait HasReconcileImpl<E: Element> {
    fn get_consumed_types(widget: &E::ArcWidget) -> &[TypeKey];

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
    >;

    fn perform_inflate_element(
        widget: &E::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(E, ContainerOf<E, ArcChildWidget<E::ChildProtocol>>), BuildSuspendedError>;
}
