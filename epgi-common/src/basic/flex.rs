use std::iter::zip;

use epgi_2d::{
    Affine2dCanvas, Affine2dPaintContextExt, ArcBoxRenderObject, ArcBoxWidget, BlendMode,
    BoxConstraints, BoxMultiChildElement, BoxMultiChildElementTemplate, BoxMultiChildHitTest,
    BoxMultiChildLayout, BoxMultiChildPaint, BoxMultiChildRender, BoxMultiChildRenderElement,
    BoxMultiChildRenderTemplate, BoxOffset, BoxProtocol, BoxSize, PRECISION_ERROR_TOLERANCE,
};
use epgi_core::{
    foundation::{
        set_if_changed, Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Protocol,
        Provide,
    },
    template::ImplByTemplate,
    tree::{ArcChildWidget, BuildContext, ChildWidget, ElementBase, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Axis {
    Horizontal,
    Veritcal,
}

/// How the children should be placed along the main axis in a flex layout.
///
/// See also:
///
///  * [Column], [Row], and [Flex], the flex widgets.
///  * [RenderFlex], the flex render object.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum MainAxisAlignment {
    /// Place the children as close to the start of the main axis as possible.
    ///
    /// If this value is used in a horizontal direction, a [TextDirection] must be
    /// available to determine if the start is the left or the right.
    ///
    /// If this value is used in a vertical direction, a [VerticalDirection] must be
    /// available to determine if the start is the top or the bottom.
    Start,

    /// Place the children as close to the end of the main axis as possible.
    ///
    /// If this value is used in a horizontal direction, a [TextDirection] must be
    /// available to determine if the end is the left or the right.
    ///
    /// If this value is used in a vertical direction, a [VerticalDirection] must be
    /// available to determine if the end is the top or the bottom.
    End,

    /// Place the children as close to the middle of the main axis as possible.
    Center,

    /// Place the free space evenly between the children.
    SpaceBetween,

    /// Place the free space evenly between the children as well as half of that
    /// space before and after the first and last child.
    SpaceAround,

    /// Place the free space evenly between the children as well as before and
    /// after the first and last child.
    SpaceEvenly,
}

/// How the children should be placed along the cross axis in a flex layout.
///
/// See also:
///
///  * [Column], [Row], and [Flex], the flex widgets.
///  * [Flex.crossAxisAlignment], the property on flex widgets that
///    has this type.
///  * [RenderFlex], the flex render object.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum CrossAxisAlignment {
    /// Place the children with their start edge aligned with the start side of
    /// the cross axis.
    ///
    /// For example, in a column (a flex with a vertical axis) whose
    /// [TextDirection] is [TextDirection.ltr], this aligns the left edge of the
    /// children along the left edge of the column.
    ///
    /// If this value is used in a horizontal direction, a [TextDirection] must be
    /// available to determine if the start is the left or the right.
    ///
    /// If this value is used in a vertical direction, a [VerticalDirection] must be
    /// available to determine if the start is the top or the bottom.
    Start,

    /// Place the children as close to the end of the cross axis as possible.
    ///
    /// For example, in a column (a flex with a vertical axis) whose
    /// [TextDirection] is [TextDirection.ltr], this aligns the right edge of the
    /// children along the right edge of the column.
    ///
    /// If this value is used in a horizontal direction, a [TextDirection] must be
    /// available to determine if the end is the left or the right.
    ///
    /// If this value is used in a vertical direction, a [VerticalDirection] must be
    /// available to determine if the end is the top or the bottom.
    End,

    /// Place the children so that their centers align with the middle of the
    /// cross axis.
    ///
    /// This is the default cross-axis alignment.
    Center,

    /// Require the children to fill the cross axis.
    ///
    /// This causes the constraints passed to the children to be tight in the
    /// cross axis.
    Stretch,
    // /// Place the children along the cross axis such that their baselines match.
    // ///
    // /// Consider using this value for any horizontal main axis (as with [Row])
    // /// where the children primarily contain text.  If the different children
    // /// have text with different font metrics (for example because they differ
    // /// in [TextStyle.fontSize] or other [TextStyle] properties, or because
    // /// they use different fonts due to being written in different scripts),
    // /// then this typically produces better visual alignment than the other
    // /// [CrossAxisAlignment] values, which use no information about
    // /// where the text sits vertically within its bounding box.
    // ///
    // /// The baseline of a widget is typically the typographic baseline of the
    // /// first text in the first [Text] or [RichText] widget it encloses, if any.
    // /// The typographic baseline is a horizontal line used for aligning text,
    // /// which is specified by each font; for alphabetic scripts, it ordinarily
    // /// runs along the bottom of letters excluding any descenders.
    // ///
    // /// Because baselines are always horizontal, this alignment is intended for
    // /// horizontal main axes (as with [Row]). If the main axis is vertical
    // /// (as with [Column]), then this value is treated like [start].
    // ///
    // /// For horizontal main axes, if the minimum height constraint passed to the
    // /// flex layout exceeds the intrinsic height of the cross axis, children will
    // /// be aligned as close to the top as they can be while honoring the baseline
    // /// alignment. In other words, the extra space will be below all the children.
    // ///
    // /// Children who report no baseline will be top-aligned.
    // ///
    // /// See also:
    // ///
    // ///  * [RenderBox.getDistanceToBaseline], which defines the baseline of a box.
    // Baseline,
}

/// How much space should be occupied in the main axis.
///
/// During a flex layout, available space along the main axis is allocated to
/// children. After allocating space, there might be some remaining free space.
/// This value controls whether to maximize or minimize the amount of free
/// space, subject to the incoming layout constraints.
///
/// See also:
///
///  * [Column], [Row], and [Flex], the flex widgets.
///  * [Expanded] and [Flexible], the widgets that controls a flex widgets'
///    children's flex.
///  * [RenderFlex], the flex render object.
///  * [MainAxisAlignment], which controls how the free space is distributed.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum MainAxisSize {
    /// Minimize the amount of free space along the main axis, subject to the
    /// incoming layout constraints.
    ///
    /// If the incoming layout constraints have a large enough
    /// [BoxConstraints.minWidth] or [BoxConstraints.minHeight], there might still
    /// be a non-zero amount of free space.
    ///
    /// If the incoming layout constraints are unbounded, and any children have a
    /// non-zero [FlexParentData.flex] and a [FlexFit.tight] fit (as applied by
    /// [Expanded]), the [RenderFlex] will assert, because there would be infinite
    /// remaining free space and boxes cannot be given infinite size.
    Min,

    /// Maximize the amount of free space along the main axis, subject to the
    /// incoming layout constraints.
    ///
    /// If the incoming layout constraints have a small enough
    /// [BoxConstraints.maxWidth] or [BoxConstraints.maxHeight], there might still
    /// be no free space.
    ///
    /// If the incoming layout constraints are unbounded, the [RenderFlex] will
    /// assert, because there would be infinite remaining free space and boxes
    /// cannot be given infinite size.
    Max,
}

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Flex<P>>))]
pub struct Flex<P: Protocol> {
    /// The direction to use as the main axis.
    pub direction: Axis,
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
    pub flip_main_axis: bool,
    #[builder(default = false)]
    pub flip_cross_axis: bool,
    pub children: Vec<Flexible<P>>,
}

#[derive(Clone, Debug)]
pub struct Flexible<P: Protocol> {
    pub flex: u32,
    pub fit: FlexFit,
    pub child: ArcChildWidget<P>,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct FlexibleConfig {
    flex: u32,
    fit: FlexFit,
}

impl<P: Protocol> Flexible<P> {
    fn get_flexible_config(&self) -> FlexibleConfig {
        FlexibleConfig {
            flex: self.flex,
            fit: self.fit,
        }
    }
}

impl<P: Protocol> From<ArcChildWidget<P>> for Flexible<P> {
    fn from(value: ArcChildWidget<P>) -> Self {
        Flexible {
            flex: 0,
            fit: FlexFit::Tight,
            child: value,
        }
    }
}

impl<W: ChildWidget<P>, P: Protocol> From<Asc<W>> for Flexible<P> {
    fn from(value: Asc<W>) -> Self {
        Flexible {
            flex: 0,
            fit: FlexFit::Tight,
            child: value,
        }
    }
}

/// How the child is inscribed into the available space.
///
/// See also:
///
///  * [RenderFlex], the flex render object.
///  * [Column], [Row], and [Flex], the flex widgets.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum FlexFit {
    /// The child is forced to fill the available space.
    ///
    // /// The [Expanded] widget assigns this kind of [FlexFit] to its child.
    Tight,

    /// The child can be at most as large as the available space (but is
    /// allowed to be smaller).
    ///
    // /// The [Flexible] widget assigns this kind of [FlexFit] to its child.
    Loose,
}

impl Widget for Flex<BoxProtocol> {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type Element = FlexElement;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

#[derive(Clone)]
pub struct FlexElement {}

impl ImplByTemplate for FlexElement {
    type Template = BoxMultiChildElementTemplate<true, false>;
}

impl BoxMultiChildElement for FlexElement {
    type ArcWidget = Asc<Flex<BoxProtocol>>;

    fn get_child_widgets(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<Vec<ArcBoxWidget>, BuildSuspendedError> {
        Ok(widget
            .children
            .iter()
            .map(|flexible| flexible.child.clone())
            .collect())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {}
    }
}

pub(super) fn get_flexible_configs(children: &Vec<Flexible<impl Protocol>>) -> Vec<FlexibleConfig> {
    children.iter().map(Flexible::get_flexible_config).collect()
}

impl BoxMultiChildRenderElement for FlexElement {
    type Render = RenderFlex;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderFlex {
            direction: widget.direction,
            main_axis_alignment: widget.main_axis_alignment,
            main_axis_size: widget.main_axis_size,
            cross_axis_alignment: widget.cross_axis_alignment,
            flexible_configs: get_flexible_configs(&widget.children),
            flip_main_axis: widget.flip_main_axis,
            flip_cross_axis: widget.flip_cross_axis,
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        [
            set_if_changed(&mut render.direction, widget.direction),
            set_if_changed(&mut render.main_axis_alignment, widget.main_axis_alignment),
            set_if_changed(&mut render.main_axis_size, widget.main_axis_size),
            set_if_changed(
                &mut render.cross_axis_alignment,
                widget.cross_axis_alignment,
            ),
            set_if_changed(
                &mut render.flexible_configs,
                get_flexible_configs(&widget.children),
            ),
            set_if_changed(&mut render.flip_main_axis, widget.flip_main_axis),
            set_if_changed(&mut render.flip_cross_axis, widget.flip_cross_axis),
        ]
        .iter()
        .any(|&changed| changed)
        .then_some(RenderAction::Relayout)
    }
}

pub struct RenderFlex {
    pub direction: Axis,
    pub main_axis_alignment: MainAxisAlignment,
    pub main_axis_size: MainAxisSize,
    pub cross_axis_alignment: CrossAxisAlignment,
    pub flexible_configs: Vec<FlexibleConfig>,
    pub flip_main_axis: bool,
    pub flip_cross_axis: bool,
}

impl ImplByTemplate for RenderFlex {
    type Template = BoxMultiChildRenderTemplate<false, false, false, false>;
}

impl BoxMultiChildRender for RenderFlex {
    type LayoutMemo = (Vec<BoxOffset>, f32);
}

impl BoxMultiChildLayout for RenderFlex {
    fn perform_layout(
        &mut self,
        constraints: &BoxConstraints,
        children: &Vec<ArcBoxRenderObject>,
    ) -> (BoxSize, Self::LayoutMemo) {
        debug_assert_eq!(self.flexible_configs.len(), children.len());
        fn get_main_size(size: &BoxSize, direction: &Axis) -> f32 {
            match direction {
                Axis::Horizontal => size.width,
                Axis::Veritcal => size.height,
            }
        }
        fn get_cross_size(size: &BoxSize, direction: &Axis) -> f32 {
            match direction {
                Axis::Horizontal => size.height,
                Axis::Veritcal => size.width,
            }
        }
        // RenderFlex::_computeSized
        let mut child_sizes = std::iter::repeat(BoxSize::ZERO)
            .take(children.len())
            .collect::<Vec<_>>();
        let mut total_flex = 0;
        let max_main_size = get_main_size(&constraints.biggest(), &self.direction);
        let can_flex = max_main_size.is_finite();

        let mut cross_size = 0.0f32;
        let mut allocated_size = 0.0;

        let stretched = self.cross_axis_alignment == CrossAxisAlignment::Stretch;

        for ((child, FlexibleConfig { flex, fit: _ }), size) in zip(
            zip(children.iter(), self.flexible_configs.iter()),
            child_sizes.iter_mut(),
        ) {
            if *flex > 0 {
                total_flex += flex;
            } else {
                let inner_constraints = match (stretched, self.direction) {
                    (true, Axis::Horizontal) => {
                        BoxConstraints::new_tight_height(constraints.max_height)
                    }
                    (true, Axis::Veritcal) => {
                        BoxConstraints::new_tight_width(constraints.max_width)
                    }
                    (false, Axis::Horizontal) => {
                        BoxConstraints::new_max_height(constraints.max_height)
                    }
                    (false, Axis::Veritcal) => BoxConstraints::new_max_width(constraints.max_width),
                };
                let child_size = child.layout_use_size(&inner_constraints);
                allocated_size += get_main_size(&child_size, &self.direction);
                cross_size = cross_size.max(get_cross_size(&child_size, &self.direction));
                *size = child_size;
            }
        }

        let free_space = (if can_flex { max_main_size } else { 0.0 } - allocated_size).max(0.0);
        let mut allocated_flex_space = 0.0;

        if total_flex > 0 {
            let space_per_flex = free_space / total_flex as f32;
            for ((child, FlexibleConfig { flex, fit }), size) in zip(
                zip(children.iter(), self.flexible_configs.iter()),
                child_sizes.iter_mut(),
            ) {
                if *flex > 0 {
                    let max_child_extent = if can_flex {
                        space_per_flex * *flex as f32 // TODO: last child accomodation
                    } else {
                        f32::INFINITY
                    };
                    let min_child_extent = match fit {
                        FlexFit::Tight => max_child_extent,
                        FlexFit::Loose => 0.0,
                    };
                    assert!(min_child_extent.is_finite());
                    let min_cross_size = if stretched {
                        get_cross_size(&constraints.biggest(), &self.direction)
                    } else {
                        0.0
                    };
                    let inner_constraints = match self.direction {
                        Axis::Horizontal => BoxConstraints {
                            min_width: min_child_extent,
                            max_width: max_child_extent,
                            min_height: min_cross_size,
                            max_height: constraints.max_height,
                        },
                        Axis::Veritcal => BoxConstraints {
                            min_width: min_cross_size,
                            max_width: constraints.max_width,
                            min_height: min_child_extent,
                            max_height: max_child_extent,
                        },
                    };
                    let child_size = child.layout_use_size(&inner_constraints);
                    let child_main_size = get_main_size(&child_size, &self.direction);
                    // TODO assert
                    allocated_size += child_main_size;
                    allocated_flex_space += max_child_extent;
                    cross_size = cross_size.max(get_cross_size(&child_size, &self.direction));
                    *size = child_size;
                }
            }
        }

        let ideal_size = match self.main_axis_size {
            MainAxisSize::Min if can_flex => max_main_size,
            _ => allocated_size,
        };

        // RenderFlex::performLayout

        let (actual_size, actual_main_size, cross_size) = match self.direction {
            Axis::Horizontal => {
                let size = constraints.constrain(BoxSize {
                    width: ideal_size,
                    height: cross_size,
                });
                (size, size.width, size.height)
            }
            Axis::Veritcal => {
                let size = constraints.constrain(BoxSize {
                    width: cross_size,
                    height: ideal_size,
                });
                (size, size.height, size.width)
            }
        };

        let actual_main_size_delta = actual_main_size - allocated_size;
        let overflow = 0.0f32.max(-actual_main_size_delta);
        let remaining_space = 0.0f32.max(actual_main_size_delta);
        let child_count = children.len();
        let between_space = match self.main_axis_alignment {
            MainAxisAlignment::Start | MainAxisAlignment::End | MainAxisAlignment::Center => 0.0,
            MainAxisAlignment::SpaceBetween if child_count > 1 => {
                remaining_space / (child_count - 1) as f32
            }
            MainAxisAlignment::SpaceAround if child_count > 0 => {
                remaining_space / child_count as f32
            }
            MainAxisAlignment::SpaceEvenly if child_count > 0 => {
                remaining_space / (child_count + 1) as f32
            }
            _ => 0.0,
        };
        let leading_space = match self.main_axis_alignment {
            MainAxisAlignment::Start => 0.0,
            MainAxisAlignment::End => remaining_space,
            MainAxisAlignment::Center => remaining_space / 2.0,
            MainAxisAlignment::SpaceBetween => 0.0,
            MainAxisAlignment::SpaceAround => between_space / 2.0,
            MainAxisAlignment::SpaceEvenly => between_space,
        };

        let mut child_main_position = if self.flip_main_axis {
            actual_main_size - leading_space
        } else {
            leading_space
        };

        let child_offsets = child_sizes
            .into_iter()
            .map(|child_size| {
                let child_cross_position = match self.cross_axis_alignment {
                    CrossAxisAlignment::Start | CrossAxisAlignment::End => {
                        if !self.flip_cross_axis
                            == (self.cross_axis_alignment == CrossAxisAlignment::Start)
                        {
                            0.0
                        } else {
                            cross_size - get_cross_size(&child_size, &self.direction)
                        }
                    }
                    CrossAxisAlignment::Center => {
                        (cross_size - get_cross_size(&child_size, &self.direction)) / 2.0
                    }
                    CrossAxisAlignment::Stretch => 0.0,
                };

                if self.flip_main_axis {
                    child_main_position -= get_main_size(&child_size, &self.direction);
                }
                let child_offset = match self.direction {
                    Axis::Horizontal => BoxOffset {
                        x: child_main_position,
                        y: child_cross_position,
                    },
                    Axis::Veritcal => BoxOffset {
                        x: child_cross_position,
                        y: child_main_position,
                    },
                };
                if self.flip_main_axis {
                    child_main_position -= between_space;
                } else {
                    child_main_position +=
                        get_main_size(&child_size, &self.direction) + between_space;
                }
                child_offset
            })
            .collect();

        (actual_size, (child_offsets, overflow))
    }
}

impl BoxMultiChildPaint for RenderFlex {
    fn perform_paint(
        &self,
        size: &BoxSize,
        offset: &BoxOffset,
        memo: &Self::LayoutMemo,
        children: &Vec<ArcBoxRenderObject>,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        let (child_offsets, overflow) = memo;
        debug_assert_eq!(children.len(), child_offsets.len());
        if *overflow < PRECISION_ERROR_TOLERANCE {
            for (child_offset, child) in zip(child_offsets, children) {
                paint_ctx.paint(child, &(offset + child_offset));
            }
        } else {
            paint_ctx.clip_rect(*offset & *size, BlendMode::default(), 1.0, |paint_ctx| {
                for (child_offset, child) in zip(child_offsets, children) {
                    paint_ctx.paint(child, &(offset + child_offset));
                }
            });
            // todo!(paint overflow indicator)
        };
    }
}

impl BoxMultiChildHitTest for RenderFlex {}
