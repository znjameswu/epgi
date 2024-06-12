use std::{
    borrow::Cow,
    f32::consts::{FRAC_PI_2, PI},
    ops::Deref,
    time::Duration,
};

use epgi_2d::{
    Affine2dCanvas, Affine2dPaintContextExt, ArcBoxWidget, BoxConstraints, BoxOffset, BoxProtocol,
    BoxSize, Brush, Color, EllipticalArc, Point2d, Stroke, StrokeCap, StrokePainter,
};
use epgi_common::{
    AnimationControllerConf, AnimationFrame, BuildContextUseAnimationControllerExt, ConstrainedBox,
    CustomPaint, CustomPainter, Interval, SawTooth, Tween, FAST_OUT_SLOW_IN,
};
use epgi_core::{
    foundation::{Arc, Asc, AscProvideExt, InlinableDwsizeVec, PaintContext, Provide, TypeKey},
    nodes::{ConsumerElement, ConsumerWidget},
    read_one_provider_into,
    tree::{BuildContext, Widget},
};
use epgi_macro::Declarative;
use lazy_static::lazy_static;
use typed_builder::TypedBuilder;

use crate::ThemeData;

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<CircularProgressIndicator>))]
pub struct CircularProgressIndicator {
    pub value: Option<f32>,
    pub background_color: Option<Color>,
    pub color: Option<Color>,
    pub value_color: Option<Color>,
    #[builder(default = 4.0)]
    pub stroke_width: f32,
    #[builder(default = 0.0)]
    pub stroke_align: f32,
    #[builder(default)]
    pub stroke_cap: Option<StrokeCap>,
}

impl Widget for CircularProgressIndicator {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Element = ConsumerElement<BoxProtocol>;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> Asc<dyn ConsumerWidget<BoxProtocol>> {
        self
    }
}

lazy_static! {
    static ref CIRCULAR_PROGRESS_INDICATOR_CONSUMED_TYPES_DETERMINATE: [TypeKey; 2] = [
        TypeKey::of::<ProgressIndicatorThemeData>(),
        TypeKey::of::<ThemeData>(),
    ];
    static ref CIRCULAR_PROGRESS_INDICATOR_CONSUMED_TYPES_INDETERMINATE: [TypeKey; 3] = [
        TypeKey::of::<ProgressIndicatorThemeData>(),
        TypeKey::of::<ThemeData>(),
        TypeKey::of::<AnimationFrame>(),
    ];
}

const MIN_CIRCULAR_PROGRESS_INDICATOR_SIZE: f32 = 36.0;

const INTETERMINATE_CIRCULAR_DURATION: Duration = Duration::from_millis(1333 * 2222);

const PATH_COUNT: u32 = 2222;

const ROTATION_COUNT: u32 = 1333;

impl ConsumerWidget<BoxProtocol> for CircularProgressIndicator {
    fn get_consumed_types(&self) -> Cow<[TypeKey]> {
        if self.value.is_some() {
            CIRCULAR_PROGRESS_INDICATOR_CONSUMED_TYPES_DETERMINATE
                .deref()
                .into()
        } else {
            CIRCULAR_PROGRESS_INDICATOR_CONSUMED_TYPES_INDETERMINATE
                .deref()
                .into()
        }
    }

    fn build(
        &self,
        ctx: &mut BuildContext,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> ArcBoxWidget {
        // let (indicator_theme, theme) =
        //     read_providers!(provider_values, ProgressIndicatorThemeData, ThemeData);

        read_one_provider_into!(indicator_theme, provider_values, ProgressIndicatorThemeData);
        read_one_provider_into!(theme, provider_values, ThemeData);
        let animation_frame = if self.value.is_none() {
            read_one_provider_into!(animation_frame, provider_values, AnimationFrame);
            Some(animation_frame)
        } else {
            None
        };

        let track_color = self
            .background_color
            .or(indicator_theme.circular_track_color);
        let value_color = self
            .value_color
            .or(indicator_theme.color)
            // .or(todo!()) // _CircularProgressIndicatorDefaultsM3/2 just gives the same default value
            .unwrap_or(theme.color_scheme.primary);

        let (x, _animation_controller) = ctx.use_animation_controller_repeating_with(
            false,
            AnimationControllerConf!(duration = INTETERMINATE_CIRCULAR_DURATION),
            animation_frame.as_deref(),
        );

        let (start_angle, sweep_angle) = if let Some(value) = self.value {
            (-FRAC_PI_2, value.clamp(0.0, 1.0) * (2.0 * PI))
        } else {
            let head_curve = FAST_OUT_SLOW_IN
                .chain_after(Interval {
                    begin: 0.0,
                    end: 0.5,
                })
                .chain_after(SawTooth { count: PATH_COUNT });

            let tail_curve = FAST_OUT_SLOW_IN
                .chain_after(Interval {
                    begin: 0.5,
                    end: 1.0,
                })
                .chain_after(SawTooth { count: PATH_COUNT });

            let offset_curve = SawTooth { count: PATH_COUNT };
            let rotation_curve = SawTooth {
                count: ROTATION_COUNT,
            };

            let head_value = head_curve.interp(x);
            let tail_value = tail_curve.interp(x);
            let offset_value = offset_curve.interp(x);
            let rotation_value = rotation_curve.interp(x);

            let start_angle = -FRAC_PI_2
                + (tail_value * 0.75 + rotation_value + offset_value * 0.25) * (2.0 * PI);
            let sweep_angle = (head_value - tail_value) * (0.75 * PI);
            (start_angle, sweep_angle)
        };

        ConstrainedBox!(
            constraints = BoxConstraints {
                min_width: MIN_CIRCULAR_PROGRESS_INDICATOR_SIZE,
                max_width: f32::INFINITY,
                min_height: MIN_CIRCULAR_PROGRESS_INDICATOR_SIZE,
                max_height: f32::INFINITY,
            },
            child = CustomPaint!(
                painter = CircularProgressIndicatorPainter {
                    background_color: track_color,
                    value_color,
                    start_angle,
                    sweep_angle,
                    stroke_width: self.stroke_width,
                    stroke_align: self.stroke_align,
                    stroke_cap: self.stroke_cap,
                    indeterminate_mode: self.value.is_none(),
                },
                foreground_painter = ()
            )
        )
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct ProgressIndicatorThemeData {
    pub color: Option<Color>,
    pub linear_track_color: Option<Color>,
    pub linear_min_height: Option<f32>,
    pub circular_track_color: Option<Color>,
    pub refresh_background_color: Option<Color>,
}

#[derive(PartialEq, Clone, Debug)]
struct CircularProgressIndicatorPainter {
    background_color: Option<Color>,
    value_color: Color,
    start_angle: f32,
    sweep_angle: f32,
    stroke_width: f32,
    stroke_align: f32,
    stroke_cap: Option<StrokeCap>,
    indeterminate_mode: bool,
}

impl CustomPainter for CircularProgressIndicatorPainter {
    fn perform_paint(
        &self,
        size: &BoxSize,
        offset: &BoxOffset,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        let center = Point2d {
            x: offset.x + size.width / 2.0,
            y: offset.y + size.height / 2.0,
        };
        if let Some(background_color) = self.background_color {
            paint_ctx.stroke_elliptical_arc(
                EllipticalArc {
                    c: center,
                    r: (size.width / 2.0, size.height / 2.0),
                    start_angle: 0.0,
                    sweep_angle: 2.0 * PI,
                    x_rotation: 0.0,
                },
                StrokePainter {
                    stroke: Stroke::new(self.stroke_width as _),
                    brush: Brush::Solid(background_color),
                    transform: None,
                },
            )
        }

        let stroke_cap = self.stroke_cap.unwrap_or(if self.indeterminate_mode {
            StrokeCap::Square
        } else {
            StrokeCap::Butt
        });
        paint_ctx.stroke_elliptical_arc(
            EllipticalArc {
                c: center,
                r: (size.width / 2.0, size.height / 2.0),
                start_angle: self.start_angle,
                sweep_angle: self.sweep_angle,
                x_rotation: 0.0,
            },
            StrokePainter {
                stroke: Stroke::new(self.stroke_width as _).with_caps(stroke_cap),
                brush: Brush::Solid(self.value_color),
                transform: None,
            },
        )
    }
    fn should_repaint(&self, other: &Self) -> bool {
        self.ne(other)
    }
}
