use epgi_core::{
    common::{
        create_root_element, ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, DryLayout,
        Element, ElementNode, GetSuspense, LayerScope, Reconciler, Render, RenderElement,
        RenderObject, Widget,
    },
    foundation::{
        Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Key, Never, PaintContext, Protocol,
        Provide,
    },
};
use vello::util::{RenderContext, RenderSurface};

use crate::{Affine2d, Affine2dCanvas, BoxProtocol};

pub struct RenderRootView {
    child: Option<ArcChildRenderObject<BoxProtocol>>,
}

impl Render for RenderRootView {
    type Element = RootViewElement;

    type ChildIter = Option<ArcChildRenderObject<BoxProtocol>>;

    fn children(&self) -> Self::ChildIter {
        todo!()
    }

    type LayoutMemo = ();

    fn perform_layout<'a, 'layout>(
        &'a self,
        _constraints: &'a <<Self::Element as Element>::ParentProtocol as Protocol>::Constraints,
    ) -> (
        <<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        Self::LayoutMemo,
    ) {
        unreachable!()
    }

    const PERFORM_DRY_LAYOUT: Option<epgi_core::common::PerformDryLayout<Self>> =
        Some(<Self as DryLayout>::PERFORM_DRY_LAYOUT);

    fn perform_paint(
        &self,
        _size: &<<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        _transform: &<<Self::Element as Element>::ParentProtocol as Protocol>::Transform,
        _memo: &Self::LayoutMemo,
        _paint_ctx: impl PaintContext<
            Canvas = <<Self::Element as Element>::ParentProtocol as Protocol>::Canvas,
        >,
    ) {
        unreachable!()
    }
}

impl DryLayout for RenderRootView {
    fn compute_dry_layout(
        &self,
        constraints: &<<Self::Element as Element>::ParentProtocol as Protocol>::Constraints,
    ) -> <<Self::Element as Element>::ParentProtocol as Protocol>::Size {
        todo!()
    }

    fn perform_layout<'a, 'layout>(
        &'a self,
        constraints: &'a <<Self::Element as Element>::ParentProtocol as Protocol>::Constraints,
        size: &'a <<Self::Element as Element>::ParentProtocol as Protocol>::Size,
    ) -> Self::LayoutMemo {
        // self.render_ctx.resize_surface(&mut self.surface, size.width, size.height)
    }
}

impl RenderRootView {
    pub fn render(&self) {}
}

#[derive(Debug)]
pub struct RootView {
    pub child: Option<ArcChildWidget<BoxProtocol>>,
}

impl Widget for RootView {
    type Element = RootViewElement;

    fn create_element(self: Asc<Self>) -> Self::Element {
        todo!()
    }

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as Element>::ArcWidget {
        todo!()
    }
}

#[derive(Clone)]
pub struct RootViewElement {
    pub child: Option<ArcChildElementNode<BoxProtocol>>,
}

impl Element for RootViewElement {
    type ArcWidget = Asc<RootView>;

    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Provided = Never;

    fn perform_rebuild_element(
        // Rational for a moving self: Allows users to destructure the self without needing to fill in a placeholder value.
        self,
        widget: &Self::ArcWidget,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        reconciler: impl Reconciler<Self::ChildProtocol>,
    ) -> Result<Self, (Self, BuildSuspendedError)> {
        todo!()
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        reconciler: impl Reconciler<Self::ChildProtocol>, // TODO: A specialized reconciler for inflate, to save passing &JobIds
    ) -> Result<Self, BuildSuspendedError> {
        todo!()
    }

    type ChildIter = Option<ArcChildElementNode<BoxProtocol>>;

    fn children(&self) -> Self::ChildIter {
        todo!()
    }

    type ArcRenderObject = Arc<RenderObject<RenderRootView>>;
}

impl RenderElement for RootViewElement {
    type Render = RenderRootView;

    fn try_create_render_object(
        &self,
        widget: &Self::ArcWidget,
    ) -> Option<Arc<RenderObject<Self::Render>>> {
        todo!()
    }

    fn update_render_object_widget(
        widget: &Self::ArcWidget,
        render_object: &Arc<RenderObject<Self::Render>>,
    ) {
        todo!()
    }

    fn try_update_render_object_children(
        &self,
        render_object: &Arc<RenderObject<Self::Render>>,
    ) -> Result<(), ()> {
        todo!()
    }

    fn detach_render_object(render_object: &Arc<RenderObject<Self::Render>>) {
        todo!()
    }

    const GET_SUSPENSE: Option<GetSuspense<Self>> = None;
}

