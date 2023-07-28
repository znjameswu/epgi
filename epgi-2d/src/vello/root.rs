use epgi_core::{
    common::{
        ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, DryLayout, Element, GetSuspense,
        LayoutExecutor, PerformLayout, Reconciler, Render, RenderElement, RenderObject, Widget,
    },
    foundation::{
        Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Key, Never, PaintingContext, Protocol,
        Provide,
    },
};
use vello::util::{RenderContext, RenderSurface};

use crate::BoxProtocol;

pub struct RenderRootView {
    render_ctx: RenderContext,
    surface: RenderSurface,
    child: ArcChildRenderObject<BoxProtocol>,
}

impl Render for RenderRootView {
    type Element = RootViewElement;

    type ChildIter = [ArcChildRenderObject<BoxProtocol>; 1];

    fn get_children(&self) -> Self::ChildIter {
        todo!()
    }

    fn set_children(&mut self, new_children: Self::ChildIter) {
        todo!()
    }

    type LayoutMemo = ();

    const PERFORM_LAYOUT: PerformLayout<Self> = <Self as DryLayout>::PERFORM_LAYOUT;

    fn perform_paint(
        &self,
        size: &<<Self::Element as Element>::SelfProtocol as Protocol>::Size,
        transformation: &<<Self::Element as Element>::SelfProtocol as Protocol>::CanvasTransformation,
        memo: &Self::LayoutMemo,
        paint_ctx: &mut impl PaintingContext<
            Canvas = <<Self::Element as Element>::SelfProtocol as Protocol>::Canvas,
        >,
    ) {
        // self.child.paint(transformation, paint_ctx)
        //todo!()
    }
}

impl DryLayout for RenderRootView {
    fn compute_dry_layout(
        &self,
        constraints: &<<Self::Element as Element>::SelfProtocol as Protocol>::Constraints,
    ) -> <<Self::Element as Element>::SelfProtocol as Protocol>::Size {
        todo!()
    }

    fn perform_layout<'a, 'layout>(
        &'a self,
        constraints: &'a <<Self::Element as Element>::SelfProtocol as Protocol>::Constraints,
        size: &'a <<Self::Element as Element>::SelfProtocol as Protocol>::Size,
        executor: LayoutExecutor<'a, 'layout>,
    ) -> Self::LayoutMemo {
        // self.render_ctx.resize_surface(&mut self.surface, size.width, size.height)
    }
}

impl RenderRootView {
    pub fn render(&self) {}
}

#[derive(Debug)]
pub struct RootView {
    child: ArcChildWidget<BoxProtocol>,
}

impl Widget for RootView {
    type Element = RootViewElement;

    fn key(&self) -> &dyn Key {
        todo!()
    }

    fn create_element(self: Asc<Self>) -> Self::Element {
        todo!()
    }

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as Element>::ArcWidget {
        todo!()
    }
}

#[derive(Clone)]
pub struct RootViewElement {}

impl Element for RootViewElement {
    type ArcWidget = Asc<RootView>;

    type SelfProtocol = BoxProtocol;

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

    type ChildIter = [ArcChildElementNode<BoxProtocol>; 1];

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
