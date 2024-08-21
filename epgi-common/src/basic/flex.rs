use std::{
    borrow::{Borrow, BorrowMut},
    f32::INFINITY,
    iter::zip,
    marker::PhantomData,
};

use epgi_2d::{
    Affine2dPaintContextExt, BlendMode, BoxConstraints, BoxOffset, BoxProtocol, BoxSize,
    PRECISION_ERROR_TOLERANCE,
};
use epgi_core::{
    foundation::{
        set_if_changed, Arc, Asc, BuildSuspendedError, Canvas, InlinableDwsizeVec, Intrinsics,
        PaintContext, Protocol, Provide, SurrogateProtocol, VecContainer,
    },
    max,
    template::{
        AdapterRender, AdapterRenderTemplate, ImplByTemplate, MultiChildElement,
        MultiChildElementTemplate, MultiChildHitTest, MultiChildLayout, MultiChildPaint,
        MultiChildRender, MultiChildRenderTemplate, SingleChildElement, SingleChildElementTemplate,
        SingleChildRenderElement,
    },
    tree::{
        ArcChildRenderObject, ArcChildWidget, BuildContext, ChildRenderObject,
        ElementBase, FullRender, RenderAction, Widget,
    },
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Axis {
    Horizontal,
    Vertical,
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
    pub children: Vec<ArcChildWidget<FlexProtocol<P>>>,
}

impl<P: Protocol> Widget for Flex<P>
where
    RenderFlex<P>: FullRender<
        ParentProtocol = P,
        ChildProtocol = FlexProtocol<P>,
        ChildContainer = VecContainer,
    >,
{
    type ParentProtocol = P;
    type ChildProtocol = FlexProtocol<P>;
    type Element = FlexElement<P>;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

#[derive(Clone)]
pub struct FlexElement<P: Protocol> {
    phantom: PhantomData<P>,
}

impl<P: Protocol> ImplByTemplate for FlexElement<P> {
    type Template = MultiChildElementTemplate<false>;
}

impl<P: Protocol> MultiChildElement for FlexElement<P>
where
    RenderFlex<P>: FullRender<
        ParentProtocol = P,
        ChildProtocol = FlexProtocol<P>,
        ChildContainer = VecContainer,
    >,
{
    type ParentProtocol = P;
    type ChildProtocol = FlexProtocol<P>;
    type ArcWidget = Asc<Flex<P>>;
    type Render = RenderFlex<P>;
    fn get_child_widgets(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<Vec<ArcChildWidget<FlexProtocol<P>>>, BuildSuspendedError> {
        Ok(widget.children.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {
            phantom: PhantomData,
        }
    }

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderFlex {
            direction: widget.direction.clone(),
            main_axis_alignment: widget.main_axis_alignment,
            main_axis_size: widget.main_axis_size,
            cross_axis_alignment: widget.cross_axis_alignment,
            flip_main_axis: widget.flip_main_axis,
            flip_cross_axis: widget.flip_cross_axis,
            phantom: PhantomData,
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        [
            set_if_changed(&mut render.direction, widget.direction.clone()),
            set_if_changed(&mut render.main_axis_alignment, widget.main_axis_alignment),
            set_if_changed(&mut render.main_axis_size, widget.main_axis_size),
            set_if_changed(
                &mut render.cross_axis_alignment,
                widget.cross_axis_alignment,
            ),
            set_if_changed(&mut render.flip_main_axis, widget.flip_main_axis),
            set_if_changed(&mut render.flip_cross_axis, widget.flip_cross_axis),
        ]
        .iter()
        .any(|&changed| changed)
        .then_some(RenderAction::Relayout)
    }
}

pub struct RenderFlex<P: Protocol> {
    pub direction: Axis,
    pub main_axis_alignment: MainAxisAlignment,
    pub main_axis_size: MainAxisSize,
    pub cross_axis_alignment: CrossAxisAlignment,
    pub flip_main_axis: bool,
    pub flip_cross_axis: bool,
    pub phantom: PhantomData<P>,
}

impl<P: Protocol> ImplByTemplate for RenderFlex<P> {
    type Template = MultiChildRenderTemplate<false, false, false, false>;
}

impl<P: Protocol> MultiChildRender for RenderFlex<P> {
    type ParentProtocol = P;
    type ChildProtocol = FlexProtocol<P>;
    type LayoutMemo = (Vec<P::Offset>, f32);

    fn compute_intrinsics(
        &mut self,
        children: &Vec<ArcChildRenderObject<FlexProtocol<P>>>,
        intrinsics: &mut P::Intrinsics,
    ) {
        unimplemented!()
    }
}

impl<P: Protocol> MultiChildLayout for RenderFlex<P>
where
    RenderFlex<P>: FlexRender<P>,
{
    fn perform_layout(
        &mut self,
        constraints: &P::Constraints,
        children: &Vec<ArcChildRenderObject<FlexProtocol<P>>>,
    ) -> (P::Size, Self::LayoutMemo) {
        default_flex_perform_layout(
            self,
            self.main_axis_size,
            self.main_axis_alignment,
            self.flip_main_axis,
            constraints,
            children,
        )
        // debug_assert_eq!(self.flexible_configs.len(), children.len());
    }
}

pub fn default_flex_perform_layout<P: Protocol, R: FlexRender<P>>(
    render: &R,
    main_axis_size: MainAxisSize,
    main_axis_alignment: MainAxisAlignment,
    flip_main_axis: bool,
    constraints: &P::Constraints,
    children: &Vec<ArcChildRenderObject<FlexProtocol<P>>>,
) -> (P::Size, (Vec<P::Offset>, f32)) {
    // RenderFlex::_computeSized
    let mut child_sizes = std::iter::repeat(R::placeholder_size())
        .take(children.len())
        .collect::<Vec<_>>();
    let mut total_flex = 0;
    let max_main_size = render.get_max_main_size(constraints);
    let can_flex = max_main_size.is_finite();

    let mut cross_size = render.initial_cross_size();
    let mut allocated_size = 0.0;

    for (child, size) in zip(children.iter(), child_sizes.iter_mut()) {
        let FlexibleConfig { flex, fit: _ } = child.get_flexible_config();
        if flex > 0 {
            total_flex += flex;
        } else {
            let inner_constraints = render.child_constraints(None, constraints);
            let child_size = child.layout_use_size(&inner_constraints);
            allocated_size += render.get_main_size(&child_size);
            render.reduce_cross_size(&mut cross_size, render.get_cross_size(&child_size));
            *size = child_size;
        }
    }

    let free_space = (if can_flex { max_main_size } else { 0.0 } - allocated_size).max(0.0);
    let mut allocated_flex_space = 0.0;

    if total_flex > 0 {
        let space_per_flex = free_space / total_flex as f32;
        for (child, size) in zip(children.iter(), child_sizes.iter_mut()) {
            let FlexibleConfig { flex, fit } = child.get_flexible_config();
            if flex > 0 {
                let max_child_extent = if can_flex {
                    space_per_flex * flex as f32 // TODO: last child accomodation
                } else {
                    f32::INFINITY
                };
                let min_child_extent = match fit {
                    FlexFit::Tight => max_child_extent,
                    FlexFit::Loose => 0.0,
                };
                assert!(min_child_extent.is_finite());
                let inner_constraints = render
                    .child_constraints(Some((min_child_extent, max_child_extent)), constraints);

                let child_size = child.layout_use_size(&inner_constraints);
                let child_main_size = render.get_main_size(&child_size);
                // TODO assert
                allocated_size += child_main_size;
                allocated_flex_space += max_child_extent;
                render.reduce_cross_size(&mut cross_size, render.get_cross_size(&child_size));
                *size = child_size;
            }
        }
    }

    let ideal_size = match main_axis_size {
        MainAxisSize::Min if can_flex => max_main_size,
        _ => allocated_size,
    };

    // RenderFlex::performLayout
    let (actual_size, actual_main_size, cross_size) =
        render.constrain_size(ideal_size, cross_size, constraints);

    let actual_main_size_delta = actual_main_size - allocated_size;
    let overflow = 0.0f32.max(-actual_main_size_delta);
    let remaining_space = 0.0f32.max(actual_main_size_delta);
    let child_count = children.len();
    let between_space = match main_axis_alignment {
        MainAxisAlignment::Start | MainAxisAlignment::End | MainAxisAlignment::Center => 0.0,
        MainAxisAlignment::SpaceBetween if child_count > 1 => {
            remaining_space / (child_count - 1) as f32
        }
        MainAxisAlignment::SpaceAround if child_count > 0 => remaining_space / child_count as f32,
        MainAxisAlignment::SpaceEvenly if child_count > 0 => {
            remaining_space / (child_count + 1) as f32
        }
        _ => 0.0,
    };
    let leading_space = match main_axis_alignment {
        MainAxisAlignment::Start => 0.0,
        MainAxisAlignment::End => remaining_space,
        MainAxisAlignment::Center => remaining_space / 2.0,
        MainAxisAlignment::SpaceBetween => 0.0,
        MainAxisAlignment::SpaceAround => between_space / 2.0,
        MainAxisAlignment::SpaceEvenly => between_space,
    };

    let mut child_main_position = if flip_main_axis {
        actual_main_size - leading_space
    } else {
        leading_space
    };

    let child_offsets = child_sizes
        .into_iter()
        .map(|child_size| {
            if flip_main_axis {
                child_main_position -= render.get_main_size(&child_size);
            }

            let child_offset = render.position_child(
                child_main_position,
                cross_size.clone(),
                &child_size,
                constraints,
            );

            if flip_main_axis {
                child_main_position -= between_space;
            } else {
                child_main_position += render.get_main_size(&child_size) + between_space;
            }
            child_offset
        })
        .collect();

    (actual_size, (child_offsets, overflow))
}

pub trait FlexRender<P: Protocol>: Send + Sync + 'static {
    type CrossSize: Clone + Send + Sync + 'static;

    fn get_main_size(&self, size: &P::Size) -> f32;
    fn get_cross_size(&self, size: &P::Size) -> Self::CrossSize;

    fn get_max_main_size(&self, parent_constraints: &P::Constraints) -> f32;

    fn placeholder_size() -> P::Size;
    fn initial_cross_size(&self) -> Self::CrossSize;
    fn reduce_cross_size(&self, cross_size: &mut Self::CrossSize, child_size: Self::CrossSize);

    fn child_constraints(
        &self,
        main_size_range: Option<(f32, f32)>,
        parent_constraints: &P::Constraints,
    ) -> P::Constraints;

    /// Returns: constrained size, constrained main size, constrained cross size
    fn constrain_size(
        &self,
        main_size: f32,
        cross_size: Self::CrossSize,
        parent_constraints: &P::Constraints,
    ) -> (P::Size, f32, Self::CrossSize);

    fn position_child(
        &self,
        main_offset: f32,
        cross_size: Self::CrossSize,
        child_size: &P::Size,
        parent_constraints: &P::Constraints,
    ) -> P::Offset;

    fn perform_paint(
        &self,
        size: &P::Size,
        offset: &P::Offset,
        child_offsets: &Vec<P::Offset>,
        overflow: f32,
        children: &Vec<ArcChildRenderObject<FlexProtocol<P>>>,
        paint_ctx: &mut impl PaintContext<Canvas = P::Canvas>,
    );
}

impl FlexRender<BoxProtocol> for RenderFlex<BoxProtocol> {
    type CrossSize = f32;

    fn get_main_size(&self, size: &BoxSize) -> f32 {
        match self.direction {
            Axis::Horizontal => size.width,
            Axis::Vertical => size.height,
        }
    }
    fn get_cross_size(&self, size: &BoxSize) -> f32 {
        match self.direction {
            Axis::Horizontal => size.height,
            Axis::Vertical => size.width,
        }
    }

    fn get_max_main_size(&self, parent_constraints: &BoxConstraints) -> f32 {
        self.get_main_size(&parent_constraints.biggest())
    }

    fn placeholder_size() -> BoxSize {
        BoxSize::ZERO
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
        parent_constraints: &BoxConstraints,
    ) -> BoxConstraints {
        let (max_main_size, min_main_size) = main_size_range.unwrap_or((0.0, INFINITY));
        match self.direction {
            Axis::Horizontal => BoxConstraints {
                min_width: min_main_size,
                max_width: max_main_size,
                min_height: if self.cross_axis_alignment != CrossAxisAlignment::Stretch {
                    0.0
                } else {
                    parent_constraints.max_height
                },
                max_height: parent_constraints.max_height,
            },
            Axis::Vertical => BoxConstraints {
                min_width: if self.cross_axis_alignment != CrossAxisAlignment::Stretch {
                    0.0
                } else {
                    parent_constraints.max_width
                },
                max_width: parent_constraints.max_width,
                min_height: min_main_size,
                max_height: max_main_size,
            },
        }
    }

    fn constrain_size(
        &self,
        main_size: f32,
        cross_size: f32,
        parent_constraints: &BoxConstraints,
    ) -> (BoxSize, f32, f32) {
        match self.direction {
            Axis::Horizontal => {
                let size = parent_constraints.constrain(BoxSize {
                    width: main_size,
                    height: cross_size,
                });
                (size, size.width, size.height)
            }
            Axis::Vertical => {
                let size = parent_constraints.constrain(BoxSize {
                    width: cross_size,
                    height: main_size,
                });
                (size, size.height, size.width)
            }
        }
    }

    fn position_child(
        &self,
        main_offset: f32,
        cross_size: f32,
        child_size: &BoxSize,
        _parent_constraints: &BoxConstraints,
    ) -> BoxOffset {
        let child_cross_position = match self.cross_axis_alignment {
            CrossAxisAlignment::Start | CrossAxisAlignment::End => {
                if !self.flip_cross_axis == (self.cross_axis_alignment == CrossAxisAlignment::Start)
                {
                    0.0
                } else {
                    cross_size - self.get_cross_size(&child_size)
                }
            }
            CrossAxisAlignment::Center => (cross_size - self.get_cross_size(&child_size)) / 2.0,
            CrossAxisAlignment::Stretch => 0.0,
        };
        let child_offset = match self.direction {
            Axis::Horizontal => BoxOffset {
                x: main_offset,
                y: child_cross_position,
            },
            Axis::Vertical => BoxOffset {
                x: child_cross_position,
                y: main_offset,
            },
        };
        child_offset
    }

    fn perform_paint(
        &self,
        &size: &<BoxProtocol as Protocol>::Size,
        &offset: &<BoxProtocol as Protocol>::Offset,
        child_offsets: &Vec<<BoxProtocol as Protocol>::Offset>,
        overflow: f32,
        children: &Vec<ArcChildRenderObject<FlexProtocol<BoxProtocol>>>,
        paint_ctx: &mut impl PaintContext<Canvas = <BoxProtocol as Protocol>::Canvas>,
    ) {
        if overflow < PRECISION_ERROR_TOLERANCE {
            for (&child_offset, child) in zip(child_offsets, children) {
                paint_ctx.paint(child, &(offset + child_offset));
            }
        } else {
            paint_ctx.clip_rect(offset & size, BlendMode::default(), 1.0, |paint_ctx| {
                for (&child_offset, child) in zip(child_offsets, children) {
                    paint_ctx.paint(child, &(offset + child_offset));
                }
            });
            // todo!(paint overflow indicator)
        };
    }
}

impl<P: Protocol> MultiChildPaint for RenderFlex<P>
where
    RenderFlex<P>: FlexRender<P>,
{
    fn perform_paint(
        &self,
        size: &P::Size,
        offset: &P::Offset,
        memo: &Self::LayoutMemo,
        children: &Vec<ArcChildRenderObject<FlexProtocol<P>>>,
        paint_ctx: &mut impl PaintContext<Canvas = P::Canvas>,
    ) {
        let (child_offsets, overflow) = memo;
        FlexRender::perform_paint(
            self,
            size,
            offset,
            child_offsets,
            *overflow,
            children,
            paint_ctx,
        )
    }
}

impl<P: Protocol> MultiChildHitTest for RenderFlex<P> {}

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

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct FlexibleConfig {
    flex: u32,
    fit: FlexFit,
}

impl Default for FlexibleConfig {
    fn default() -> Self {
        Self {
            flex: 0,
            fit: FlexFit::Tight,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FlexProtocol<P: Protocol> {
    phantom: PhantomData<P>,
}

#[derive(Debug, Clone)]
pub enum FlexIntrinsics<T> {
    Flex { res: FlexibleConfig },
    Intrinsics(T),
}

impl<T: Intrinsics> Intrinsics for FlexIntrinsics<T> {
    fn eq_tag(&self, other: &Self) -> bool {
        use FlexIntrinsics::*;
        match (self, other) {
            (Flex { .. }, Flex { .. }) => true,
            (Intrinsics(x1), Intrinsics(x2)) => x1.eq_tag(x2),
            _ => false,
        }
    }

    fn eq_param(&self, other: &Self) -> bool {
        use FlexIntrinsics::*;
        match (self, other) {
            (Flex { .. }, Flex { .. }) => true,
            (Intrinsics(x1), Intrinsics(x2)) => x1.eq_param(x2),
            _ => false,
        }
    }
}

pub trait FlexRenderObjectIntrinsicsExt {
    fn get_flexible_config(&self) -> FlexibleConfig;
}

impl<P: Protocol> FlexRenderObjectIntrinsicsExt for dyn ChildRenderObject<FlexProtocol<P>> {
    fn get_flexible_config(&self) -> FlexibleConfig {
        let mut intrinsics = FlexIntrinsics::Flex {
            res: FlexibleConfig::default(),
        };
        self.get_intrinsics(&mut intrinsics);
        let FlexIntrinsics::Flex { res } = intrinsics else {
            panic!("Child returns a wrong intrinsics type.")
        };
        res
    }
}

impl<P: Protocol> Protocol for FlexProtocol<P> {
    type Constraints = P::Constraints;
    type Offset = P::Offset;
    type Size = P::Size;
    type Intrinsics = FlexIntrinsics<P::Intrinsics>;
    type Canvas = P::Canvas;

    fn position_in_shape(
        position: &<P::Canvas as Canvas>::HitPosition,
        offset: &P::Offset,
        size: &P::Size,
    ) -> bool {
        P::position_in_shape(position, offset, size)
    }
}

impl<P: Protocol> SurrogateProtocol<P> for FlexProtocol<P> {
    fn convert_constraints(value: &Self::Constraints) -> impl Borrow<P::Constraints> {
        value
    }

    fn convert_offset(value: Self::Offset) -> P::Offset {
        value
    }

    fn recover_size(value: P::Size) -> Self::Size {
        value
    }

    fn convert_intrinsics(
        value: &mut Self::Intrinsics,
    ) -> Result<impl BorrowMut<P::Intrinsics>, ()> {
        match value {
            FlexIntrinsics::Flex { res } => {
                *res = FlexibleConfig {
                    flex: 0,
                    fit: FlexFit::Tight,
                };
                return Err(());
            }
            FlexIntrinsics::Intrinsics(x) => Ok(x),
        }
    }

    fn recover_intrinsics(value: P::Intrinsics) -> Self::Intrinsics {
        FlexIntrinsics::Intrinsics(value)
    }
}

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Flexible<P>>))]
pub struct Flexible<P: Protocol> {
    #[builder(default = 1)]
    pub flex: u32,
    #[builder(default=FlexFit::Loose)]
    pub fit: FlexFit,
    pub child: ArcChildWidget<P>,
}

impl<P: Protocol> Widget for Flexible<P> {
    type ParentProtocol = FlexProtocol<P>;
    type ChildProtocol = P;
    type Element = FlexibleElement<P>;

    fn into_arc_widget(self: Asc<Self>) -> Asc<Self> {
        self
    }
}

#[derive(Clone, Debug)]
pub struct FlexibleElement<P: Protocol> {
    phantom: PhantomData<P>,
}

impl<P: Protocol> ImplByTemplate for FlexibleElement<P> {
    type Template = SingleChildElementTemplate<true, false>;
}

impl<P: Protocol> SingleChildElement for FlexibleElement<P> {
    type ParentProtocol = FlexProtocol<P>;
    type ChildProtocol = P;
    type ArcWidget = Asc<Flexible<P>>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<P>, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<P: Protocol> SingleChildRenderElement for FlexibleElement<P> {
    type Render = RenderFlexible<P>;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderFlexible {
            config: FlexibleConfig {
                flex: widget.flex,
                fit: widget.fit,
            },
            phantom: PhantomData,
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        max!(
            set_if_changed(&mut render.config.flex, widget.flex).then_some(RenderAction::Relayout),
            set_if_changed(&mut render.config.fit, widget.fit).then_some(RenderAction::Relayout),
        )
    }
}

pub struct RenderFlexible<P: Protocol> {
    config: FlexibleConfig,
    phantom: PhantomData<P>,
}

impl<P: Protocol> ImplByTemplate for RenderFlexible<P> {
    type Template = AdapterRenderTemplate;
}

impl<P: Protocol> AdapterRender for RenderFlexible<P> {
    type ParentProtocol = FlexProtocol<P>;
    type ChildProtocol = P;
    type LayoutMemo = ();

    fn perform_layout(
        &mut self,
        constraints: &P::Constraints,
        child: &ArcChildRenderObject<P>,
    ) -> (P::Size, ()) {
        (child.layout_use_size(constraints), ())
    }

    fn compute_intrinsics(
        &mut self,
        child: &ArcChildRenderObject<P>,
        intrinsics: &mut FlexIntrinsics<P::Intrinsics>,
    ) {
        match intrinsics {
            FlexIntrinsics::Flex { res } => {
                *res = self.config.clone();
            }
            FlexIntrinsics::Intrinsics(intrinsics) => child.get_intrinsics(intrinsics),
        }
    }

    fn perform_paint(
        &self,
        _size: &P::Size,
        offset: &P::Offset,
        _memo: &(),
        child: &ArcChildRenderObject<P>,
        paint_ctx: &mut impl PaintContext<Canvas = P::Canvas>,
    ) {
        paint_ctx.paint(child, offset)
    }
}
