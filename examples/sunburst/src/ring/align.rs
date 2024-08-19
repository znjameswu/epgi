use epgi_common::Lerp;
use epgi_core::{
    foundation::{set_if_changed, Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Provide},
    max,
    template::{
        ImplByTemplate, ShiftedRender, ShiftedRenderTemplate, SingleChildElement,
        SingleChildElementTemplate, SingleChildRenderElement,
    },
    tree::{ArcChildWidget, BuildContext, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use super::{
    ArcRingRenderObject, ArcRingWidget, RingConstraints, RingOffset, RingProtocol, RingSize,
};

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<RingAlign>))]
pub struct RingAlign {
    pub alignment: RingAlignment,
    #[builder(default)]
    pub radial_factor: Option<f32>,
    #[builder(default)]
    pub angular_factor: Option<f32>,
    pub child: ArcRingWidget,
}

#[derive(Lerp, PartialEq, Clone, Copy, Debug)]
pub struct RingAlignment {
    pub radial: f32,
    pub angular: f32,
}

impl RingAlignment {
    pub const INNER_START: Self = Self {
        radial: -1.0,
        angular: -1.0,
    };
    pub const INNER_CENTER: Self = Self {
        radial: -1.0,
        angular: 0.0,
    };
    pub const INNER_END: Self = Self {
        radial: -1.0,
        angular: 1.0,
    };
    pub const CENTER_START: Self = Self {
        radial: 0.0,
        angular: -1.0,
    };
    pub const CENTER: Self = Self {
        radial: 0.0,
        angular: 0.0,
    };
    pub const CENTER_END: Self = Self {
        radial: 0.0,
        angular: 1.0,
    };
    pub const OUTER_START: Self = Self {
        radial: 1.0,
        angular: -1.0,
    };
    pub const OUTER_CENTER: Self = Self {
        radial: 1.0,
        angular: 0.0,
    };
    pub const OUTER_END: Self = Self {
        radial: 1.0,
        angular: 1.0,
    };

    pub fn along_offset(&self, offset: RingOffset) -> RingOffset {
        let center_r = offset.r / 2.0;
        let center_theta = offset.theta / 2.0;
        return RingOffset {
            r: center_r + center_r * self.radial,
            theta: center_theta + center_theta * self.angular,
        };
    }
}

impl Widget for RingAlign {
    type ParentProtocol = RingProtocol;
    type ChildProtocol = RingProtocol;
    type Element = RingAlignElement;

    fn into_arc_widget(self: Asc<Self>) -> Asc<Self> {
        self
    }
}

#[derive(Clone)]
pub struct RingAlignElement {}

impl ImplByTemplate for RingAlignElement {
    type Template = SingleChildElementTemplate<true, false>;
}

impl SingleChildElement for RingAlignElement {
    type ParentProtocol = RingProtocol;
    type ChildProtocol = RingProtocol;
    type ArcWidget = Asc<RingAlign>;

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

impl SingleChildRenderElement for RingAlignElement {
    type Render = RenderPositionedRing;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderPositionedRing {
            alignment: widget.alignment,
            dr_factor: widget.radial_factor,
            dtheta_factor: widget.angular_factor,
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        max!(
            set_if_changed(&mut render.alignment, widget.alignment)
                .then_some(RenderAction::Relayout),
            set_if_changed(&mut render.dr_factor, widget.radial_factor)
                .then_some(RenderAction::Relayout),
            set_if_changed(&mut render.dtheta_factor, widget.angular_factor)
                .then_some(RenderAction::Relayout),
        )
    }
}

pub struct RenderPositionedRing {
    pub alignment: RingAlignment,
    pub dr_factor: Option<f32>,
    pub dtheta_factor: Option<f32>,
}

impl ImplByTemplate for RenderPositionedRing {
    type Template = ShiftedRenderTemplate;
}

impl ShiftedRender for RenderPositionedRing {
    type Protocol = RingProtocol;
    type LayoutMemo = RingOffset;

    fn get_child_offset(
        &self,
        _size: &RingSize,
        &offset: &RingOffset,
        &child_extra_offset: &RingOffset,
    ) -> RingOffset {
        offset + child_extra_offset
    }

    fn perform_layout(
        &mut self,
        constraints: &RingConstraints,
        child: &ArcRingRenderObject,
    ) -> (RingSize, Self::LayoutMemo) {
        let shrink_warp_dr = self.dr_factor.is_some() || constraints.max_dr == f32::INFINITY;
        let shrink_warp_dtheta =
            self.dtheta_factor.is_some() || constraints.max_dtheta == f32::INFINITY;

        let child_size = child.layout_use_size(&constraints.loosen());

        let size = constraints.constrain(RingSize {
            dr: if shrink_warp_dr {
                child_size.dr * self.dr_factor.unwrap_or(1.0)
            } else {
                f32::INFINITY
            },
            dtheta: if shrink_warp_dtheta {
                child_size.dtheta * self.dtheta_factor.unwrap_or(1.0)
            } else {
                f32::INFINITY
            },
        });

        let child_extra_offset = self.alignment.along_offset(RingOffset {
            r: size.dr - child_size.dr,
            theta: size.dtheta - child_size.dtheta,
        });
        (size, child_extra_offset)
    }
}
