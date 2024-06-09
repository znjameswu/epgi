use std::any::TypeId;

use crate::{
    foundation::{AnyRawPointer, ArrayContainer, Canvas, PaintContext, Protocol},
    tree::{
        ArcChildRenderObject, HitTestBehavior, HitTestContext, HitTestResult, RecordedChildLayer,
        Render, RenderImpl, RenderObject,
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

    /// The actual method that was invoked for hit-testing.
    ///
    /// Note however, this method is hard to impl directly. Therefore, if not for rare edge cases,
    /// it is recommended to implement [ProxyRender::hit_test_child], [ProxyRender::hit_test_self],
    /// and [ProxyRender::hit_test_behavior] instead. This method has a default impl that is composed on top of those method.
    ///
    /// If you do indeed overwrite the default impl of this method without using the other methods,
    /// you can assume the other methods mentioned above are `unreachable!()`.
    fn hit_test(
        &self,
        ctx: &mut HitTestContext<<Self::Protocol as Protocol>::Canvas>,
        size: &<Self::Protocol as Protocol>::Size,
        offset: &<Self::Protocol as Protocol>::Offset,
        child: &ArcChildRenderObject<Self::Protocol>,
    ) -> HitTestResult {
        use HitTestResult::*;
        let hit_self = self.hit_test_self(ctx.curr_position(), size, offset);
        if !hit_self {
            // Stop hit-test children if the hit is outside of parent
            return NotHit;
        }

        let hit_children = self.hit_test_child(ctx, size, offset, child);
        if hit_children {
            return Hit;
        }

        use HitTestBehavior::*;
        match self.hit_test_behavior() {
            DeferToChild => NotHit,
            Transparent => HitThroughSelf,
            Opaque => Hit,
        }
    }

    #[allow(unused_variables)]
    fn hit_test_child(
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
        _memo: &(),
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
    fn hit_test(
        render: &R,
        ctx: &mut HitTestContext<<R::Protocol as Protocol>::Canvas>,
        size: &<R::Protocol as Protocol>::Size,
        offset: &<R::Protocol as Protocol>::Offset,
        _memo: &(),
        [child]: &[ArcChildRenderObject<R::Protocol>; 1],
        adopted_children: &[RecordedChildLayer<<R::Protocol as Protocol>::Canvas>],
    ) -> HitTestResult {
        debug_assert!(
            adopted_children.is_empty(),
            "Proxy render does not take adoption"
        );
        R::hit_test(render, ctx, size, offset, child)
    }

    fn hit_test_children(
        _render: &R,
        _ctx: &mut HitTestContext<<R::Protocol as Protocol>::Canvas>,
        _size: &<R::Protocol as Protocol>::Size,
        _offset: &<R::Protocol as Protocol>::Offset,
        _memo: &(),
        [_child]: &[ArcChildRenderObject<R::Protocol>; 1],
        _adopted_children: &[RecordedChildLayer<<R::Protocol as Protocol>::Canvas>],
    ) -> bool {
        unreachable!(
            "TemplatePaint has already provided a hit_test implementation, \
            but hit_test_children is still invoked somehow. This indicates a framework bug."
        )
    }

    fn hit_test_self(
        _render: &R,
        _position: &<<R::Protocol as Protocol>::Canvas as Canvas>::HitPosition,
        _size: &<R::Protocol as Protocol>::Size,
        _offset: &<R::Protocol as Protocol>::Offset,
        _memo: &(),
    ) -> bool {
        unreachable!(
            "TemplatePaint has already provided a hit_test implementation, \
            but hit_test_self is still invoked somehow. This indicates a framework bug."
        )
    }

    fn hit_test_behavior(_render: &R) -> HitTestBehavior {
        unreachable!(
            "TemplatePaint has already provided a hit_test implementation, \
            but hit_test_behavior is still invoked somehow. This indicates a framework bug."
        )
    }

    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut RenderObject<R>) -> AnyRawPointer)]
    where
        R: Render,
    {
        <R as ProxyRender>::all_hit_test_interfaces()
    }
}
