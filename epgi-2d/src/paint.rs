use std::ops::BitAnd;

use epgi_core::foundation::{Canvas, PaintContext};
pub use peniko::{
    BlendMode, Brush, Cap, Color, ColorStops, Dashes, Extend, Fill, Format, Gradient, GradientKind,
    Image, Join, Stroke,
};

use crate::{Affine2d, BoxOffset, BoxSize, Paragraph, Point2d};

pub enum Affine2dPaintCommand {
    // TODO: New clip should stack on top of existing clip
    ClipPath {
        path: Path,
    },
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
    Image {
        image: Image,
        top_left: Point2d,
        // TODO: ImageFilter, FilterQuality, ColorFilter, InvertColors, MaskFilter
    },
    // ImageNine, // TODO
    ImageRect {
        image: Image,
        src: Rect,
        dst: Rect,
        // TODO: ImageFilter, FilterQuality, ColorFilter, InvertColors, MaskFilter
    },
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
    Path {
        path: Path,
        paint: Painter,
    },
    // Picture,
    // Points, //https://stackoverflow.com/a/56896362
    Rect {
        rect: Rect,
    },
    RRect {
        rect: Rect,
        radius: RRectRadius,
    },
    // Shadow, // TODO
    // Vertices, // TODO
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

impl Rect {
    pub fn new(left: f32, top: f32, right: f32, botton: f32) -> Self {
        Self {
            left,
            top,
            right,
            botton,
        }
    }
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

pub struct Path {}

pub trait Affine2dPaintContextExt {
    fn clip_path(&mut self, path: Path);
    fn clip_rect(&mut self, rect: Rect);
    fn clip_rrect(&mut self, rect: Rect, radius: RRectRadius);
    fn draw_arc(
        &mut self,
        rect: Rect,
        start_angle: f32,
        sweep_angle: f32,
        use_center: bool,
        painter: Painter,
    );
    fn draw_circle(&mut self, center: Point2d, radius: f32, use_center: bool, painter: Painter);
    fn draw_color(&mut self, color: Color, blend_mode: BlendMode);
    fn draw_drrect(&mut self, outer: RRect, inner: RRect, painter: Painter);
    fn draw_image(&mut self, image: Image, top_left: Point2d);
    fn draw_image_rect(&mut self, image: Image, src: Rect, dst: Rect);
    fn draw_line(&mut self, p1: Point2d, p2: Point2d, painter: StrokePainter);
    fn draw_oval(&mut self, rect: Rect, painter: Painter);
    fn draw_paint(&mut self, painter: FillPainter);
    fn draw_paragraph(&mut self, paragraph: Paragraph, offset: BoxOffset);
    fn draw_path(&mut self, path: Path, paint: Painter);
    fn draw_rect(&mut self, rect: Rect);
    fn draw_rrect(&mut self, rect: Rect, radius: RRectRadius);
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
    fn draw_circle(&mut self, center: Point2d, radius: f32, use_center: bool, painter: Painter) {
        self.add_command(Affine2dPaintCommand::Circle {
            center,
            radius,
            use_center,
            painter,
        })
    }
    #[inline(always)]
    fn draw_color(&mut self, color: Color, blend_mode: BlendMode) {
        self.add_command(Affine2dPaintCommand::Color { color, blend_mode })
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
    fn draw_paragraph(&mut self, paragraph: Paragraph, offset: BoxOffset) {
        self.add_command(Affine2dPaintCommand::Paragraph { paragraph, offset })
    }
    #[inline(always)]
    fn draw_path(&mut self, path: Path, paint: Painter) {
        self.add_command(Affine2dPaintCommand::Path { path, paint })
    }
    #[inline(always)]
    fn draw_rect(&mut self, rect: Rect) {
        self.add_command(Affine2dPaintCommand::Rect { rect })
    }
    #[inline(always)]
    fn draw_rrect(&mut self, rect: Rect, radius: RRectRadius) {
        self.add_command(Affine2dPaintCommand::RRect { rect, radius })
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