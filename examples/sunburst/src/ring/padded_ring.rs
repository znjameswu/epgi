use epgi_common::Lerp;
use epgi_core::{
    foundation::{set_if_changed, Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Provide},
    max,
    template::{
        ImplByTemplate, ShiftedRender, ShiftedRenderTemplate, SingleChildElement,
        SingleChildElementTemplate, SingleChildRenderElement,
    },
    tree::{BuildContext, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use super::{
    ArcRingRenderObject, ArcRingWidget, RingConstraints, RingIntrinsics, RingOffset, RingProtocol,
    RingSize,
};

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<PaddedRing>))]
pub struct PaddedRing {
    pub padding: RingEdgeInsets,
    pub child: ArcRingWidget,
}

#[derive(Lerp, PartialEq, Default, Clone, Copy, Debug)]
pub struct RingEdgeInsets {
    pub inner: f32,
    pub outer: f32,
    pub start: f32,
    pub end: f32,
}

impl RingEdgeInsets {
    pub fn new() -> Self {
        Self {
            inner: 0.0,
            outer: 0.0,
            start: 0.0,
            end: 0.0,
        }
    }

    pub fn inner(mut self, inner: f32) -> Self {
        self.inner = inner;
        self
    }
    pub fn outer(mut self, outer: f32) -> Self {
        self.outer = outer;
        self
    }
    pub fn start(mut self, start: f32) -> Self {
        self.start = start;
        self
    }
    pub fn end(mut self, end: f32) -> Self {
        self.end = end;
        self
    }

    pub fn new_all(value: f32) -> Self {
        Self {
            inner: value,
            outer: value,
            start: value,
            end: value,
        }
    }

    pub fn new_symmetric(radial: f32, angular: f32) -> Self {
        Self {
            inner: radial,
            outer: radial,
            start: angular,
            end: angular,
        }
    }
}

pub trait RingGeometryEdgeInsetsExt {
    fn deflate(&self, edges: RingEdgeInsets) -> Self;
    fn inflate(&self, edges: RingEdgeInsets) -> Self;
}

impl RingGeometryEdgeInsetsExt for RingConstraints {
    fn deflate(&self, edges: RingEdgeInsets) -> Self {
        let radial = edges.inner + edges.outer;
        let angular = edges.start + edges.end;
        RingConstraints {
            min_dr: self.min_dr - radial,
            max_dr: self.max_dr - radial,
            min_dtheta: self.min_dtheta - angular,
            max_dtheta: self.max_dtheta - angular,
        }
    }

    fn inflate(&self, edges: RingEdgeInsets) -> Self {
        let radial = edges.inner + edges.outer;
        let angular = edges.start + edges.end;
        RingConstraints {
            min_dr: self.min_dr + radial,
            max_dr: self.max_dr + radial,
            min_dtheta: self.min_dtheta + angular,
            max_dtheta: self.max_dtheta + angular,
        }
    }
}

impl RingGeometryEdgeInsetsExt for RingSize {
    fn deflate(&self, edges: RingEdgeInsets) -> Self {
        let radial = edges.inner + edges.outer;
        let angular = edges.start + edges.end;
        RingSize {
            dr: self.dr - radial,
            dtheta: self.dtheta - angular,
        }
    }

    fn inflate(&self, edges: RingEdgeInsets) -> Self {
        let radial = edges.inner + edges.outer;
        let angular = edges.start + edges.end;
        RingSize {
            dr: self.dr + radial,
            dtheta: self.dtheta + angular,
        }
    }
}

impl Widget for PaddedRing {
    type ParentProtocol = RingProtocol;
    type ChildProtocol = RingProtocol;
    type Element = PaddedRingElement;

    fn into_arc_widget(self: Asc<Self>) -> Asc<Self> {
        self
    }
}

#[derive(Clone)]
pub struct PaddedRingElement {}

impl ImplByTemplate for PaddedRingElement {
    type Template = SingleChildElementTemplate<true, false>;
}

impl SingleChildElement for PaddedRingElement {
    type ParentProtocol = RingProtocol;
    type ChildProtocol = RingProtocol;
    type ArcWidget = Asc<PaddedRing>;

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

impl SingleChildRenderElement for PaddedRingElement {
    type Render = RenderRingPadding;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderRingPadding {
            padding: widget.padding,
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        max!(set_if_changed(&mut render.padding, widget.padding).then_some(RenderAction::Relayout))
    }
}

pub struct RenderRingPadding {
    pub padding: RingEdgeInsets,
}

impl ImplByTemplate for RenderRingPadding {
    type Template = ShiftedRenderTemplate;
}

impl ShiftedRender for RenderRingPadding {
    type Protocol = RingProtocol;
    type LayoutMemo = ();

    fn get_child_offset(&self, _size: &RingSize, offset: &RingOffset, _memo: &()) -> RingOffset {
        *offset
            + RingOffset {
                r: self.padding.inner,
                theta: self.padding.start,
            }
    }

    fn perform_layout(
        &mut self,
        constraints: &RingConstraints,
        child: &ArcRingRenderObject,
    ) -> (RingSize, Self::LayoutMemo) {
        let child_size = child.layout_use_size(&constraints.deflate(self.padding));
        let size = constraints.constrain(child_size.inflate(self.padding));
        (size, ())
    }

    fn compute_intrinsics(&mut self, child: &ArcRingRenderObject, intrinsics: &mut RingIntrinsics) {
        child.get_intrinsics(intrinsics);
        unimplemented!();
    }
}
