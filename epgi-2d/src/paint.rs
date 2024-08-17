use epgi_core::foundation::{Canvas, PaintContext};

use peniko::kurbo::Stroke;

pub use peniko::{
    BlendMode, Brush, Color, ColorStops, Extend, Fill, Format, Gradient, GradientKind, Image,
};

use crate::{
    Affine2d, BoxOffset, Circle, CircularArc, CubicBez, Ellipse, EllipticalArc, Line, Paragraph, Point2d, QuadBez, RRect, Rect, RingSector, SingleLineOffset
};

pub enum Affine2dPaintCommand<'a> {
    DrawShape {
        shape: Affine2dCanvasShape,
        // transform: Affine2d,
        painter: Painter,
    },
    ClipShape {
        shape: Affine2dCanvasShape,
        // transform: Affine2d,
        blend: BlendMode,
        alpha: f32,
    },
    PopClip,
    DrawParagraph {
        paragraph: &'a Paragraph,
        offset: &'a [SingleLineOffset], // transform: Affine2d,
    },
}

/// Although we provide a circle primitive, but vello does not have a precise circle encoding.
/// Scaling under a cached layer will cause distortion when scaling
/// Consider this as a known bug, won't fix.
/// This also affects any shape with arc component, such as RRect
pub enum Affine2dCanvasShape {
    Rect(Rect),
    RRect(RRect),
    RRectPadding { rrect: RRect, padding: f32 },
    Circle(Circle),
    Ellipse(Ellipse),
    RingSector(RingSector),
    Triangle(Point2d, Point2d, Point2d),
    Polygon(Vec<Point2d>),
    Path(Path),
    Line(Line),
    CircularArc(CircularArc),
    EllipticalArc(EllipticalArc),
    QuadBez(QuadBez),
    CubicBez(CubicBez),
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

pub struct Path {
    pub path_els: Vec<Affine2dCanvasPathEl>,
}

pub trait Affine2dPaintContextExt {
    fn with_paint_offset(
        &mut self,
        offset: &BoxOffset,
        op: impl FnOnce(&mut Self),
    );
    fn clip_rect(&mut self, rect: Rect, blend: BlendMode, alpha: f32, op: impl FnOnce(&mut Self));
    fn clip_rrect(
        &mut self,
        rrect: RRect,

        blend: BlendMode,
        alpha: f32,
        op: impl FnOnce(&mut Self),
    );
    fn clip_circle(
        &mut self,
        circle: Circle,

        blend: BlendMode,
        alpha: f32,
        op: impl FnOnce(&mut Self),
    );
    fn clip_ellipse(
        &mut self,
        ellipse: Ellipse,

        blend: BlendMode,
        alpha: f32,
        op: impl FnOnce(&mut Self),
    );
    fn clip_ring_sector(
        &mut self,
        ring_sector: RingSector,

        blend: BlendMode,
        alpha: f32,
        op: impl FnOnce(&mut Self),
    );
    fn clip_triangle(
        &mut self,
        p0: Point2d,
        p1: Point2d,
        p2: Point2d,

        blend: BlendMode,
        alpha: f32,
        op: impl FnOnce(&mut Self),
    );
    fn clip_polygon(
        &mut self,
        polygon: Vec<Point2d>,

        blend: BlendMode,
        alpha: f32,
        op: impl FnOnce(&mut Self),
    );
    fn clip_path(&mut self, path: Path, blend: BlendMode, alpha: f32, op: impl FnOnce(&mut Self));
    fn clip_circle_segment(
        &mut self,
        segment: CircularArc,

        blend: BlendMode,
        alpha: f32,
        op: impl FnOnce(&mut Self),
    );

    fn draw_rect(&mut self, rect: Rect, painter: Painter);
    fn draw_rrect(&mut self, rrect: RRect, painter: Painter);
    fn draw_circle(&mut self, circle: Circle, painter: Painter);
    fn draw_ellipse(&mut self, ellipse: Ellipse, painter: Painter);
    fn draw_ring_sector(&mut self, ring_sector: RingSector, painter: Painter);
    fn draw_triangle(&mut self, p0: Point2d, p1: Point2d, p2: Point2d, painter: Painter);
    fn draw_polygon(&mut self, polygon: Vec<Point2d>, painter: Painter);
    fn draw_path(&mut self, path: Path, painter: Painter);

    fn stroke_line(&mut self, line: Line, painter: StrokePainter);
    fn stroke_arc(&mut self, arc: CircularArc, painter: StrokePainter);
    fn stroke_elliptical_arc(&mut self, arc: EllipticalArc, painter: StrokePainter);
    fn stroke_quad_bez(&mut self, seg: QuadBez, painter: StrokePainter);
    fn stroke_cubic_bez(&mut self, seg: CubicBez, painter: StrokePainter);

    fn fill_circle_segment(&mut self, segment: CircularArc, painter: FillPainter);
    fn fill_rrect_padding(&mut self, rrect: RRect, linewidth: f32, painter: FillPainter);

    fn draw_image_rect(&mut self, image: Image, src: Rect, dst: Rect);

    fn draw_paragraph(&mut self, paragraph: &Paragraph, offset: &[SingleLineOffset]);
}

impl<T: ?Sized> Affine2dPaintContextExt for T
where
    T: PaintContext,
    for<'a> T::Canvas: Canvas<Transform = Affine2d, PaintCommand<'a> = Affine2dPaintCommand<'a>>,
{
    fn with_paint_offset(
        &mut self,
        offset: &BoxOffset,
        op: impl FnOnce(&mut Self),
    ) {
        self.with_transform(Affine2d::from_translation(offset), op)
    }
    #[inline(always)]
    fn clip_rect(&mut self, rect: Rect, blend: BlendMode, alpha: f32, op: impl FnOnce(&mut Self)) {
        self.add_command(Affine2dPaintCommand::ClipShape {
            shape: Affine2dCanvasShape::Rect(rect),

            blend,
            alpha,
        });
        op(self);
        self.add_command(Affine2dPaintCommand::PopClip);
    }

    #[inline(always)]
    fn clip_rrect(
        &mut self,
        rrect: RRect,

        blend: BlendMode,
        alpha: f32,
        op: impl FnOnce(&mut Self),
    ) {
        self.add_command(Affine2dPaintCommand::ClipShape {
            shape: Affine2dCanvasShape::RRect(rrect),

            blend,
            alpha,
        });
        op(self);
        self.add_command(Affine2dPaintCommand::PopClip);
    }

    #[inline(always)]
    fn clip_circle(
        &mut self,
        circle: Circle,

        blend: BlendMode,
        alpha: f32,
        op: impl FnOnce(&mut Self),
    ) {
        self.add_command(Affine2dPaintCommand::ClipShape {
            shape: Affine2dCanvasShape::Circle(circle),

            blend,
            alpha,
        });
        op(self);
        self.add_command(Affine2dPaintCommand::PopClip);
    }

    #[inline(always)]
    fn clip_ellipse(
        &mut self,
        ellipse: Ellipse,

        blend: BlendMode,
        alpha: f32,
        op: impl FnOnce(&mut Self),
    ) {
        self.add_command(Affine2dPaintCommand::ClipShape {
            shape: Affine2dCanvasShape::Ellipse(ellipse),

            blend,
            alpha,
        });
        op(self);
        self.add_command(Affine2dPaintCommand::PopClip);
    }

    #[inline(always)]
    fn clip_ring_sector(
        &mut self,
        ring_sector: RingSector,

        blend: BlendMode,
        alpha: f32,
        op: impl FnOnce(&mut Self),
    ) {
        self.add_command(Affine2dPaintCommand::ClipShape {
            shape: Affine2dCanvasShape::RingSector(ring_sector),

            blend,
            alpha,
        });
        op(self);
        self.add_command(Affine2dPaintCommand::PopClip);
    }

    #[inline(always)]
    fn clip_triangle(
        &mut self,
        p0: Point2d,
        p1: Point2d,
        p2: Point2d,

        blend: BlendMode,
        alpha: f32,
        op: impl FnOnce(&mut Self),
    ) {
        self.add_command(Affine2dPaintCommand::ClipShape {
            shape: Affine2dCanvasShape::Triangle(p0, p1, p2),

            blend,
            alpha,
        });
        op(self);
        self.add_command(Affine2dPaintCommand::PopClip);
    }

    #[inline(always)]
    fn clip_polygon(
        &mut self,
        polygon: Vec<Point2d>,

        blend: BlendMode,
        alpha: f32,
        op: impl FnOnce(&mut Self),
    ) {
        self.add_command(Affine2dPaintCommand::ClipShape {
            shape: Affine2dCanvasShape::Polygon(polygon),

            blend,
            alpha,
        });
        op(self);
        self.add_command(Affine2dPaintCommand::PopClip);
    }

    #[inline(always)]
    fn clip_path(&mut self, path: Path, blend: BlendMode, alpha: f32, op: impl FnOnce(&mut Self)) {
        self.add_command(Affine2dPaintCommand::ClipShape {
            shape: Affine2dCanvasShape::Path(path),

            blend,
            alpha,
        });
        op(self);
        self.add_command(Affine2dPaintCommand::PopClip);
    }

    #[inline(always)]
    fn clip_circle_segment(
        &mut self,
        segment: CircularArc,

        blend: BlendMode,
        alpha: f32,
        op: impl FnOnce(&mut Self),
    ) {
        self.add_command(Affine2dPaintCommand::ClipShape {
            shape: Affine2dCanvasShape::CircularArc(segment),

            blend,
            alpha,
        });
        op(self);
        self.add_command(Affine2dPaintCommand::PopClip);
    }

    #[inline(always)]
    fn draw_rect(&mut self, rect: Rect, painter: Painter) {
        self.add_command(Affine2dPaintCommand::DrawShape {
            shape: Affine2dCanvasShape::Rect(rect),

            painter,
        });
    }

    #[inline(always)]
    fn draw_rrect(&mut self, rrect: RRect, painter: Painter) {
        self.add_command(Affine2dPaintCommand::DrawShape {
            shape: Affine2dCanvasShape::RRect(rrect),

            painter,
        });
    }

    #[inline(always)]
    fn draw_circle(&mut self, circle: Circle, painter: Painter) {
        self.add_command(Affine2dPaintCommand::DrawShape {
            shape: Affine2dCanvasShape::Circle(circle),

            painter,
        });
    }

    #[inline(always)]
    fn draw_ellipse(&mut self, ellipse: Ellipse, painter: Painter) {
        self.add_command(Affine2dPaintCommand::DrawShape {
            shape: Affine2dCanvasShape::Ellipse(ellipse),

            painter,
        });
    }

    #[inline(always)]
    fn draw_ring_sector(&mut self, ring_sector: RingSector, painter: Painter) {
        self.add_command(Affine2dPaintCommand::DrawShape {
            shape: Affine2dCanvasShape::RingSector(ring_sector),

            painter,
        });
    }

    #[inline(always)]
    fn draw_triangle(&mut self, p0: Point2d, p1: Point2d, p2: Point2d, painter: Painter) {
        self.add_command(Affine2dPaintCommand::DrawShape {
            shape: Affine2dCanvasShape::Triangle(p0, p1, p2),

            painter,
        });
    }

    #[inline(always)]
    fn draw_polygon(&mut self, polygon: Vec<Point2d>, painter: Painter) {
        self.add_command(Affine2dPaintCommand::DrawShape {
            shape: Affine2dCanvasShape::Polygon(polygon),

            painter,
        });
    }

    #[inline(always)]
    fn draw_path(&mut self, path: Path, painter: Painter) {
        self.add_command(Affine2dPaintCommand::DrawShape {
            shape: Affine2dCanvasShape::Path(path),

            painter,
        });
    }

    #[inline(always)]
    fn stroke_line(&mut self, line: Line, painter: StrokePainter) {
        self.add_command(Affine2dPaintCommand::DrawShape {
            shape: Affine2dCanvasShape::Line(line),

            painter: Painter::Stroke(painter),
        });
    }

    #[inline(always)]
    fn stroke_arc(&mut self, arc: CircularArc, painter: StrokePainter) {
        self.add_command(Affine2dPaintCommand::DrawShape {
            shape: Affine2dCanvasShape::CircularArc(arc),

            painter: Painter::Stroke(painter),
        });
    }

    #[inline(always)]
    fn stroke_elliptical_arc(&mut self, arc: EllipticalArc, painter: StrokePainter) {
        self.add_command(Affine2dPaintCommand::DrawShape {
            shape: Affine2dCanvasShape::EllipticalArc(arc),

            painter: Painter::Stroke(painter),
        });
    }

    #[inline(always)]
    fn stroke_quad_bez(&mut self, seg: QuadBez, painter: StrokePainter) {
        self.add_command(Affine2dPaintCommand::DrawShape {
            shape: Affine2dCanvasShape::QuadBez(seg),

            painter: Painter::Stroke(painter),
        });
    }

    #[inline(always)]
    fn stroke_cubic_bez(&mut self, seg: CubicBez, painter: StrokePainter) {
        self.add_command(Affine2dPaintCommand::DrawShape {
            shape: Affine2dCanvasShape::CubicBez(seg),

            painter: Painter::Stroke(painter),
        });
    }

    #[inline(always)]
    fn fill_circle_segment(&mut self, segment: CircularArc, painter: FillPainter) {
        self.add_command(Affine2dPaintCommand::DrawShape {
            shape: Affine2dCanvasShape::CircularArc(segment),

            painter: Painter::Fill(painter),
        });
    }

    #[inline(always)]
    fn fill_rrect_padding(&mut self, rrect: RRect, padding: f32, painter: FillPainter) {
        self.add_command(Affine2dPaintCommand::DrawShape {
            shape: Affine2dCanvasShape::RRectPadding { rrect, padding },

            painter: Painter::Fill(painter),
        });
    }

    #[inline(always)]
    fn draw_image_rect(&mut self, image: Image, src: Rect, dst: Rect) {
        self.add_command(Affine2dPaintCommand::DrawShape {
            shape: Affine2dCanvasShape::Rect(dst),
            painter: Painter::Fill(FillPainter {
                fill: Fill::EvenOdd,
                brush: Brush::Image(image),
                transform: None,
            }),
        });
    }

    #[inline(always)]
    fn draw_paragraph(&mut self, paragraph: &Paragraph, offset: &[SingleLineOffset]) {
        self.add_command(Affine2dPaintCommand::DrawParagraph { paragraph, offset })
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
