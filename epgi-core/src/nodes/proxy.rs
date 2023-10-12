use crate::foundation::{PaintContext, Protocol};

use crate::tree::{
    ArcChildWidget, ChildRenderObject, Element, PerformDryLayout, PerformLayerPaint,
    RenderObjectUpdateResult, Widget,
};

use super::{
    SingleChildRenderObject, SingleChildRenderObjectElement, SingleChildRenderObjectWidget,
};

pub trait ProxyWidget:
    Widget<
        Element = SingleChildRenderObjectElement<Self>,
        ParentProtocol = Self::Protocol,
        ChildProtocol = Self::Protocol
    > + Sized
{
    type Protocol: Protocol;
    type RenderState: Send + Sync;

    fn child(&self) -> &ArcChildWidget<Self::Protocol>;

    fn create_render_state(&self) -> Self::RenderState;

    fn update_render_state(&self, render_state: &mut Self::RenderState)
        -> RenderObjectUpdateResult;

    const NOOP_UPDATE_RENDER_OBJECT: bool = false;

    fn detach_render_state(render_state: &mut Self::RenderState);

    const NOOP_DETACH: bool = false;

    type LayoutMemo: Send + Sync + Default + 'static;

    #[inline(always)]
    fn perform_layout(
        _state: &Self::RenderState,
        child: &dyn ChildRenderObject<Self::Protocol>,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo) {
        let size = child.layout_use_size(constraints);
        (size, Default::default())
    }

    /// If this is not None, then [`Self::perform_layout`]'s implementation will be ignored.
    const PERFORM_DRY_LAYOUT: Option<PerformDryLayout<SingleChildRenderObject<Self>>> = None;

    // We don't make perform paint into an associated constant because it has an generic paramter
    // Then we have to go to associated generic type, which makes the boilerplate explodes.
    #[inline(always)]
    fn perform_paint(
        _state: &Self::RenderState,
        child: &dyn ChildRenderObject<Self::Protocol>,
        _size: &<Self::ParentProtocol as Protocol>::Size,
        transform: &<Self::ParentProtocol as Protocol>::Transform,
        _memo: &Self::LayoutMemo,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) {
        paint_ctx.paint(child, transform)
    }

    /// If this is not None, then [`Self::perform_paint`]'s implementation will be ignored.
    const PERFORM_LAYER_PAINT: Option<PerformLayerPaint<SingleChildRenderObject<Self>>> = None;
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
    fn update_render_state(
        &self,
        render_state: &mut Self::RenderState,
    ) -> RenderObjectUpdateResult {
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
        child: &dyn ChildRenderObject<Self::ChildProtocol>,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo) {
        T::perform_layout(state, child, constraints)
    }

    const PERFORM_DRY_LAYOUT: Option<PerformDryLayout<SingleChildRenderObject<Self>>> =
        T::PERFORM_DRY_LAYOUT;

    #[inline(always)]
    fn perform_paint(
        state: &Self::RenderState,
        child: &dyn ChildRenderObject<Self::ChildProtocol>,
        size: &<Self::ParentProtocol as Protocol>::Size,
        transform: &<Self::ParentProtocol as Protocol>::Transform,
        memo: &Self::LayoutMemo,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) {
        T::perform_paint(state, child, size, transform, memo, paint_ctx)
    }

    const PERFORM_LAYER_PAINT: Option<PerformLayerPaint<SingleChildRenderObject<Self>>> =
        T::PERFORM_LAYER_PAINT;
}
