use std::borrow::Cow;

use crate::{
    foundation::{
        Arc, BuildSuspendedError, InlinableDwsizeVec, Protocol, Provide, TypeKey, VecContainer,
        EMPTY_CONSUMED_TYPES,
    },
    tree::{
        default_reconcile_vec, ArcChildElementNode, ArcChildWidget, ArcWidget, BuildContext,
        ChildRenderObjectsUpdateCallback, ElementBase, ElementImpl, ElementReconcileItem,
        FullRender, ImplElement, RenderAction,
    },
};

use super::{
    ImplByTemplate, TemplateElement, TemplateElementBase, TemplateProvideElement,
    TemplateRenderElement,
};

pub struct MultiChildElementTemplate<const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool>;

pub trait MultiChildElement: Clone + Send + Sync + Sized + 'static {
    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol;

    type ArcWidget: ArcWidget<Element = Self>;

    #[allow(unused_variables)]
    fn get_consumed_types(widget: &Self::ArcWidget) -> Cow<[TypeKey]> {
        EMPTY_CONSUMED_TYPES.into()
    }

    fn get_child_widgets(
        element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<Vec<ArcChildWidget<Self::ChildProtocol>>, BuildSuspendedError>;

    /// A major limitation to the Multi child element template is that,
    /// we cannot provide consumed values and build context during the creation the Element itself.
    /// On top of that, since you can no longer access hooks when creating the Element itself,
    /// it also becomes impossible to suspend safely during the process, hence the "must-succeed" signature.
    /// We expect most people does not need provider or hooks during this process.
    /// If you do need, you can always perform relevant operations in the parent and pass it down in widget.
    fn create_element(widget: &Self::ArcWidget) -> Self;
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateElementBase<E>
    for MultiChildElementTemplate<RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: MultiChildElement,
{
    type ParentProtocol = E::ParentProtocol;
    type ChildProtocol = E::ChildProtocol;
    type ChildContainer = VecContainer;

    type ArcWidget = E::ArcWidget;

    fn get_consumed_types(widget: &Self::ArcWidget) -> Cow<[TypeKey]> {
        E::get_consumed_types(widget)
    }

    fn perform_rebuild_element(
        element: &mut E,
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        children: Vec<ArcChildElementNode<E::ChildProtocol>>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<E::ChildProtocol>>,
    ) -> Result<
        (
            Vec<ElementReconcileItem<E::ChildProtocol>>,
            Option<ChildRenderObjectsUpdateCallback<Self::ChildContainer, E::ChildProtocol>>,
        ),
        (
            Vec<ArcChildElementNode<E::ChildProtocol>>,
            BuildSuspendedError,
        ),
    > {
        let new_widgets = match E::get_child_widgets(Some(element), widget, ctx, provider_values) {
            Err(error) => return Err((children, error)),
            Ok(new_widgets) => new_widgets,
        };
        Ok(default_reconcile_vec(
            children,
            new_widgets,
            nodes_needing_unmount,
        ))
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(E, Vec<ArcChildWidget<E::ChildProtocol>>), BuildSuspendedError> {
        let element = E::create_element(widget);
        let child_widgets = E::get_child_widgets(None, widget, ctx, provider_values)?;
        Ok((element, child_widgets))
    }
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateElement<E>
    for MultiChildElementTemplate<RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ElementBase,
    ElementImpl<RENDER_ELEMENT, PROVIDE_ELEMENT>: ImplElement<E>,
{
    type Impl = ElementImpl<RENDER_ELEMENT, PROVIDE_ELEMENT>;
}

pub trait MultiChildRenderElement: MultiChildElement {
    type Render: FullRender<
        ParentProtocol = Self::ParentProtocol,
        ChildProtocol = Self::ChildProtocol,
        ChildContainer = VecContainer,
    >;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render;
    /// Update necessary properties of render object given by the widget
    ///
    /// Called during the commit phase, when the widget is updated.
    /// Always called after [RenderElement::try_update_render_object_children].
    /// If that call failed to update children (indicating suspense), then this call will be skipped.
    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction>;

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
    for MultiChildElementTemplate<RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: MultiChildRenderElement,
{
    type Render = E::Render;

    fn create_render(element: &E, widget: &<E as ElementBase>::ArcWidget) -> Self::Render {
        E::create_render(element, widget)
    }

    fn update_render(
        render: &mut Self::Render,
        widget: &<E as ElementBase>::ArcWidget,
    ) -> Option<RenderAction> {
        E::update_render(render, widget)
    }

    const NOOP_UPDATE_RENDER_OBJECT: bool = E::NOOP_UPDATE_RENDER_OBJECT;
}

pub trait MultiChildProvideElement: MultiChildElement {
    type Provided: Provide;
    fn get_provided_value(widget: &Self::ArcWidget) -> Arc<Self::Provided>;
}

impl<E, const RENDER_ELEMENT: bool, const PROVIDE_ELEMENT: bool> TemplateProvideElement<E>
    for MultiChildElementTemplate<RENDER_ELEMENT, PROVIDE_ELEMENT>
where
    E: ImplByTemplate<Template = Self>,
    E: MultiChildProvideElement,
{
    type Provided = E::Provided;

    fn get_provided_value(widget: &<E as ElementBase>::ArcWidget) -> Arc<Self::Provided> {
        E::get_provided_value(widget)
    }
}
