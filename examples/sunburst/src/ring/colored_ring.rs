use epgi_2d::{
    Affine2dCanvas, Affine2dPaintContextExt, Brush, Circle, Color, Fill, FillPainter, Painter,
    Point2d, RingSector,
};
use epgi_core::{
    foundation::{
        set_if_changed, Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Provide,
    },
    template::{
        ImplByTemplate, ProxyRender, ProxyRenderTemplate, SingleChildElement,
        SingleChildElementTemplate, SingleChildRenderElement,
    },
    tree::{ArcChildRenderObject, ArcChildWidget, BuildContext, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use super::{RingOffset, RingProtocol, RingSize};

#[derive(Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<ColoredRing>))]
#[derive(Debug)]
pub struct ColoredRing {
    pub color: Color,
    pub child: ArcChildWidget<RingProtocol>,
}

impl Widget for ColoredRing {
    type ParentProtocol = RingProtocol;
    type ChildProtocol = RingProtocol;
    type Element = ColoredRingElement;

    fn into_arc_widget(self: Asc<Self>) -> Asc<Self> {
        self
    }
}

#[derive(Clone)]
pub struct ColoredRingElement;

impl ImplByTemplate for ColoredRingElement {
    type Template = SingleChildElementTemplate<true, false>;
}

impl SingleChildElement for ColoredRingElement {
    type ParentProtocol = RingProtocol;
    type ChildProtocol = RingProtocol;
    type ArcWidget = Asc<ColoredRing>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<RingProtocol>, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self
    }
}

impl SingleChildRenderElement for ColoredRingElement {
    type Render = RenderColoredRing;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderColoredRing {
            color: widget.color.clone(),
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        set_if_changed(&mut render.color, widget.color).then_some(RenderAction::Repaint)
    }
}

pub struct RenderColoredRing {
    color: Color,
}

impl ImplByTemplate for RenderColoredRing {
    type Template = ProxyRenderTemplate;
}

impl ProxyRender for RenderColoredRing {
    type Protocol = RingProtocol;

    fn perform_paint(
        &self,
        size: &RingSize,
        offset: &RingOffset,
        child: &ArcChildRenderObject<RingProtocol>,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        paint_ctx.draw_ring_sector(
            RingSector {
                outer_cicle: Circle {
                    c: Point2d::ZERO,
                    r: offset.r + size.dr,
                },
                inner_radius: offset.r,
                start_angle: offset.theta,
                sweep_angle: size.dtheta,
            },
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
