use epgi_2d::{Affine2d, Affine2dCanvas, BoxConstraints, BoxOffset, BoxProtocol, BoxSize};
use epgi_core::{
    foundation::{Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Provide},
    template::{
        AdapterRender, AdapterRenderTemplate, ImplByTemplate, SingleChildElement,
        SingleChildElementTemplate, SingleChildRenderElement,
    },
    tree::{ArcChildRenderObject, BuildContext, HitTestContext, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use super::{ArcRingRenderObject, ArcRingWidget, RingConstraints, RingOffset, RingProtocol};

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<BoxAdapterRing>))]
pub struct BoxAdapterRing {
    pub child: ArcRingWidget,
}

impl Widget for BoxAdapterRing {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = RingProtocol;
    type Element = BoxAdapterRingElement;

    fn into_arc_widget(self: Arc<Self>) -> Asc<BoxAdapterRing> {
        self
    }
}

#[derive(Clone, Debug)]
pub struct BoxAdapterRingElement {}

impl ImplByTemplate for BoxAdapterRingElement {
    type Template = SingleChildElementTemplate<true, false>;
}

impl SingleChildElement for BoxAdapterRingElement {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = RingProtocol;
    type ArcWidget = Asc<BoxAdapterRing>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcRingWidget, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {}
    }
}

impl SingleChildRenderElement for BoxAdapterRingElement {
    type Render = RenderBoxAdapterRing;

    fn create_render(&self, _widget: &Self::ArcWidget) -> Self::Render {
        RenderBoxAdapterRing {}
    }

    fn update_render(
        _render: &mut Self::Render,
        _widget: &Self::ArcWidget,
    ) -> Option<RenderAction> {
        None
    }

    const NOOP_UPDATE_RENDER_OBJECT: bool = true;
}

pub struct RenderBoxAdapterRing {}

impl ImplByTemplate for RenderBoxAdapterRing {
    type Template = AdapterRenderTemplate;
}

impl AdapterRender for RenderBoxAdapterRing {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = RingProtocol;
    type LayoutMemo = BoxOffset;

    fn perform_layout(
        &mut self,
        constraints: &BoxConstraints,
        child: &ArcRingRenderObject,
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
        &offset: &BoxOffset,
        &origin_offset: &BoxOffset,
        child: &ArcRingRenderObject,
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
        &offset: &BoxOffset,
        &origin_offset: &Self::LayoutMemo,
        child: &ArcRingRenderObject,
    ) -> bool {
        ctx.hit_test_with_paint_transform(
            child.clone(),
            &Affine2d::from_translation(&(offset + origin_offset)),
        )
    }

    fn compute_intrinsics(
        &mut self,
        child: &ArcChildRenderObject<Self::ChildProtocol>,
        intrinsics: &mut <Self::ParentProtocol as epgi_core::foundation::Protocol>::Intrinsics,
    ) {
        unimplemented!()
    }
}
