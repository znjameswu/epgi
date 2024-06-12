use std::{marker::PhantomData, sync::Arc};

use epgi_2d::{
    Affine2dCanvas, ArcBoxRenderObject, ArcBoxWidget, BoxOffset, BoxProtocol, BoxProxyRender,
    BoxProxyRenderTemplate, BoxSingleChildElement, BoxSingleChildElementTemplate,
    BoxSingleChildRenderElement, BoxSize, Point2d,
};
use epgi_core::{
    foundation::{Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Provide},
    template::ImplByTemplate,
    tree::{BuildContext, HitTestContext, HitTestResult, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::PhantomBox;

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<CustomPaint<B, F>>))]
pub struct CustomPaint<B: CustomPainter, F: CustomPainter = ()> {
    pub painter: B,
    pub foreground_painter: F,
    #[cfg(feature = "box_intrinsics")]
    #[builder(default = BoxSize::ZERO)]
    pub size: BoxSize, // This field is for intrincis only
    #[builder(default = PhantomBox!())]
    pub child: ArcBoxWidget,
}

pub trait CustomPainter: Clone + std::fmt::Debug + Send + Sync + 'static {
    #[allow(unused_variables)]
    fn perform_paint(
        &self,
        size: &BoxSize,
        offset: &BoxOffset,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
    }

    fn should_repaint(&self, other: &Self) -> bool;

    #[allow(unused_variables)]
    fn hit_test(
        &self,
        position: &Point2d,
        size: &BoxSize,
        offset: &BoxOffset,
    ) -> Option<HitTestResult> {
        None
    }
}

impl CustomPainter for () {
    fn should_repaint(&self, _other: &Self) -> bool {
        false
    }
}

impl<B: CustomPainter, F: CustomPainter> Widget for CustomPaint<B, F> {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Element = CustomPaintElement<B, F>;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> Asc<CustomPaint<B, F>> {
        self
    }
}

#[derive(Clone, Debug)]
pub struct CustomPaintElement<B: CustomPainter, F: CustomPainter> {
    phantom: PhantomData<(F, B)>,
}

impl<B: CustomPainter, F: CustomPainter> ImplByTemplate for CustomPaintElement<B, F> {
    type Template = BoxSingleChildElementTemplate<true, false>;
}

impl<B: CustomPainter, F: CustomPainter> BoxSingleChildElement for CustomPaintElement<B, F> {
    type ArcWidget = Asc<CustomPaint<B, F>>;

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

impl<B: CustomPainter, F: CustomPainter> BoxSingleChildRenderElement for CustomPaintElement<B, F> {
    type Render = RenderCustomPaint<B, F>;

    fn create_render(&self, widget: &Self::ArcWidget) -> RenderCustomPaint<B, F> {
        RenderCustomPaint {
            painter: widget.painter.clone(),
            foreground_painter: widget.foreground_painter.clone(),
        }
    }

    fn update_render(
        render: &mut RenderCustomPaint<B, F>,
        widget: &Self::ArcWidget,
    ) -> Option<RenderAction> {
        if render.painter.should_repaint(&widget.painter) {
            render.painter = widget.painter.clone();
            return Some(RenderAction::Repaint);
        }
        return None;
    }
}

pub struct RenderCustomPaint<B: CustomPainter, F: CustomPainter> {
    pub painter: B,
    pub foreground_painter: F,
}

impl<B: CustomPainter, F: CustomPainter> ImplByTemplate for RenderCustomPaint<B, F> {
    type Template = BoxProxyRenderTemplate;
}

impl<B: CustomPainter, F: CustomPainter> BoxProxyRender for RenderCustomPaint<B, F> {
    fn perform_paint(
        &self,
        size: &BoxSize,
        offset: &BoxOffset,
        child: &ArcBoxRenderObject,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        self.painter.perform_paint(size, offset, paint_ctx);
        paint_ctx.paint(child, offset);
        self.foreground_painter
            .perform_paint(size, offset, paint_ctx);
    }

    fn hit_test_child(
        &self,
        ctx: &mut HitTestContext<Affine2dCanvas>,
        size: &BoxSize,
        offset: &BoxOffset,
        child: &ArcBoxRenderObject,
    ) -> bool {
        let foreground_hit = self
            .foreground_painter
            .hit_test(ctx.curr_position(), size, offset)
            .unwrap_or(HitTestResult::NotHit);
        if foreground_hit == HitTestResult::Hit {
            return true;
        }
        let children_hit = ctx.hit_test(child.clone());
        return children_hit || foreground_hit == HitTestResult::HitThroughSelf;
    }

    fn hit_test_self(
        &self,
        position: &Point2d,
        size: &BoxSize,
        offset: &BoxOffset,
    ) -> HitTestResult {
        self.painter
            .hit_test(position, size, offset)
            .unwrap_or(HitTestResult::Hit)
    }
}
