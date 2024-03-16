use std::ops::Mul;

use crate::Point2d;

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
pub struct Affine2d(pub [f32; 6]);

impl Affine2d {
    pub const IDENTITY: Self = Affine2d([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    pub fn translation(&self) -> Point2d {
        Point2d {
            x: self.0[4],
            y: self.0[5],
        }
    }
}

impl Into<vello_encoding::Transform> for Affine2d {
    fn into(self) -> vello_encoding::Transform {
        vello_encoding::Transform {
            matrix: [self.0[0], self.0[1], self.0[2], self.0[3]],
            translation: [self.0[4], self.0[5]],
        }
    }
}

impl Mul<Point2d> for &Affine2d {
    type Output = Point2d;

    fn mul(self, rhs: Point2d) -> Self::Output {
        let a = [
            self.0[0] * rhs.x,
            self.0[1] * rhs.x,
            self.0[2] * rhs.y,
            self.0[3] * rhs.y,
        ];
        Point2d {
            x: a[0] + a[2] + self.0[5],
            y: a[1] + a[3] + self.0[6],
        }
    }
}

impl Mul<Point2d> for Affine2d {
    type Output = Point2d;

    fn mul(self, rhs: Point2d) -> Self::Output {
        (&self) * rhs
    }
}

impl Mul<&Affine2d> for &Affine2d {
    type Output = Affine2d;

    fn mul(self, rhs: &Affine2d) -> Self::Output {
        let a = [
            self.0[0] * rhs.0[0],
            self.0[1] * rhs.0[0],
            self.0[2] * rhs.0[1],
            self.0[3] * rhs.0[1],
            self.0[0] * rhs.0[2],
            self.0[1] * rhs.0[2],
            self.0[2] * rhs.0[3],
            self.0[3] * rhs.0[3],
            self.0[0] * rhs.0[4],
            self.0[1] * rhs.0[4],
            self.0[2] * rhs.0[5],
            self.0[3] * rhs.0[5],
        ];
        let b = [
            a[0] + a[2],
            a[1] + a[3],
            a[4] + a[6],
            a[5] + a[7],
            a[8] + a[10],
            a[9] + a[11],
        ];

        Affine2d([b[0], b[1], b[2], b[3], b[4] + self.0[4], b[5] + self.0[5]])
    }
}

impl Mul<Affine2d> for Affine2d {
    type Output = Affine2d;

    fn mul(self, rhs: Affine2d) -> Self::Output {
        (&self) * (&rhs)
    }
}
