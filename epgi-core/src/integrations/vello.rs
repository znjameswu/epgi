use vello::util::{RenderContext, RenderSurface};

use crate::{
    common::{
        ArcChildRenderObject, Element, LayoutExecutor, PerformLayout, Render, RootViewElement, DryLayout,
    },
    foundation::{BoxProtocol, Protocol}, rendering::PaintingContext,
};

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
        paint_ctx: &mut impl PaintingContext<<<Self::Element as Element>::SelfProtocol as Protocol>::Canvas>,
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
