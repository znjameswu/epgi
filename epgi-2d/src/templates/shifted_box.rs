use std::any::TypeId;

use epgi_core::{
    foundation::{AnyRawPointer, ArrayContainer, PaintContext, Protocol},
    template::{
        ImplByTemplate, TemplateHitTest, TemplateLayout, TemplatePaint, TemplateRender,
        TemplateRenderBase,
    },
    tree::{
        HitTestBehavior, HitTestContext, HitTestResult, RecordedChildLayer, Render, RenderImpl,
        RenderObject,
    },
};

use crate::{
    Affine2dCanvas, ArcBoxRenderObject, BoxConstraints, BoxOffset, BoxProtocol, BoxSize, Point2d,
};

pub struct ShiftedBoxRenderTemplate;

pub trait ShiftedBoxRender: Send + Sync + Sized + 'static {
    type LayoutMemo: Send + Sync;

    fn get_child_offset(
        &self,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &Self::LayoutMemo,
    ) -> BoxOffset;

    fn perform_layout(
        &mut self,
        constraints: &BoxConstraints,
        child: &ArcBoxRenderObject,
    ) -> (BoxSize, Self::LayoutMemo);

    #[allow(unused_variables)]
    fn perform_paint(
        &self,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &Self::LayoutMemo,
        child: &ArcBoxRenderObject,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        paint_ctx.paint(child, &self.get_child_offset(size, offset, memo))
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
        ctx: &mut HitTestContext<Affine2dCanvas>,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &Self::LayoutMemo,
        child: &ArcBoxRenderObject,
    ) -> HitTestResult {
        use HitTestResult::*;
        let hit_self = self.hit_test_self(ctx.curr_position(), size, offset, memo);
        if !hit_self {
            // Stop hit-test children if the hit is outside of parent
            return NotHit;
        }

        let hit_children = self.hit_test_child(ctx, size, offset, memo, child);
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
        ctx: &mut HitTestContext<Affine2dCanvas>,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &Self::LayoutMemo,
        child: &ArcBoxRenderObject,
    ) -> bool {
        ctx.hit_test(child.clone())
    }

    #[allow(unused_variables)]
    fn hit_test_self(
        &self,
        position: &Point2d,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &Self::LayoutMemo,
    ) -> bool {
        BoxProtocol::position_in_shape(position, offset, size)
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

impl<R> TemplateRenderBase<R> for ShiftedBoxRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: ShiftedBoxRender,
{
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type ChildContainer = ArrayContainer<1>;

    type LayoutMemo = R::LayoutMemo;

    fn detach(render: &mut R) {
        R::detach(render)
    }

    const NOOP_DETACH: bool = R::NOOP_DETACH;
}

impl<R> TemplateRender<R> for ShiftedBoxRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: ShiftedBoxRender,
{
    type RenderImpl = RenderImpl<false, false, false, false>;
}

impl<R> TemplateLayout<R> for ShiftedBoxRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: ShiftedBoxRender,
{
    fn perform_layout(
        render: &mut R,
        constraints: &BoxConstraints,
        [child]: &[ArcBoxRenderObject; 1],
    ) -> (BoxSize, R::LayoutMemo) {
        R::perform_layout(render, constraints, child)
    }
}

impl<R> TemplatePaint<R> for ShiftedBoxRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: ShiftedBoxRender,
{
    fn perform_paint(
        render: &R,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &R::LayoutMemo,
        [child]: &[ArcBoxRenderObject; 1],
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        R::perform_paint(render, size, offset, memo, child, paint_ctx)
    }
}

impl<R> TemplateHitTest<R> for ShiftedBoxRenderTemplate
where
    R: ImplByTemplate<Template = Self>,
    R: ShiftedBoxRender,
{
    fn hit_test(
        render: &R,
        ctx: &mut HitTestContext<Affine2dCanvas>,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &R::LayoutMemo,
        [child]: &[ArcBoxRenderObject; 1],
        adopted_children: &[RecordedChildLayer<Affine2dCanvas>],
    ) -> HitTestResult {
        debug_assert!(
            adopted_children.is_empty(),
            "Proxy render does not take adoption"
        );
        R::hit_test(render, ctx, size, offset, memo, child)
    }

    fn hit_test_children(
        _render: &R,
        _ctx: &mut HitTestContext<Affine2dCanvas>,
        _size: &BoxSize,
        _offset: &BoxOffset,
        _memo: &R::LayoutMemo,
        [_child]: &[ArcBoxRenderObject; 1],
        _adopted_children: &[RecordedChildLayer<Affine2dCanvas>],
    ) -> bool {
        unreachable!(
            "TemplatePaint has already provided a hit_test implementation, \
            but hit_test_children is still invoked somehow. This indicates a framework bug."
        )
    }

    fn hit_test_self(
        _render: &R,
        _position: &Point2d,
        _size: &BoxSize,
        _offset: &BoxOffset,
        _memo: &R::LayoutMemo,
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
        <R as ShiftedBoxRender>::all_hit_test_interfaces()
    }
}
