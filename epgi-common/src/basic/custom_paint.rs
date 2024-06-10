use std::{marker::PhantomData, sync::Arc};

use epgi_2d::{
    Affine2dCanvas, ArcBoxRenderObject, ArcBoxWidget, BoxOffset, BoxProtocol, BoxProxyRender,
    BoxProxyRenderTemplate, BoxSingleChildElement, BoxSingleChildElementTemplate,
    BoxSingleChildRenderElement, BoxSize, Point2d,
};
use epgi_core::{
    foundation::{
        set_if_changed, Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Provide,
    },
    template::ImplByTemplate,
    tree::{BuildContext, HitTestContext, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::PhantomBox;

#[derive(Debug, Declarative, TypedBuilder)]
pub struct CustomPaint<P: CustomPainter> {
    pub painter: P,
    #[cfg(feature = "box_intrinsics")]
    #[builder(default = BoxSize::ZERO)]
    pub size: BoxSize, // This field is for intrincis only
    #[builder(default = PhantomBox!())]
    pub child: ArcBoxWidget,
}

pub trait CustomPainter: Clone + std::fmt::Debug + Send + Sync + 'static {
    fn perform_paint_background(
        &self,
        size: &BoxSize,
        offset: &BoxOffset,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    );

    fn perform_paint_foreground(
        &self,
        size: &BoxSize,
        offset: &BoxOffset,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    );

    fn should_repaint(&self, other: &Self) -> bool;

    fn hit_test_background(
        &self,
        position: &Point2d,
        size: &BoxSize,
        offset: &BoxOffset,
    ) -> Option<bool>;

    fn hit_test_foreground(
        &self,
        position: &Point2d,
        size: &BoxSize,
        offset: &BoxOffset,
    ) -> Option<bool>;
}

impl<P: CustomPainter> Widget for CustomPaint<P> {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Element = CustomPaintElement<P>;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> Asc<CustomPaint<P>> {
        self
    }
}

#[derive(Clone, Debug)]
pub struct CustomPaintElement<P: CustomPainter> {
    phantom: PhantomData<P>,
}

impl<P: CustomPainter> ImplByTemplate for CustomPaintElement<P> {
    type Template = BoxSingleChildElementTemplate<true, false>;
}

impl<P: CustomPainter> BoxSingleChildElement for CustomPaintElement<P> {
    type ArcWidget = Asc<CustomPaint<P>>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcBoxWidget, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<P: CustomPainter> BoxSingleChildRenderElement for CustomPaintElement<P> {
    type Render = RenderCustomPaint<P>;

    fn create_render(&self, widget: &Self::ArcWidget) -> RenderCustomPaint<P> {
        RenderCustomPaint {
            painter: widget.painter.clone(),
        }
    }

    fn update_render(
        render: &mut RenderCustomPaint<P>,
        widget: &Self::ArcWidget,
    ) -> Option<RenderAction> {
        if render.painter.should_repaint(&widget.painter) {
            render.painter = widget.painter.clone();
            return Some(RenderAction::Repaint);
        }
        return None;
    }
}

pub struct RenderCustomPaint<P: CustomPainter> {
    pub painter: P,
}

impl<P: CustomPainter> ImplByTemplate for RenderCustomPaint<P> {
    type Template = BoxProxyRenderTemplate;
}

impl<P: CustomPainter> BoxProxyRender for RenderCustomPaint<P> {
    fn perform_paint(
        &self,
        size: &BoxSize,
        offset: &BoxOffset,
        child: &ArcBoxRenderObject,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        self.painter
            .perform_paint_background(size, offset, paint_ctx);
        paint_ctx.paint(child, offset);
        self.painter
            .perform_paint_background(size, offset, paint_ctx);
    }

    fn hit_test_child(
        &self,
        ctx: &mut HitTestContext<Affine2dCanvas>,
        size: &BoxSize,
        offset: &BoxOffset,
        child: &ArcBoxRenderObject,
    ) -> bool {
        if let Some(true) = self
            .painter
            .hit_test_foreground(ctx.curr_position(), size, offset)
        {
            return true;
        }
        ctx.hit_test(child.clone())
    }

    fn hit_test_self(&self, position: &Point2d, size: &BoxSize, offset: &BoxOffset) -> bool {
        self.painter
            .hit_test_background(position, size, offset)
            .unwrap_or(true)
    }
}
