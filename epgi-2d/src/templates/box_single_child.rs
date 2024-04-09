use std::marker::PhantomData;

use epgi_core::{
    foundation::{Arc, ArrayContainer, BuildSuspendedError, InlinableDwsizeVec, Provide, TypeKey},
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

use crate::BoxProtocol;

pub struct BoxSingleChildElementTemplate<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool>(
    PhantomData<E>,
);

pub trait BoxSingleChildElement: Clone + Send + Sync + Sized + 'static {
    type ArcWidget: ArcWidget<Element = Self>;

    #[allow(unused_variables)]
    fn get_consumed_types(widget: &Self::ArcWidget) -> &[TypeKey] {
        &[]
    }

    fn get_child_widget(
        element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<BoxProtocol>, BuildSuspendedError>;

    /// A major limitation to the single child element template is that,
    /// we cannot provide consumed values and build context during the creation the Element itself.
    /// On top of that, since you can no longer access hooks when creating the Element itself,
    /// it also becomes impossible to suspend safely during the process, hence the "must-succeed" signature.
    /// We expect most people does not need provider or hooks during this process.
    /// If you do need, you can always perform relevant operations in the parent and pass it down in widget.
    fn create_element(widget: &Self::ArcWidget) -> Self;
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateElementBase<E>
    for BoxSingleChildElementTemplate<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: BoxSingleChildElement,
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
        let child_widget = match E::get_child_widget(Some(element), widget, ctx, provider_values) {
            Err(error) => return Err(([child], error)),
            Ok(child_wdiget) => child_wdiget,
        };
        let item = match child.can_rebuild_with(child_widget) {
            Ok(item) => item,
            Err((child, child_widget)) => {
                nodes_needing_unmount.push(child);
                ElementReconcileItem::new_inflate(child_widget)
            }
        };
        Ok(([item], None))
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(E, [ArcChildWidget<BoxProtocol>; 1]), BuildSuspendedError> {
        let element = E::create_element(widget);
        let child_widget = E::get_child_widget(None, widget, ctx, provider_values)?;
        Ok((element, [child_widget]))
    }
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateElement<E>
    for BoxSingleChildElementTemplate<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ElementBase,
    ElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>: ImplElement<Element = E>,
{
    type ElementImpl = ElementImpl<E, RENDER_ELEMENT, PROVIDE_ELEMENT>;
}

pub trait BoxSingleChildRenderElement: BoxSingleChildElement {
    type Render: Render<
        ParentProtocol = BoxProtocol,
        ChildProtocol = BoxProtocol,
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
    for BoxSingleChildElementTemplate<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: BoxSingleChildRenderElement,
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

pub trait BoxSingleChildProvideElement: BoxSingleChildElement {
    type Provided: Provide;
    fn get_provided_value(widget: &Self::ArcWidget) -> Arc<Self::Provided>;
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateProvideElement<E>
    for BoxSingleChildElementTemplate<E, RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: BoxSingleChildProvideElement,
{
    type Provided = E::Provided;

    fn get_provided_value(widget: &<E as ElementBase>::ArcWidget) -> Arc<Self::Provided> {
        E::get_provided_value(widget)
    }
}
