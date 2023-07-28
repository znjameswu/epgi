use std::ops::BitAnd;

pub use peniko::{
    BlendMode, Brush, Cap, Color, ColorStops, Dashes, Extend, Fill, Format, Gradient, GradientKind,
    Image, Join, Stroke,
};

use crate::{BoxOffset, Affine2d, Point2d, BoxSize, Paragraph};

pub enum Affine2dPrimitive {
    ClipPath,
    ClipRect {
        rect: Rect,
    },
    ClipRRect {
        rect: Rect,
        radius: RRectRadius,
    },
    Arc {
        rect: Rect,
        start_angle: f32,
        sweep_angle: f32,
        use_center: bool,
        painter: Painter,
    },
    // Atlas,
    Circle {
        center: Point2d,
        radius: f32,
        use_center: bool,
        painter: Painter,
    },
    Color {
        color: Color,
        blend_mode: BlendMode,
    },
    DRRect {
        outer: RRect,
        inner: RRect,
        painter: Painter,
    },
    Image,
    ImageNine,
    ImageRect,
    Line {
        p1: Point2d,
        p2: Point2d,
        painter: StrokePainter,
    },
    Oval {
        rect: Rect,
        painter: Painter,
    },
    Paint {
        painter: FillPainter,
    },
    Paragraph {
        paragraph: Paragraph,
        offset: BoxOffset,
    },
    Path,
    // Picture,
    // Points, //https://stackoverflow.com/a/56896362
    Rect {
        rect: Rect,
    },
    RRect {
        rect: Rect,
        radius: RRectRadius,
    },
    Shadow,
    Vertices,
}

pub enum Painter {
    Fill(FillPainter),
    Stroke(StrokePainter),
}

pub struct FillPainter {
    pub fill: Fill,
    pub brush: Brush,
    pub transform: Option<Affine2d>,
}

pub struct StrokePainter {
    pub stroke: Stroke,
    pub brush: Brush,
    pub transform: Option<Affine2d>,
}

pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub botton: f32,
}

impl BitAnd<BoxSize> for BoxOffset {
    type Output = Rect;

    fn bitand(self, rhs: BoxSize) -> Self::Output {
        Rect {
            left: self.x,
            top: self.y,
            right: self.x + rhs.width,
            botton: self.y + rhs.height,
        }
    }
}

pub struct RRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub botton: f32,
    pub radius: RRectRadius,
}

pub struct RRectRadius {
    pub tl_radius: f32,
    pub tr_radius: f32,
    pub bl_radius: f32,
    pub br_radius: f32,
}

pub struct Circle {
    pub c: Point2d,
    pub r: f32,
}

pub struct Ellipse {
    pub affine: Affine2d,
}
