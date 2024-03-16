use epgi_2d::{
    Affine2dCanvas, Affine2dPaintContextExt, BoxOffset, BoxProtocol, BoxSize, Color, FillPainter,
    Painter, Rect,
};
use epgi_core::{
    foundation::{Asc, PaintContext},
    nodes::{ProxyWidget, SingleChildRenderObjectElement},
    tree::{ArcChildRenderObject, ArcChildWidget, RenderAction, Widget},
};

#[derive(Debug, optargs::OptStructArc)]
pub struct ColorBox {
    pub color: Color,
    pub child: ArcChildWidget<BoxProtocol>,
}

impl Widget for ColorBox {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Element = SingleChildRenderObjectElement<Self>;

    fn into_arc_widget(self: Asc<Self>) -> Asc<Self> {
        self
    }
}

impl ProxyWidget for ColorBox {
    type Protocol = BoxProtocol;

    type RenderState = Color;

    fn child(&self) -> &ArcChildWidget<Self::Protocol> {
        &self.child
    }

    fn create_render_state(&self) -> Self::RenderState {
        self.color.clone()
    }

    fn update_render_state(&self, render_state: &mut Self::RenderState) -> RenderAction {
        if &self.color != render_state {
            *render_state = self.color;
            RenderAction::Repaint
        } else {
            RenderAction::None
        }
    }

    fn detach_render_state(_render_state: &mut Self::RenderState) {}

    const NOOP_DETACH: bool = true;

    type LayoutMemo = ();

    #[inline(never)]
    fn perform_paint(
        state: &Self::RenderState,
        size: &BoxSize,
        offset: &BoxOffset,
        _memo: &Self::LayoutMemo,
        child: &ArcChildRenderObject<Self::Protocol>,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        let color = state;
        paint_ctx.draw_rect(
            Rect::new_point_size(*offset, *size),
            Painter::Fill(FillPainter {
                fill: epgi_2d::Fill::EvenOdd,
                brush: epgi_2d::Brush::Solid(*color),
                transform: None,
            }),
        );
        paint_ctx.paint(child, offset);
    }

    type LayerOrUnit = ();
}
