use epgi_2d::{
    Affine2dCanvas, Affine2dPaintContextExt, BoxOffset, BoxProtocol, BoxSingleChildElement,
    BoxSingleChildElementTemplate, BoxSingleChildRenderElement, BoxSize, Brush, Color, Fill,
    FillPainter, Painter, Rect,
};
use epgi_core::{
    foundation::{Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Provide},
    template::{ImplByTemplate, ProxyRender, ProxyRenderTemplate},
    tree::{ArcChildRenderObject, ArcChildWidget, BuildContext, RenderAction, Widget},
};

#[derive(Debug, optargs::OptStructArc)]
pub struct ColorBox {
    pub color: Color,
    pub child: ArcChildWidget<BoxProtocol>,
}

impl Widget for ColorBox {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Element = ColorBoxElement;

    fn into_arc_widget(self: Asc<Self>) -> Asc<Self> {
        self
    }
}

#[derive(Clone)]
pub struct ColorBoxElement;

impl ImplByTemplate for ColorBoxElement {
    type Template = BoxSingleChildElementTemplate<true, false>;
}

impl BoxSingleChildElement for ColorBoxElement {
    type ArcWidget = Asc<ColorBox>;

    fn get_child_widget(
        element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        ctx: BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<BoxProtocol>, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(widget: &Self::ArcWidget) -> Self {
        Self
    }
}

impl BoxSingleChildRenderElement for ColorBoxElement {
    type Render = RenderColorBox;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderColorBox {
            color: widget.color.clone(),
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> RenderAction {
        if render.color != widget.color {
            *render.color = widget.color;
            RenderAction::Repaint
        } else {
            RenderAction::None
        }
    }
}

pub struct RenderColorBox {
    color: Color,
}

impl ImplByTemplate for RenderColorBox {
    type Template = ProxyRenderTemplate;
}

impl ProxyRender for RenderColorBox {
    type Protocol = BoxProtocol;

    fn perform_paint(
        &self,
        size: &BoxSize,
        offset: &BoxOffset,
        child: &ArcChildRenderObject<BoxProtocol>,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        paint_ctx.draw_rect(
            Rect::new_point_size(*offset, *size),
            Painter::Fill(FillPainter {
                fill: Fill::EvenOdd,
                brush: Brush::Solid(self.color),
                transform: None,
            }),
        );
        paint_ctx.paint(child, offset);
    }

    const NOOP_DETACH: bool = true;
}

// impl ProxyWidget for ColorBox {
//     type Protocol = BoxProtocol;

//     type RenderState = Color;

//     fn child(&self) -> &ArcChildWidget<Self::Protocol> {
//         &self.child
//     }

//     fn create_render_state(&self) -> Self::RenderState {
//         self.color.clone()
//     }

//     fn update_render_state(&self, render_state: &mut Self::RenderState) -> RenderAction {
//         if &self.color != render_state {
//             *render_state = self.color;
//             RenderAction::Repaint
//         } else {
//             RenderAction::None
//         }
//     }

//     fn detach_render_state(_render_state: &mut Self::RenderState) {}

//     const NOOP_DETACH: bool = true;

//     type LayoutMemo = ();

//     #[inline(never)]
//     fn perform_paint(
//         state: &Self::RenderState,
//         size: &BoxSize,
//         offset: &BoxOffset,
//         _memo: &Self::LayoutMemo,
//         child: &ArcChildRenderObject<Self::Protocol>,
//         paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
//     ) {
//         let color = state;
//         paint_ctx.draw_rect(
//             Rect::new_point_size(*offset, *size),
//             Painter::Fill(FillPainter {
//                 fill: epgi_2d::Fill::EvenOdd,
//                 brush: epgi_2d::Brush::Solid(*color),
//                 transform: None,
//             }),
//         );
//         paint_ctx.paint(child, offset);
//     }

//     type LayerOrUnit = ();
// }
