use epgi_2d::{
    Affine2dCanvas, ArcBoxRenderObject, ArcBoxWidget, BoxConstraints, BoxOffset, BoxProxyRender,
    BoxProxyRenderTemplate, BoxSingleChildDryLayout, BoxSingleChildLayout, BoxSingleChildPaint,
    BoxSingleChildRender, BoxSingleChildRenderTemplate, BoxSize,
};
use epgi_core::{
    foundation::PaintContext,
    template::ImplByTemplate,
    tree::{HitTestContext, HitTestResult},
};

pub struct Align {
    pub alignment: Alignment,
    pub width_factor: Option<f32>,
    pub height_factor: Option<f32>,
    pub child: ArcBoxWidget,
}

pub struct Alignment {
    pub x: f32,
    pub y: f32,
}

impl Alignment {
    pub fn along_offset(&self, offset: BoxOffset) -> BoxOffset {
        let center_x = offset.x / 2.0;
        let center_y = offset.y / 2.0;
        return BoxOffset {
            x: center_x + center_x * self.x,
            y: center_y + center_y * self.y,
        };
    }
}

pub struct AlignElement {}

pub struct RenderPositionedBox {
    pub alignment: Alignment,
    pub width_factor: Option<f32>,
    pub height_factor: Option<f32>,
}

impl ImplByTemplate for RenderPositionedBox {
    type Template = BoxSingleChildRenderTemplate<false, false, false, false>;
}

impl BoxSingleChildRender for RenderPositionedBox {
    type LayoutMemo = BoxOffset;

    const NOOP_DETACH: bool = true;
}

impl BoxSingleChildLayout for RenderPositionedBox {
    fn perform_layout(
        &mut self,
        constraints: &BoxConstraints,
        child: &ArcBoxRenderObject,
    ) -> (BoxSize, Self::LayoutMemo) {
        let shrink_warp_width =
            self.width_factor.is_some() || constraints.max_width == f32::INFINITY;
        let shrink_warp_height =
            self.height_factor.is_some() || constraints.max_height == f32::INFINITY;

        let child_size = child.layout_use_size(&constraints.loosen());

        let size = constraints.constrain(BoxSize {
            width: if shrink_warp_width {
                child_size.width * self.width_factor.unwrap_or(1.0)
            } else {
                f32::INFINITY
            },
            height: if shrink_warp_height {
                child_size.height * self.height_factor.unwrap_or(1.0)
            } else {
                f32::INFINITY
            },
        });

        let child_offset = self.alignment.along_offset(BoxOffset {
            x: size.width - child_size.width,
            y: size.height - child_size.height,
        });
        (size, child_offset)
    }
}

impl BoxSingleChildPaint for RenderPositionedBox {
    fn perform_paint(
        &self,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &Self::LayoutMemo,
        child: &ArcBoxRenderObject,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        todo!()
    }
}
