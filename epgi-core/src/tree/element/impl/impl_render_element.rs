use crate::{
    foundation::{Arc, ArrayContainer},
    tree::{
        ArcChildElementNode, ArcChildRenderObject, ContainerOf, Element, Render, RenderAction,
        RenderObject, TreeNode,
    },
};

use super::ElementImpl;

pub trait HasRenderElementImpl<E: Element> {
    type Render: Render<
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
    fn update_render(render: &mut Self::Render, widget: &E::ArcWidget) -> RenderAction;

    /// Whether [Render::update_render_object] is a no-op and always returns None
    ///
    /// When set to true, [Render::update_render_object]'s implementation will be ignored,
    /// Certain optimizations to reduce mutex usages will be applied during the commit phase.
    /// However, if [Render::update_render_object] is actually not no-op, doing this will cause unexpected behaviors.
    ///
    /// Setting to false will always guarantee the correct behavior.
    const NOOP_UPDATE_RENDER_OBJECT: bool = false;
}

pub trait ImplElementNode<E: Element> {
    type OptionArcRenderObject: Default + Clone + Send + Sync;
    fn get_current_subtree_render_object(
        render_object: &Self::OptionArcRenderObject,
        children: &ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
    ) -> Option<ArcChildRenderObject<E::ParentProtocol>>;
}

impl<E: Element, const PROVIDE_ELEMENT: bool> ImplElementNode<E>
    for ElementImpl<E, false, PROVIDE_ELEMENT>
where
    E: TreeNode<
        ChildContainer = ArrayContainer<1>,
        ChildProtocol = <E as TreeNode>::ParentProtocol,
    >,
{
    type OptionArcRenderObject = ();

    fn get_current_subtree_render_object(
        _render_object: &(),
        [child]: &[ArcChildElementNode<E::ChildProtocol>; 1],
    ) -> Option<ArcChildRenderObject<E::ParentProtocol>> {
        child.get_current_subtree_render_object()
    }
}

impl<E: Element, const PROVIDE_ELEMENT: bool, R: Render> ImplElementNode<E>
    for ElementImpl<E, true, PROVIDE_ELEMENT>
where
    E::ElementImpl: HasRenderElementImpl<E, Render = R>,
    // This extra bound is necessary, because rust doesn't seem to understand type bounds for associated types
    R: Render<
        ParentProtocol = E::ParentProtocol,
        ChildProtocol = E::ChildProtocol,
        ChildContainer = E::ChildContainer,
    >,
{
    type OptionArcRenderObject = Option<Arc<RenderObject<R>>>;

    fn get_current_subtree_render_object(
        render_object: &Self::OptionArcRenderObject,
        _children: &ContainerOf<E, ArcChildElementNode<E::ChildProtocol>>,
    ) -> Option<ArcChildRenderObject<E::ParentProtocol>> {
        render_object
            .as_ref()
            .map(|render_object| render_object.clone() as _)
    }
}
