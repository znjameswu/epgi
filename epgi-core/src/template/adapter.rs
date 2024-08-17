use std::any::TypeId;

use crate::{
    foundation::{AnyRawPointer, ArrayContainer, Canvas, PaintContext, Protocol},
    tree::{
        ArcChildRenderObject, HitTestContext, HitTestResult, RecordedChildLayer, Render,
        RenderImpl, RenderObject,
    },
};

use super::{
    ImplByTemplate, TemplateHitTest, TemplateLayout, TemplatePaint, TemplateRender,
    TemplateRenderBase,
};

/// Adapter nodes stand for nodes that:
/// 1. Has exactly one child
/// 2. Does nothing other than translating between protocols during layout and paint
pub struct AdapterRenderTemplate;

pub trait AdapterRender: Send + Sync + Sized + 'static {
    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol<Canvas = <Self::ParentProtocol as Protocol>::Canvas>;
    type LayoutMemo: Send + Sync;

    fn perform_layout(
        &mut self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        child: &ArcChildRenderObject<Self::ChildProtocol>,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo);

    #[allow(unused_variables)]
    fn perform_paint(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        child: &ArcChildRenderObject<Self::ChildProtocol>,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    );

    /// The actual method that was invoked for hit-testing.
    ///
    /// Note however, this method is hard to impl directly. Therefore, if not for rare edge cases,
    /// it is recommended to implement [AdapterRender::hit_test_child], [AdapterRender::hit_test_self],
    /// and [AdapterRender::hit_test_behavior] instead. This method has a default impl that is composed on top of those method.
    ///
    /// If you do indeed overwrite the default impl of this method without using the other methods,
    /// you can assume the other methods mentioned above are `unreachable!()`.
    #[allow(unused_variables)]
    fn hit_test(
        &self,
        ctx: &mut HitTestContext<<Self::ParentProtocol as Protocol>::Canvas>,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        child: &ArcChildRenderObject<Self::ChildProtocol>,
    ) -> HitTestResult {
        use HitTestResult::*;
        let hit_in_bound =
            Self::ParentProtocol::position_in_shape(ctx.curr_position(), offset, size);
        if !hit_in_bound {
            return NotHit;
        }

        let hit_children = self.hit_test_child(ctx, size, offset, memo, child);
        if hit_children {
            return Hit;
        }
        // We have not hit any children. Now it up to us ourself.
        let hit_self = self.hit_test_self(ctx.curr_position(), size, offset, memo);
        return hit_self;
    }

    /// This method must be overridden if the two protocols across the adapter do not share the same origin point.
    #[allow(unused_variables)]
    fn hit_test_child(
        &self,
        ctx: &mut HitTestContext<<Self::ParentProtocol as Protocol>::Canvas>,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        child: &ArcChildRenderObject<Self::ChildProtocol>,
    ) -> bool {
        ctx.hit_test(child.clone())
    }

    #[allow(unused_variables)]
    fn hit_test_self(
        &self,
        position: &<<Self::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
    ) -> HitTestResult {
        HitTestResult::NotHit
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

impl<R> TemplateRenderBase<R> for AdapterRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: AdapterRender,
{
    type ParentProtocol = R::ParentProtocol;
    type ChildProtocol = R::ChildProtocol;
    type ChildContainer = ArrayContainer<1>;

    type LayoutMemo = R::LayoutMemo;

    fn detach(render: &mut R) {
        R::detach(render)
    }

    const NOOP_DETACH: bool = R::NOOP_DETACH;
}

impl<R> TemplateRender<R> for AdapterRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: AdapterRender,
{
    type RenderImpl = RenderImpl<false, false, false, false>;
}

impl<R> TemplateLayout<R> for AdapterRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: AdapterRender,
{
    fn perform_layout(
        render: &mut R,
        constraints: &<R::ParentProtocol as Protocol>::Constraints,
        [child]: &[ArcChildRenderObject<R::ChildProtocol>; 1],
    ) -> (<R::ParentProtocol as Protocol>::Size, R::LayoutMemo) {
        R::perform_layout(render, constraints, child)
    }
}

impl<R> TemplatePaint<R> for AdapterRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: AdapterRender,
{
    fn perform_paint(
        render: &R,
        size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        memo: &R::LayoutMemo,
        [child]: &[ArcChildRenderObject<R::ChildProtocol>; 1],
        paint_ctx: &mut impl PaintContext<Canvas = <R::ParentProtocol as Protocol>::Canvas>,
    ) {
        R::perform_paint(render, size, offset, memo, child, paint_ctx)
    }
}

impl<R> TemplateHitTest<R> for AdapterRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: AdapterRender,
{
    fn hit_test(
        render: &R,
        ctx: &mut HitTestContext<<R::ParentProtocol as Protocol>::Canvas>,
        size: &<R::ParentProtocol as Protocol>::Size,
        offset: &<R::ParentProtocol as Protocol>::Offset,
        memo: &R::LayoutMemo,
        [child]: &[ArcChildRenderObject<R::ChildProtocol>; 1],
        adopted_children: &[RecordedChildLayer<<R::ChildProtocol as Protocol>::Canvas>],
    ) -> HitTestResult {
        debug_assert!(
            adopted_children.is_empty(),
            "Adapter render does not take adoption"
        );
        R::hit_test(render, ctx, size, offset, memo, child)
    }

    fn hit_test_children(
        _render: &R,
        _ctx: &mut HitTestContext<<R::ParentProtocol as Protocol>::Canvas>,
        _size: &<R::ParentProtocol as Protocol>::Size,
        _offset: &<R::ParentProtocol as Protocol>::Offset,
        _memo: &R::LayoutMemo,
        [_child]: &[ArcChildRenderObject<R::ChildProtocol>; 1],
        _adopted_children: &[RecordedChildLayer<<R::ChildProtocol as Protocol>::Canvas>],
    ) -> bool {
        unreachable!(
            "TemplatePaint has already provided a hit_test implementation, \
            but hit_test_children is still invoked somehow. This indicates a framework bug."
        )
    }

    fn hit_test_self(
        _render: &R,
        _position: &<<R::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
        _size: &<R::ParentProtocol as Protocol>::Size,
        _offset: &<R::ParentProtocol as Protocol>::Offset,
        _memo: &R::LayoutMemo,
    ) -> HitTestResult {
        unreachable!(
            "TemplatePaint has already provided a hit_test implementation, \
            but hit_test_self is still invoked somehow. This indicates a framework bug."
        )
    }

    fn all_hit_test_interfaces() -> &'static [(TypeId, fn(*mut RenderObject<R>) -> AnyRawPointer)]
    where
        R: Render,
    {
        <R as AdapterRender>::all_hit_test_interfaces()
    }
}
