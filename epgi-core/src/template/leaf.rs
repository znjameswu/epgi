use std::any::TypeId;

use crate::{
    foundation::{
        AnyRawPointer, Arc, ArrayContainer, BuildSuspendedError, Canvas, InlinableDwsizeVec,
        PaintContext, Protocol, Provide,
    },
    tree::{
        ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, ArcWidget, BuildContext,
        ChildRenderObjectsUpdateCallback, ElementBase, ElementImpl, ElementReconcileItem,
        FullRender, HitTestBehavior, HitTestContext, RecordedChildLayer, Render, RenderAction,
        RenderImpl, RenderObject,
    },
};

use super::{
    ImplByTemplate, TemplateElement, TemplateElementBase, TemplateHitTest, TemplateLayout,
    TemplatePaint, TemplateRender, TemplateRenderBase, TemplateRenderElement,
};

pub struct LeafElementTemplate;

pub trait LeafElement: Clone + Send + Sync + Sized + 'static {
    type Protocol: Protocol;
    type ArcWidget: ArcWidget<Element = Self>;

    type Render: FullRender<
        ParentProtocol = Self::Protocol,
        ChildProtocol = Self::Protocol,
        ChildContainer = ArrayContainer<0>,
    >;

    #[allow(unused_variables)]
    fn update_element(
        &mut self,
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(), BuildSuspendedError> {
        Ok(())
    }

    fn create_element(
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<Self, BuildSuspendedError>;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render;
    /// Update necessary properties of render object given by the widget
    ///
    /// Called during the commit phase, when the widget is updated.
    /// Always called after [RenderElement::try_update_render_object_children].
    /// If that call failed to update children (indicating suspense), then this call will be skipped.
    #[allow(unused_variables)]
    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> RenderAction {
        RenderAction::None
    }

    /// Whether [Render::update_render_object] is a no-op and always returns None
    ///
    /// When set to true, [Render::update_render_object]'s implementation will be ignored,
    /// Certain optimizations to reduce mutex usages will be applied during the commit phase.
    /// However, if [Render::update_render_object] is actually not no-op, doing this will cause unexpected behaviors.
    ///
    /// Setting to false will always guarantee the correct behavior.
    const NOOP_UPDATE_RENDER_OBJECT: bool = false;
}

impl<E> TemplateElementBase<E> for LeafElementTemplate
where
    E: ImplByTemplate<Template = Self>,
    E: LeafElement,
{
    type ParentProtocol = E::Protocol;

    type ChildProtocol = E::Protocol;

    type ChildContainer = ArrayContainer<0>;

    type ArcWidget = E::ArcWidget;

    fn perform_rebuild_element(
        element: &mut E,
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        _children: [ArcChildElementNode<Self::ChildProtocol>; 0],
        _nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<Self::ChildProtocol>>,
    ) -> Result<
        (
            [ElementReconcileItem<Self::ChildProtocol>; 0],
            Option<ChildRenderObjectsUpdateCallback<Self::ChildContainer, Self::ChildProtocol>>,
        ),
        (
            [ArcChildElementNode<Self::ChildProtocol>; 0],
            BuildSuspendedError,
        ),
    > {
        E::update_element(element, widget, ctx, provider_values)
            .map(|_| ([], None))
            .map_err(|err| ([], err))
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: &mut BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<(E, [ArcChildWidget<Self::ChildProtocol>; 0]), BuildSuspendedError> {
        E::create_element(widget, ctx, provider_values).map(|element| (element, []))
    }
}

impl<E> TemplateElement<E> for LeafElementTemplate
where
    E: ImplByTemplate<Template = Self>,
    E: LeafElement,
{
    // Must have a render. Pointless to provide
    type Impl = ElementImpl<true, false>;
}

impl<E> TemplateRenderElement<E> for LeafElementTemplate
where
    E: ImplByTemplate<Template = Self>,
    E: LeafElement,
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

pub struct LeafRenderTemplate;

pub trait LeafRender: Send + Sync + Sized + 'static {
    type Protocol: Protocol;

    fn perform_layout(
        &mut self,
        constraints: &<Self::Protocol as Protocol>::Constraints,
    ) -> <Self::Protocol as Protocol>::Size;

    #[allow(unused_variables)]
    fn perform_paint(
        &self,
        size: &<Self::Protocol as Protocol>::Size,
        offset: &<Self::Protocol as Protocol>::Offset,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::Protocol as Protocol>::Canvas>,
    );

    #[allow(unused_variables)]
    fn hit_test_self(
        &self,
        position: &<<Self::Protocol as Protocol>::Canvas as Canvas>::HitPosition,
        size: &<Self::Protocol as Protocol>::Size,
        offset: &<Self::Protocol as Protocol>::Offset,
    ) -> bool {
        Self::Protocol::position_in_shape(position, offset, size)
    }

    fn hit_test_behavior(&self) -> HitTestBehavior {
        HitTestBehavior::DeferToChild
    }

    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut RenderObject<Self>) -> AnyRawPointer)]
    where
        Self: Render,
    {
        &[]
    }

    fn detach(&mut self) {}
    const NOOP_DETACH: bool = false;
}

impl<R> TemplateRenderBase<R> for LeafRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: LeafRender,
{
    type ParentProtocol = R::Protocol;
    type ChildProtocol = R::Protocol;
    type ChildContainer = ArrayContainer<0>;

    type LayoutMemo = ();

    fn detach(render: &mut R) {
        R::detach(render)
    }

    const NOOP_DETACH: bool = R::NOOP_DETACH;
}

impl<R> TemplateRender<R> for LeafRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: LeafRender,
{
    // Pointless to be layout-by-parent
    type RenderImpl = RenderImpl<false, false, false, false>;
}

impl<R> TemplateLayout<R> for LeafRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: LeafRender,
{
    fn perform_layout(
        render: &mut R,
        constraints: &<R::Protocol as Protocol>::Constraints,
        _children: &[ArcChildRenderObject<R::Protocol>; 0],
    ) -> (<R::Protocol as Protocol>::Size, ()) {
        let size = R::perform_layout(render, constraints);
        (size, ())
    }
}

impl<R> TemplatePaint<R> for LeafRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: LeafRender,
{
    fn perform_paint(
        render: &R,
        size: &<R::Protocol as Protocol>::Size,
        offset: &<R::Protocol as Protocol>::Offset,
        _memo: &(),
        _children: &[ArcChildRenderObject<R::Protocol>; 0],
        paint_ctx: &mut impl PaintContext<Canvas = <R::Protocol as Protocol>::Canvas>,
    ) {
        R::perform_paint(render, size, offset, paint_ctx)
    }
}

impl<R> TemplateHitTest<R> for LeafRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: LeafRender,
{
    fn hit_test_children(
        _render: &R,
        _ctx: &mut HitTestContext<<R::Protocol as Protocol>::Canvas>,
        _size: &<R::Protocol as Protocol>::Size,
        _offset: &<R::Protocol as Protocol>::Offset,
        _memo: &(),
        _children: &[ArcChildRenderObject<R::Protocol>; 0],
        _adopted_children: &[RecordedChildLayer<<R::Protocol as Protocol>::Canvas>],
    ) -> bool {
        false
    }

    fn hit_test_self(
        render: &R,
        position: &<<R::Protocol as Protocol>::Canvas as Canvas>::HitPosition,
        size: &<R::Protocol as Protocol>::Size,
        offset: &<R::Protocol as Protocol>::Offset,
        _memo: &(),
    ) -> bool {
        R::hit_test_self(render, position, size, offset)
    }

    fn hit_test_behavior(render: &R) -> HitTestBehavior {
        R::hit_test_behavior(render)
    }

    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut RenderObject<R>) -> AnyRawPointer)]
    where
        R: Render,
    {
        <R as LeafRender>::all_hit_test_interfaces()
    }
}
