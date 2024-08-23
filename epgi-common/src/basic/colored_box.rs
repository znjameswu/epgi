use epgi_2d::{
    Affine2dCanvas, Affine2dPaintContextExt, ArcBoxRenderObject, ArcBoxWidget, BoxOffset,
    BoxProtocol, BoxSingleChildElement, BoxSingleChildElementTemplate, BoxSingleChildRenderElement,
    BoxSize, Brush, Color, Fill, FillPainter, Painter, Rect,
};
use epgi_core::{
    foundation::{
        set_if_changed, Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Provide,
    },
    template::{ImplByTemplate, ProxyRender, ProxyRenderTemplate},
    tree::{BuildContext, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

#[derive(Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<ColoredBox>))]
#[derive(Debug)]
pub struct ColoredBox {
    pub color: Color,
    pub child: ArcBoxWidget,
}

impl Widget for ColoredBox {
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
    type ArcWidget = Asc<ColoredBox>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcBoxWidget, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
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

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        set_if_changed(&mut render.color, widget.color).then_some(RenderAction::Repaint)
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
        child: &ArcBoxRenderObject,
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
