use std::any::TypeId;

use crate::foundation::{AnyRawPointer, Canvas, PaintContext, Protocol};

use crate::tree::{
    ArcChildRenderObject, ArcChildWidget, DryLayoutFunctionTable, HitTestBehavior, HitTestResults,
    LayerOrUnit, RenderAction, RenderObject, Widget,
};

use super::{
    SingleChildRenderObject, SingleChildRenderObjectElement, SingleChildRenderObjectWidget,
};

/// Apart from having a single child and a RenderObject, the proxy widget does not alter the protocol.
pub trait ProxyWidget:
    Widget<
        Element = SingleChildRenderObjectElement<Self>,
        ParentProtocol = Self::Protocol,
        ChildProtocol = Self::Protocol,
    > + Sized
{
    type Protocol: Protocol;
    type RenderState: Send + Sync;

    fn child(&self) -> &ArcChildWidget<Self::Protocol>;

    fn create_render_state(&self) -> Self::RenderState;

    fn update_render_state(&self, render_state: &mut Self::RenderState) -> RenderAction;

    const NOOP_UPDATE_RENDER_OBJECT: bool = false;

    fn detach_render_state(render_state: &mut Self::RenderState);

    const NOOP_DETACH: bool = false;

    type LayoutMemo: Send + Sync + Default + 'static;

    #[inline(always)]
    #[allow(unused_variables)]
    fn perform_layout(
        state: &Self::RenderState,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        child: &ArcChildRenderObject<Self::Protocol>,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo) {
        let size = child.layout_use_size(constraints);
        (size, Default::default())
    }

    /// If this is not None, then [`Self::perform_layout`]'s implementation will be ignored.
    const PERFORM_DRY_LAYOUT: Option<DryLayoutFunctionTable<SingleChildRenderObject<Self>>> = None;

    // We don't make perform paint into an associated constant because it has an generic paramter
    // Then we have to go to associated generic type, which makes the boilerplate explodes.
    #[inline(always)]
    #[allow(unused_variables)]
    fn perform_paint(
        state: &Self::RenderState,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        child: &ArcChildRenderObject<Self::Protocol>,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) {
        paint_ctx.paint(child, offset)
    }

    #[inline(always)]
    #[allow(unused_variables)]
    fn hit_test_children(
        state: &Self::RenderState,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        child: &ArcChildRenderObject<Self::ChildProtocol>,
        results: &mut HitTestResults<<Self::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        results.hit_test(child.clone())
    }

    fn hit_test_self(
        state: &Self::RenderState,
        position: &<<Self::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
    ) -> Option<HitTestBehavior> {
        <Self::ParentProtocol as Protocol>::position_in_shape(position, offset, size)
            .then_some(HitTestBehavior::DeferToChild)
    }

    type LayerOrUnit: LayerOrUnit<SingleChildRenderObject<Self>>;

    fn all_hit_test_interfaces() -> &'static [(
        TypeId,
        fn(*mut RenderObject<SingleChildRenderObject<Self>>) -> AnyRawPointer,
    )] {
        &[]
    }
}

impl<T> SingleChildRenderObjectWidget for T
where
    T: ProxyWidget,
{
    type RenderState = T::RenderState;

    #[inline(always)]
    fn child(&self) -> &ArcChildWidget<Self::ChildProtocol> {
        T::child(self)
    }

    #[inline(always)]
    fn create_render_state(&self) -> Self::RenderState {
        T::create_render_state(self)
    }

    #[inline(always)]
    fn update_render_state(&self, render_state: &mut Self::RenderState) -> RenderAction {
        T::update_render_state(self, render_state)
    }
    const NOOP_UPDATE_RENDER_OBJECT: bool = T::NOOP_UPDATE_RENDER_OBJECT;

    #[inline(always)]
    fn detach_render_state(render_state: &mut Self::RenderState) {
        T::detach_render_state(render_state)
    }
    const NOOP_DETACH: bool = T::NOOP_DETACH;

    type LayoutMemo = T::LayoutMemo;

    #[inline(always)]
    fn perform_layout(
        state: &Self::RenderState,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        child: &ArcChildRenderObject<Self::ChildProtocol>,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo) {
        T::perform_layout(state, constraints, child)
    }

    const DRY_LAYOUT_FUNCTION_TABLE: Option<DryLayoutFunctionTable<SingleChildRenderObject<Self>>> =
        T::PERFORM_DRY_LAYOUT;

    #[inline(always)]
    fn perform_paint(
        state: &Self::RenderState,
        size: &<Self::ParentProtocol as Protocol>::Size,
        transform: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        child: &ArcChildRenderObject<Self::ChildProtocol>,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) {
        T::perform_paint(state, size, transform, memo, child, paint_ctx)
    }

    fn hit_test_children(
        state: &Self::RenderState,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
        child: &ArcChildRenderObject<Self::ChildProtocol>,
        results: &mut HitTestResults<<Self::ParentProtocol as Protocol>::Canvas>,
    ) -> bool {
        T::hit_test_children(state, size, offset, memo, child, results)
    }

    fn hit_test_self(
        state: &Self::RenderState,
        position: &<<Self::ParentProtocol as Protocol>::Canvas as Canvas>::HitPosition,
        size: &<Self::ParentProtocol as Protocol>::Size,
        offset: &<Self::ParentProtocol as Protocol>::Offset,
        memo: &Self::LayoutMemo,
    ) -> Option<HitTestBehavior> {
        T::hit_test_self(state, position, size, offset, memo)
    }

    type LayerOrUnit = T::LayerOrUnit;

    fn all_hit_test_interfaces() -> &'static [(
        TypeId,
        fn(*mut RenderObject<SingleChildRenderObject<Self>>) -> AnyRawPointer,
    )] {
        T::all_hit_test_interfaces()
    }
}
