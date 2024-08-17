use epgi_2d::{Affine2d, Affine2dCanvas, BoxConstraints, BoxOffset, BoxProtocol, BoxSize};
use epgi_core::{
    foundation::{Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Provide},
    template::{
        AdapterRender, AdapterRenderTemplate, ImplByTemplate, SingleChildElement,
        SingleChildElementTemplate, SingleChildRenderElement,
    },
    tree::{
        ArcChildRenderObject, ArcChildWidget, BuildContext, HitTestContext, RenderAction, Widget,
    },
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use super::{RingConstraints, RingOffset, RingProtocol};

#[derive(Debug, Declarative, TypedBuilder)]
pub struct BoxRingAdapter {
    pub child: ArcChildWidget<RingProtocol>,
}

impl Widget for BoxRingAdapter {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = RingProtocol;
    type Element = BoxRingAdapterElement;

    fn into_arc_widget(self: Arc<Self>) -> Asc<BoxRingAdapter> {
        self
    }
}

#[derive(Clone, Debug)]
pub struct BoxRingAdapterElement {}

impl ImplByTemplate for BoxRingAdapterElement {
    type Template = SingleChildElementTemplate<true, false>;
}

impl SingleChildElement for BoxRingAdapterElement {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = RingProtocol;
    type ArcWidget = Asc<BoxRingAdapter>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<RingProtocol>, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {}
    }
}

impl SingleChildRenderElement for BoxRingAdapterElement {
    type Render = RenderBoxRingAdapter;

    fn create_render(&self, _widget: &Self::ArcWidget) -> Self::Render {
        RenderBoxRingAdapter {}
    }

    fn update_render(
        _render: &mut Self::Render,
        _widget: &Self::ArcWidget,
    ) -> Option<RenderAction> {
        None
    }

    const NOOP_UPDATE_RENDER_OBJECT: bool = true;
}

pub struct RenderBoxRingAdapter {}

impl ImplByTemplate for RenderBoxRingAdapter {
    type Template = AdapterRenderTemplate;
}

impl AdapterRender for RenderBoxRingAdapter {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = RingProtocol;
    type LayoutMemo = BoxOffset;

    fn perform_layout(
        &mut self,
        constraints: &BoxConstraints,
        child: &ArcChildRenderObject<RingProtocol>,
    ) -> (BoxSize, BoxOffset) {
        let max_size = constraints.biggest();
        let max_diameter = max_size.width.min(max_size.height);
        let max_r = max_diameter / 2.0;
        let child_size = child.layout_use_size(&RingConstraints {
            min_dr: 0.0,
            max_dr: max_r,
            min_dtheta: 0.0,
            max_dtheta: std::f32::consts::TAU,
        });
        let r = child_size.dr;
        let ideal_size = BoxSize {
            width: 2.0 * r,
            height: 2.0 * r,
        };
        let origin_offset = BoxOffset { x: r, y: r };
        return (ideal_size, origin_offset);
    }

    fn perform_paint(
        &self,
        _size: &BoxSize,
        offset: &BoxOffset,
        origin_offset: &BoxOffset,
        child: &ArcChildRenderObject<RingProtocol>,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        paint_ctx.with_transform(
            Affine2d::from_translation(&(offset + origin_offset)),
            |paint_ctx| {
                paint_ctx.paint(child, &RingOffset::default());
            },
        );
    }

    //
    fn hit_test_child(
        &self,
        ctx: &mut HitTestContext<Affine2dCanvas>,
        _size: &BoxSize,
        offset: &BoxOffset,
        origin_offset: &Self::LayoutMemo,
        child: &ArcChildRenderObject<RingProtocol>,
    ) -> bool {
        ctx.hit_test_with_paint_transform(
            child.clone(),
            &Affine2d::from_translation(&(offset + origin_offset)),
        )
    }
}
