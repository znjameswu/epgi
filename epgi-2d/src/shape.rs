use std::ops::BitAnd;

use crate::{Affine2d, BoxOffset, BoxSize, Point2d};

pub struct Line {
    pub p0: Point2d,
    pub p1: Point2d,
}

pub struct Rect {
    pub l: f32,
    pub t: f32,
    pub r: f32,
    pub b: f32,
}

impl Rect {
    pub fn new_ltrb(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            l: left,
            t: top,
            r: right,
            b: bottom,
        }
    }

    pub fn new_center(center: Point2d, size: BoxSize) -> Self {
        Self {
            l: center.x - size.width / 2.0,
            t: center.y - size.height / 2.0,
            r: center.x + size.width / 2.0,
            b: center.y + size.height / 2.0,
        }
    }

    pub fn new_ltwh(left: f32, top: f32, width: f32, height: f32) -> Self {
        Self {
            l: left,
            t: top,
            r: left + width,
            b: top + height,
        }
    }

    pub fn new_point_size(point: Point2d, size: BoxSize) -> Self {
        Self {
            l: point.x,
            t: point.y,
            r: point.x + size.width,
            b: point.y + size.height,
        }
    }

    pub fn new_vertices(p1: Point2d, p2: Point2d) -> Self {
        Self {
            l: f32::min(p1.x, p2.x),
            t: f32::min(p1.y, p2.y),
            r: f32::max(p1.x, p2.x),
            b: f32::max(p1.y, p2.y),
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

    pub fn contains(&self, point: &Point2d) -> bool {
        point.x >= self.l && point.x <= self.r && point.y >= self.t && point.y <= self.b
    }
}

impl BitAnd<BoxSize> for BoxOffset {
    type Output = Rect;

    fn bitand(self, rhs: BoxSize) -> Self::Output {
        Rect::new_point_size(self, rhs)
    }
}

pub struct RRect {
    pub rect: Rect,
    pub radius: Box<RRectRadius>,
}

pub struct RRectRadius {
    pub tl: f32,
    pub tr: f32,
    pub bl: f32,
    pub br: f32,
    // pub tl_t: f32,
    // pub tl_l: f32,
    // pub tr_t: f32,
    // pub tr_r: f32,
    // pub bl_b: f32,
    // pub bl_l: f32,
    // pub br_b: f32,
    // pub br_r: f32,
}

pub struct RRectBorder {
    pub l: f32,
    pub t: f32,
    pub r: f32,
    pub b: f32,
    pub radius: Box<RRectRadius>,
    pub linewidth_inner: f32,
}

pub struct Circle {
    pub c: Point2d,
    pub r: f32,
}

pub struct Ellipse {
    pub affine: Affine2d,
}

pub struct RingSector {
    pub outer_cicle: Circle,
    pub inner_radius: f32,
    pub start_angle: f32,
    pub sweep_angle: f32,
}

pub struct CircularArc {
    pub circle: Circle,
    pub start_angle: f32,
    pub sweep_angle: f32,
}

// pub struct EllipticalArc {
//     pub sweep_angle: f32,
//     pub affine: Affine2d,
// }

pub struct EllipticalArc {
    pub c: Point2d,
    pub r: (f32, f32),
    pub start_angle: f32,
    pub sweep_angle: f32,
    pub x_rotation: f32,
}

pub struct QuadBez {
    pub p0: Point2d,
    pub p1: Point2d,
    pub p2: Point2d,
}

pub struct CubicBez {
    pub p0: Point2d,
    pub p1: Point2d,
    pub p2: Point2d,
    pub p3: Point2d,
}
