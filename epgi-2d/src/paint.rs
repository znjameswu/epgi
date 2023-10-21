use std::ops::BitAnd;

use epgi_core::foundation::{Canvas, PaintContext};
use peniko::kurbo::{self, PathEl};
pub use peniko::{
    BlendMode, Brush, Cap, Color, ColorStops, Dashes, Extend, Fill, Format, Gradient, GradientKind,
    Image, Join, Stroke,
};

use crate::{Affine2d, BoxOffset, BoxSize, Paragraph, Point2d};

pub enum Affine2dPaintCommand {
    DrawShape {
        shape: Affine2dCanvasShape,
        transform: Affine2d,
        painter: Painter,
    },
    DrawPathSeg {
        path_seg: Affine2dCanvasPathSeg,
        transform: Affine2d,
        painter: StrokePainter,
    },
    ClipShape {
        shape: Affine2dCanvasShape,
        transform: Affine2d,
        blend: BlendMode,
        alpha: f32,
    },
    PopClip,
    DrawParagraph {
        paragraph: Paragraph,
        transform: Affine2d,
    },
}

/// Although we provide a circle primitive, but vello does not have a precise circle encoding.
/// Scaling under a cached layer will cause distortion when scaling
/// Consider this as a known bug, won't fix.
/// This also affects any shape with arc component, such as RRect
pub enum Affine2dCanvasShape {
    Rect(Rect),
    Circle(Circle),
    RRect(RRect),
    RingSector(RingSector),
    Triangle(Point2d, Point2d, Point2d),
    Polygon(Vec<Point2d>),
    Path(Path),
    CircularArc(CircularArc),
    EllipticalArc(EllipticalArc)
}

pub enum Affine2dCanvasPathSeg {
    Line(Line),
    EllipticalArc(EllipticalArc),
    QuadBez(QuadBez),
    CubicBez(CubicBez),
}

pub enum Affine2dCanvasPathEl {
    MoveTo(Point2d),
    LineTo(Point2d),
    QuadTo(Point2d, Point2d),
    CubicTo(Point2d, Point2d, Point2d),
    EllipticalArcTo {
        dst: Point2d,
        center: Point2d,
        sweep_angle: f32,
    },
    ClosePath,
}

pub struct Line {
    p0: Point2d,
    p1: Point2d,
}

pub struct EllipticalArc {
    sweep_angle: f32,
    transform: Affine2d,
}

pub struct QuadBez {
    p0: Point2d,
    p1: Point2d,
    p2: Point2d,
}

pub struct CubicBez {
    p0: Point2d,
    p1: Point2d,
    p2: Point2d,
    p3: Point2d,
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
    pub l: f32,
    pub t: f32,
    pub r: f32,
    pub b: f32,
}

impl Rect {
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            l: left,
            t: top,
            r: right,
            b: bottom,
        }
    }
    pub fn width(&self) -> f32 {
        self.r - self.l
    }

    pub fn height(&self) -> f32 {
        self.b - self.t
    }

    pub fn is_sqaure(&self) -> bool {
        self.width() == self.height()
    }

    pub fn center(&self) -> Point2d {
        Point2d {
            x: (self.l + self.r) / 2.0,
            y: (self.t + self.b) / 2.0,
        }
    }
}

impl BitAnd<BoxSize> for BoxOffset {
    type Output = Rect;

    fn bitand(self, rhs: BoxSize) -> Self::Output {
        Rect {
            l: self.x,
            t: self.y,
            r: self.x + rhs.width,
            b: self.y + rhs.height,
        }
    }
}

pub struct RRect {
    pub l: f32,
    pub t: f32,
    pub r: f32,
    pub b: f32,
    pub radius: Box<RRectRadius>,
}

pub struct RRectRadius {
    pub tl_t_ratio: f32,
    pub tl_l_ratio: f32,
    pub tr_t_ratio: f32,
    pub tr_r_ratio: f32,
    pub bl_b_ratio: f32,
    pub bl_l_ratio: f32,
    pub br_b_ratio: f32,
    pub br_r_ratio: f32,
}

pub struct Circle {
    pub c: Point2d,
    pub r: f32,
}

pub struct RingSector {
    pub outer_cicle: Circle,
    pub inner_radius: f32,
    pub start_angle: f32,
    pub sweep_angle: f32
}

pub struct CircularArc {
    pub circle: Circle,
    pub start_angle: f32,
    pub sweep_angle: f32
}

pub struct Path {
    pub path_els: Vec<Affine2dCanvasPathEl>,
}

pub trait Affine2dPaintContextExt {
    fn clip_path(&mut self, path: Path, transform: Affine2d, blend: BlendMode, alpha: f32, op: impl FnOnce(&mut Self));
    fn clip_rect(&mut self, rect: Rect, transform: Affine2d, blend: BlendMode, alpha: f32, op: impl FnOnce(&mut Self));
    fn clip_rrect(&mut self, rrect: RRect, transform: Affine2d, blend: BlendMode, alpha: f32, op: impl FnOnce(&mut Self));
    fn clip_oval(&mut self, oval: Rect, transform: Affine2d, blend: BlendMode, alpha: f32, op: impl FnOnce(&mut Self));
    fn clip_circle(&mut self, circle: Circle, transform: Affine2d, blend: BlendMode, alpha: f32, op: impl FnOnce(&mut Self));
    fn clip_circle_sector(&mut self, circle_sector: RingSector, transform: Affine2d, blend: BlendMode, alpha: f32, op: impl FnOnce(&mut Self));
    fn clip_ring_sector(&mut self, ring_sector: RingSector,transform: Affine2d, blend: BlendMode, alpha: f32, op: impl FnOnce(&mut Self));
    fn clip_triangle(&mut self, p0: Point2d, p1: Point2d, p2: Point2d, transform: Affine2d, blend: BlendMode, alpha: f32, op: impl FnOnce(&mut Self));
    fn clip_polygon(&mut self, polygon: Vec<Point2d>, transform: Affine2d, blend: BlendMode, alpha: f32, op: impl FnOnce(&mut Self));
    fn draw_arc(
        &mut self,
        rect: Rect,
        start_angle: f32,
        sweep_angle: f32,
        use_center: bool,
        painter: Painter,
    );
    fn draw_circle(&mut self, circle: Circle, painter: Painter);
    fn draw_drrect(&mut self, outer: RRect, inner: RRect, painter: Painter);
    fn draw_image(&mut self, image: Image, top_left: Point2d);
    fn draw_image_rect(&mut self, image: Image, src: Rect, dst: Rect);
    fn draw_line(&mut self, p1: Point2d, p2: Point2d, painter: StrokePainter);
    fn draw_oval(&mut self, rect: Rect, painter: Painter);
    fn draw_paint(&mut self, painter: FillPainter);
    fn draw_paragraph(&mut self, paragraph: Paragraph, offset: BoxOffset);
    fn draw_path(&mut self, path: Path, painter: Painter);
    fn draw_rect(&mut self, rect: Rect, painter: Painter);
    fn draw_rrect(&mut self, rect: Rect, radius: RRectRadius, painter: Painter);
}

impl<T> Affine2dPaintContextExt for T
where
    T: PaintContext,
    T::Canvas: Canvas<Transform = Affine2d, PaintCommand = Affine2dPaintCommand>,
{
    #[inline(always)]
    fn clip_path(&mut self, path: Path) {
        self.add_command(Affine2dPaintCommand::ClipPath { path })
    }
    #[inline(always)]
    fn clip_rect(&mut self, rect: Rect) {
        self.add_command(Affine2dPaintCommand::ClipRect { rect })
    }
    #[inline(always)]
    fn clip_rrect(&mut self, rect: Rect, radius: RRectRadius) {
        self.add_command(Affine2dPaintCommand::ClipRRect { rect, radius })
    }
    #[inline(always)]
    fn draw_arc(
        &mut self,
        rect: Rect,
        start_angle: f32,
        sweep_angle: f32,
        use_center: bool,
        painter: Painter,
    ) {
        self.add_command(Affine2dPaintCommand::Arc {
            rect,
            start_angle,
            sweep_angle,
            use_center,
            painter,
        })
    }
    #[inline(always)]
    fn draw_circle(&mut self, center: Point2d, radius: f32, painter: Painter) {
        self.add_command(Affine2dPaintCommand::Circle {
            center,
            radius,
            painter,
        })
    }
    #[inline(always)]
    fn draw_color(&mut self, color: Color) {
        self.add_command(Affine2dPaintCommand::Color { color })
    }
    #[inline(always)]
    fn draw_drrect(&mut self, outer: RRect, inner: RRect, painter: Painter) {
        self.add_command(Affine2dPaintCommand::DRRect {
            outer,
            inner,
            painter,
        })
    }
    #[inline(always)]
    fn draw_image(&mut self, image: Image, top_left: Point2d) {
        self.add_command(Affine2dPaintCommand::Image { image, top_left })
    }
    #[inline(always)]
    fn draw_image_rect(&mut self, image: Image, src: Rect, dst: Rect) {
        self.add_command(Affine2dPaintCommand::ImageRect { image, src, dst })
    }
    #[inline(always)]
    fn draw_line(&mut self, p1: Point2d, p2: Point2d, painter: StrokePainter) {
        self.add_command(Affine2dPaintCommand::Line { p1, p2, painter })
    }
    #[inline(always)]
    fn draw_oval(&mut self, rect: Rect, painter: Painter) {
        self.add_command(Affine2dPaintCommand::Oval { rect, painter })
    }
    #[inline(always)]
    fn draw_paint(&mut self, painter: FillPainter) {
        self.add_command(Affine2dPaintCommand::Paint { painter })
    }
    #[inline(always)]
    fn draw_paragraph(&mut self, paragraph: Paragraph, transform: Affine2d) {
        self.add_command(Affine2dPaintCommand::DrawParagraph {
            paragraph,
            transform,
        })
    }
    #[inline(always)]
    fn draw_path(&mut self, path: Path, painter: Painter) {
        self.add_command(Affine2dPaintCommand::Path { path, painter })
    }
    #[inline(always)]
    fn draw_rect(&mut self, rect: Rect, painter: Painter) {
        self.add_command(Affine2dPaintCommand::Rect { rect, painter })
    }
    #[inline(always)]
    fn draw_rrect(&mut self, rect: Rect, radius: RRectRadius, painter: Painter) {
        self.add_command(Affine2dPaintCommand::RRect {
            rect,
            radius,
            painter,
        })
    }
}

// /// We could store an Affine2d from a unit square, however that would require matrix-matrix multiplication.
// /// By storing three points, we simplify to matrix-vector multiplicaiton and vector-vector addition/subtraction.
// pub struct AffineRect {
//     pub lt: Point2d,
//     pub rt: Point2d,
//     pub rb: Point2d,
// }

// pub struct AffineArc {
//     pub start_angle: f32,
//     pub sweep_angle: f32,
//     pub affine: Affine2d,
// }

// pub struct Line {
//     pub p0: Point2d,
//     pub p1: Point2d,
// }

// pub struct AffineRRect {
//     pub lt: Point2d,
//     pub rt: Point2d,
//     pub rb: Point2d,
//     pub lt_radius_x_ratio: f32,
//     pub rt_radius_x_ratio: f32,
//     pub rb_radius_x_ratio: f32,
//     pub lb_radius_x_ratio: f32,
//     pub lt_radius_y_ratio: f32,
// }

// trait Affine2dShape {
//     type ScalarArray: Array<f32>;
//     type VectorArray: Array<Point2d>;
//     type AffineArray: Array<Affine2d>;

//     fn breakdown(self) -> (Self::ScalarArray, Self::VectorArray, Self::AffineArray);
// }

// trait Array<T> {
//     const LENGTH: usize;
// }

// impl<T, const N: usize> Array<T> for [T; N] {
//     const LENGTH: usize = N;
// }

// macro_rules! impl_shape {
//     ($ty: ty, $self:ident, $n_scalar: literal, $scalar: expr, $n_vector: literal,$vector: expr, $n_affine: literal,$affine: expr) => {
//         impl Affine2dShape for $ty {
//             type SCALAR_ARRAY = [f32; $n_scalar];
//             type VECTOR_ARRAY = [Point2d; $n_vector];
//             type AFFINE_ARRAY = [Affine2d; $n_affine];

//             fn breakdown($self) -> (Self::SCALAR_ARRAY, Self::VECTOR_ARRAY, Self::AFFINE_ARRAY) {
//                 (
//                     $scalar,
//                     $vector,
//                     $affine
//                 )
//             }
//         }
//     };
// }

// // impl_shape!(Rect, self, 0, [], 2, self.LTRB, 0, []);
// // impl_shape!(Ellipse, self, 0, [], 0, [], 1, [self.affine]);
