use std::any::TypeId;

use crate::{
    foundation::{AnyRawPointer, ArrayContainer, Canvas, PaintContext, Protocol},
    tree::{
        ArcChildRenderObject, ComposableChildLayer, HitTestBehavior, HitTestContext, Render,
        RenderImpl, RenderObject,
    },
};

use super::{
    ImplByTemplate, TemplateHitTest, TemplateLayout, TemplatePaint, TemplateRender,
    TemplateRenderBase,
};

/// Proxy nodes stand for nodes that:
/// 1. Has exactly one child
/// 2. Has the same parent protocol and child protocol
pub struct ProxyRenderTemplate;

pub trait ProxyRender: Send + Sync + Sized + 'static {
    type Protocol: Protocol;

    fn perform_layout(
        &mut self,
        constraints: &<Self::Protocol as Protocol>::Constraints,
        child: &ArcChildRenderObject<Self::Protocol>,
    ) -> <Self::Protocol as Protocol>::Size {
        child.layout_use_size(constraints)
    }

    #[allow(unused_variables)]
    fn perform_paint(
        &self,
        size: &<Self::Protocol as Protocol>::Size,
        offset: &<Self::Protocol as Protocol>::Offset,
        child: &ArcChildRenderObject<Self::Protocol>,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::Protocol as Protocol>::Canvas>,
    ) {
        paint_ctx.paint(child, offset)
    }

    #[allow(unused_variables)]
    fn hit_test_children(
        &self,
        ctx: &mut HitTestContext<<Self::Protocol as Protocol>::Canvas>,
        size: &<Self::Protocol as Protocol>::Size,
        offset: &<Self::Protocol as Protocol>::Offset,
        child: &ArcChildRenderObject<Self::Protocol>,
    ) -> bool {
        ctx.hit_test(child.clone())
    }

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

impl<R> TemplateRenderBase<R> for ProxyRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: ProxyRender,
{
    type ParentProtocol = R::Protocol;
    type ChildProtocol = R::Protocol;
    type ChildContainer = ArrayContainer<1>;

    type LayoutMemo = ();

    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut RenderObject<R>) -> AnyRawPointer)]
    where
        R: Render,
    {
        <R as ProxyRender>::all_hit_test_interfaces()
    }

    fn detach(render: &mut R) {
        R::detach(render)
    }

    const NOOP_DETACH: bool = R::NOOP_DETACH;
}

impl<R> TemplateRender<R> for ProxyRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: ProxyRender,
{
    type RenderImpl = RenderImpl<false, false, false, false>;
}

impl<R> TemplateLayout<R> for ProxyRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: ProxyRender,
{
    fn perform_layout(
        render: &mut R,
        constraints: &<R::Protocol as Protocol>::Constraints,
        [child]: &[ArcChildRenderObject<R::Protocol>; 1],
    ) -> (<R::Protocol as Protocol>::Size, ()) {
        let size = R::perform_layout(render, constraints, child);
        (size, ())
    }
}

impl<R> TemplatePaint<R> for ProxyRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: ProxyRender,
{
    fn perform_paint(
        render: &R,
        size: &<R::Protocol as Protocol>::Size,
        offset: &<R::Protocol as Protocol>::Offset,
        memo: &(),
        [child]: &[ArcChildRenderObject<R::Protocol>; 1],
        paint_ctx: &mut impl PaintContext<Canvas = <R::Protocol as Protocol>::Canvas>,
    ) {
        R::perform_paint(render, size, offset, child, paint_ctx)
    }
}

impl<R> TemplateHitTest<R> for ProxyRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: ProxyRender,
{
    fn hit_test_children(
        render: &R,
        ctx: &mut HitTestContext<<R::Protocol as Protocol>::Canvas>,
        size: &<R::Protocol as Protocol>::Size,
        offset: &<R::Protocol as Protocol>::Offset,
        memo: &(),
        [child]: &[ArcChildRenderObject<R::Protocol>; 1],
        adopted_children: &[ComposableChildLayer<<R::Protocol as Protocol>::Canvas>],
    ) -> bool {
        R::hit_test_children(render, ctx, size, offset, child)
    }

    fn hit_test_self(
        render: &R,
        position: &<<R::Protocol as Protocol>::Canvas as Canvas>::HitPosition,
        size: &<R::Protocol as Protocol>::Size,
        offset: &<R::Protocol as Protocol>::Offset,
        memo: &(),
    ) -> bool {
        R::hit_test_self(render, position, size, offset)
    }

    fn hit_test_behavior(render: &R) -> HitTestBehavior {
        R::hit_test_behavior(render)
    }
}
