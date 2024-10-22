use std::{
    f32::{consts::TAU, INFINITY},
    iter::zip,
    marker::PhantomData,
};

use epgi_2d::{
    Affine2dCanvas, Affine2dPaintContextExt, BlendMode, Circle, Point2d, RingSector,
    PRECISION_ERROR_TOLERANCE,
};
use epgi_common::{Axis, FlexRender, RenderFlex};
pub use epgi_common::{
    CrossAxisAlignment, Flexible, FlexibleConfig, MainAxisAlignment, MainAxisSize,
};
use epgi_core::{
    foundation::{
        set_if_changed, Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Provide,
    },
    template::{ImplByTemplate, MultiChildElement, MultiChildElementTemplate},
    tree::{BuildContext, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use super::{
    ArcRingRenderObject, ArcRingWidget, RingConstraints, RingOffset, RingProtocol, RingSize,
};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum RingAxis {
    Angular,
    Radial,
}

impl From<RingAxis> for Axis {
    fn from(value: RingAxis) -> Self {
        match value {
            RingAxis::Angular => Axis::Horizontal,
            RingAxis::Radial => Axis::Vertical,
        }
    }
}

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<FlexedRing>))]
pub struct FlexedRing {
    /// The direction to use as the main axis.
    pub direction: RingAxis,
    /// How the children should be placed along the main axis.
    #[builder(default = MainAxisAlignment::Start)]
    pub main_axis_alignment: MainAxisAlignment,
    /// How much space should be occupied in the main axis.
    ///
    /// After allocating space to children, there might be some remaining free
    /// space. This value controls whether to maximize or minimize the amount of
    /// free space, subject to the incoming layout constraints.
    ///
    /// If some children have a non-zero flex factors (and none have a fit of
    /// [FlexFit::Loose]), they will expand to consume all the available space and
    /// there will be no remaining free space to maximize or minimize, making this
    /// value irrelevant to the final layout.
    #[builder(default = MainAxisSize::Max)]
    pub main_axis_size: MainAxisSize,
    /// How the children should be placed along the cross axis.
    #[builder(default = CrossAxisAlignment::Center)]
    pub cross_axis_alignment: CrossAxisAlignment,
    #[builder(default = false)]
    pub flip_horizontal: bool,
    #[builder(default = false)]
    pub flip_vertical: bool,
    pub children: Vec<ArcRingWidget>,
}

impl Widget for FlexedRing {
    type ParentProtocol = RingProtocol;
    type ChildProtocol = RingProtocol;
    type Element = FlexedRingElement;

    fn into_arc_widget(self: Arc<Self>) -> Asc<Self> {
        self
    }
}

#[derive(Clone, Debug)]
pub struct FlexedRingElement {}

impl ImplByTemplate for FlexedRingElement {
    type Template = MultiChildElementTemplate<false>;
}

impl MultiChildElement for FlexedRingElement {
    type ParentProtocol = RingProtocol;
    type ChildProtocol = RingProtocol;
    type ArcWidget = Asc<FlexedRing>;
    type Render = RenderFlex<RingProtocol>;

    fn get_child_widgets(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<Vec<ArcRingWidget>, BuildSuspendedError> {
        Ok(widget.children.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {}
    }

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderFlex {
            direction: widget.direction.into(),
            main_axis_alignment: widget.main_axis_alignment,
            main_axis_size: widget.main_axis_size,
            cross_axis_alignment: widget.cross_axis_alignment,
            flip_main_axis: widget.flip_horizontal,
            flip_cross_axis: widget.flip_vertical,
            phantom: PhantomData,
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        [
            set_if_changed(&mut render.direction, widget.direction.into()),
            set_if_changed(&mut render.main_axis_alignment, widget.main_axis_alignment),
            set_if_changed(&mut render.main_axis_size, widget.main_axis_size),
            set_if_changed(
                &mut render.cross_axis_alignment,
                widget.cross_axis_alignment,
            ),
            set_if_changed(&mut render.flip_main_axis, widget.flip_horizontal),
            set_if_changed(&mut render.flip_cross_axis, widget.flip_vertical),
        ]
        .iter()
        .any(|&changed| changed)
        .then_some(RenderAction::Relayout)
    }
}

// pub struct RenderRingFlex {
//     pub direction: RingAxis,
//     pub main_axis_alignment: MainAxisAlignment,
//     pub main_axis_size: MainAxisSize,
//     pub cross_axis_alignment: CrossAxisAlignment,
//     pub flexible_configs: Vec<FlexibleConfig>,
//     pub flip_main_axis: bool,
//     pub flip_cross_axis: bool,
// }

impl FlexRender<RingProtocol> for RenderFlex<RingProtocol> {
    type CrossSize = f32;

    fn get_main_size(&self, size: &RingSize) -> f32 {
        match self.direction {
            Axis::Horizontal => size.dtheta,
            Axis::Vertical => size.dr,
        }
    }
    fn get_cross_size(&self, size: &RingSize) -> f32 {
        match self.direction {
            Axis::Horizontal => size.dr,
            Axis::Vertical => size.dtheta,
        }
    }

    fn get_max_main_size(&self, parent_constraints: &RingConstraints) -> f32 {
        self.get_main_size(&parent_constraints.biggest())
    }

    fn placeholder_size() -> RingSize {
        RingSize::ZERO
    }
    fn initial_cross_size(&self) -> Self::CrossSize {
        0.0
    }
    fn reduce_cross_size(&self, cross_size: &mut f32, child_cross_size: f32) {
        *cross_size = cross_size.max(child_cross_size);
    }

    fn child_constraints(
        &self,
        main_size_range: Option<(f32, f32)>,
        parent_constraints: &RingConstraints,
    ) -> RingConstraints {
        match self.direction {
            Axis::Horizontal => {
                let (min_dtheta, max_dtheta) = main_size_range.unwrap_or((0.0, TAU));
                let min_dr = if self.cross_axis_alignment != CrossAxisAlignment::Stretch {
                    0.0
                } else {
                    parent_constraints.max_dr
                };
                RingConstraints {
                    min_dr,
                    max_dr: parent_constraints.max_dr,
                    min_dtheta,
                    max_dtheta,
                }
            }
            Axis::Vertical => {
                let (min_dr, max_dr) = main_size_range.unwrap_or((0.0, INFINITY));
                let min_dtheta = if self.cross_axis_alignment != CrossAxisAlignment::Stretch {
                    0.0
                } else {
                    parent_constraints.max_dtheta
                };
                RingConstraints {
                    min_dr,
                    max_dr,
                    min_dtheta,
                    max_dtheta: parent_constraints.max_dtheta,
                }
            }
        }
    }

    fn constrain_size(
        &self,
        main_size: f32,
        cross_size: f32,
        parent_constraints: &RingConstraints,
    ) -> (RingSize, f32, f32) {
        match self.direction {
            Axis::Horizontal => {
                let size = parent_constraints.constrain(RingSize {
                    dr: cross_size,
                    dtheta: main_size,
                });
                (size, size.dtheta, size.dr)
            }
            Axis::Vertical => {
                let size = parent_constraints.constrain(RingSize {
                    dr: main_size,
                    dtheta: cross_size,
                });
                (size, size.dr, size.dtheta)
            }
        }
    }

    fn position_child(
        &self,
        main_offset: f32,
        cross_size: f32,
        child_size: &RingSize,
        _parent_constraints: &RingConstraints,
    ) -> RingOffset {
        let child_cross_position = match self.cross_axis_alignment {
            CrossAxisAlignment::Start | CrossAxisAlignment::End => {
                if !self.flip_cross_axis == (self.cross_axis_alignment == CrossAxisAlignment::Start)
                {
                    0.0
                } else {
                    cross_size - self.get_cross_size(child_size)
                }
            }
            CrossAxisAlignment::Center => (cross_size - self.get_cross_size(child_size)) / 2.0,
            CrossAxisAlignment::Stretch => 0.0,
        };
        let child_offset = match self.direction {
            Axis::Horizontal => RingOffset {
                r: child_cross_position,
                theta: main_offset,
            },
            Axis::Vertical => RingOffset {
                r: main_offset,
                theta: child_cross_position,
            },
        };
        child_offset
    }

    fn perform_paint(
        &self,
        size: &RingSize,
        &offset: &RingOffset,
        child_offsets: &Vec<RingOffset>,
        overflow: f32,
        children: &Vec<ArcRingRenderObject>,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        debug_assert_eq!(children.len(), child_offsets.len());
        if overflow < PRECISION_ERROR_TOLERANCE {
            for (&child_offset, child) in zip(child_offsets, children) {
                paint_ctx.paint(child, &(offset + child_offset));
            }
        } else {
            let ring_sector = RingSector {
                outer_cicle: Circle {
                    c: Point2d::ZERO,
                    r: offset.r + size.dr,
                },
                inner_radius: offset.r,
                start_angle: offset.theta,
                sweep_angle: size.dtheta,
            };
            paint_ctx.clip_ring_sector(ring_sector, BlendMode::default(), 1.0, |paint_ctx| {
                for (&child_offset, child) in zip(child_offsets, children) {
                    paint_ctx.paint(child, &(offset + child_offset));
                }
            });
            // todo!(paint overflow indicator)
        };
    }
}
