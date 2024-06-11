use peniko::kurbo;

use crate::{
    Affine2d, Circle, CircularArc, CubicBez, Ellipse, EllipticalArc, Line, Point2d, QuadBez, RRect,
    Rect, RingSector,
};

pub trait IntoKurbo {
    type Output;
    fn into_kurbo(self) -> Self::Output;
}

impl IntoKurbo for Point2d {
    type Output = kurbo::Point;

    fn into_kurbo(self) -> Self::Output {
        kurbo::Point {
            x: self.x as _,
            y: self.y as _,
        }
    }
}

impl IntoKurbo for Affine2d {
    type Output = kurbo::Affine;

    fn into_kurbo(self) -> Self::Output {
        kurbo::Affine::new(self.0.map(|x| x as f64))
    }
}

impl IntoKurbo for Rect {
    type Output = kurbo::Rect;

    fn into_kurbo(self) -> Self::Output {
        kurbo::Rect {
            x0: self.l as _,
            y0: self.t as _,
            x1: self.r as _,
            y1: self.b as _,
        }
    }
}

impl IntoKurbo for RRect {
    type Output = kurbo::RoundedRect;

    fn into_kurbo(self) -> Self::Output {
        kurbo::RoundedRect::from_rect(
            self.rect.into_kurbo(),
            kurbo::RoundedRectRadii {
                top_left: self.radius.tl as _,
                top_right: self.radius.tr as _,
                bottom_right: self.radius.br as _,
                bottom_left: self.radius.bl as _,
            },
        )
    }
}

pub const KURBO_RECT_ALL: kurbo::Rect = kurbo::Rect {
    x0: f64::MIN,
    y0: f64::MIN,
    x1: f64::MAX,
    y1: f64::MAX,
};

impl IntoKurbo for Circle {
    type Output = kurbo::Circle;

    fn into_kurbo(self) -> Self::Output {
        kurbo::Circle {
            center: self.c.into_kurbo(),
            radius: self.r as _,
        }
    }
}

impl IntoKurbo for Ellipse {
    type Output = kurbo::Ellipse;

    fn into_kurbo(self) -> Self::Output {
        kurbo::Ellipse::from_affine(self.affine.into_kurbo())
    }
}

impl IntoKurbo for RingSector {
    type Output = kurbo::CircleSegment;

    fn into_kurbo(self) -> Self::Output {
        kurbo::CircleSegment {
            center: self.outer_cicle.c.into_kurbo(),
            outer_radius: self.outer_cicle.r as _,
            inner_radius: self.inner_radius as _,
            start_angle: self.start_angle as _,
            sweep_angle: self.sweep_angle as _,
        }
    }
}

impl IntoKurbo for Line {
    type Output = kurbo::Line;

    fn into_kurbo(self) -> Self::Output {
        kurbo::Line {
            p0: self.p0.into_kurbo(),
            p1: self.p1.into_kurbo(),
        }
    }
}

impl IntoKurbo for CircularArc {
    type Output = kurbo::Arc;

    fn into_kurbo(self) -> Self::Output {
        kurbo::Arc {
            center: self.circle.c.into_kurbo(),
            radii: (self.circle.r as _, self.circle.r as _).into(),
            start_angle: self.start_angle as _,
            sweep_angle: self.sweep_angle as _,
            x_rotation: 0.0,
        }
    }
}

// impl IntoKurbo for EllipticalArc {
//     type Output = kurbo::Arc;

//     fn into_kurbo(self) -> Self::Output {
//         let (th1, d1, d2, th2) = svd::svd(self.affine);
//         kurbo::Arc {
//             center: self.affine.translation().into_kurbo(),
//             radii: (d1 as _, d2 as _).into(),
//             start_angle: th2 as _,
//             sweep_angle: self.sweep_angle as _,
//             x_rotation: th1 as _,
//         }
//     }
// }

impl IntoKurbo for EllipticalArc {
    type Output = kurbo::Arc;

    fn into_kurbo(self) -> Self::Output {
        kurbo::Arc {
            center: self.c.into_kurbo(),
            radii: (self.r.0 as _, self.r.1 as _).into(),
            start_angle: self.start_angle as _,
            sweep_angle: self.sweep_angle as _,
            x_rotation: self.x_rotation as _,
        }
    }
}

// /// Source: https://github.com/linebender/kurbo/issues/250
// mod svd {
//     use crate::Affine2d;

//     /// Compute the singular value decomposition of the linear transformation (ignoring the
//     /// translation).
//     ///
//     /// All non-degenerate linear transformations can be represented as
//     ///
//     ///  1. a rotation about the origin.
//     ///  2. a scaling along the x and y axes
//     ///  3. another rotation about the origin
//     ///
//     /// composed together. Decomposing a 2x2 matrix in this way is called a "singular value
//     /// decomposition" and is written `U Σ V^T`, where U and V^T are orthogonal (rotations) and Σ
//     /// is a diagonal matrix (a scaling along the axes).
//     ///
//     /// Will return NaNs if the matrix (or equivalently the linear map) is singular.
//     ///
//     /// The return values correspond to the operations as they would be written, meaning if we
//     /// label the return value `(rot2, scale, rot1)`, `rot1` is performed first, followed by
//     /// `scale`, followed by `rot2`.
//     // Heavily influenced by
//     // https://scicomp.stackexchange.com/questions/8899/robust-algorithm-for-2-times-2-svd
//     pub(crate) fn svd(affine: Affine2d) -> (f32, f32, f32, f32) {
//         let (x, y, z, s2, c2) = rq(affine);
//         /*
//         println!(
//             "RQ = ( {x:+6.4}, {y:+6.4} )( {c2:+6.4}, {:+6.4} )\n     \
//                  (       0, {z:+6.4} )( {s2:+6.4}, {c2:+6.4} )",
//             s2
//         );
//         */
//         // Calculate tangent of rotation on R[x,y;0,z] to diagonalize R^T*R
//         let scalar = x.abs().max(y.abs()).recip();
//         let x_ = x * scalar;
//         let y_ = y * scalar;
//         let z_ = z * scalar;
//         let numer = (x_ - z_) * (x_ + z_) - y_ * y_;
//         let gamma = if numer == 0. { 1. } else { x_ * y_ };
//         let zeta = numer / gamma;
//         let t = 2. * zeta.signum() / (zeta.abs() + (zeta * zeta + 4.).sqrt());

//         // Calculate sines and cosines
//         let c1 = (t * t + 1.).sqrt().recip();
//         let s1 = c1 * t;

//         // Calculate U*S = R*rot(c1,s1)
//         let usa = c1 * x + s1 * y;
//         let usb = c1 * y - s1 * x;
//         let usc = s1 * z;
//         let usd = c1 * z;

//         // Update V = rot(c1,s1)^T*Q
//         let t = c1 * c2 + s1 * s2;
//         let s2 = c1 * s2 - c2 * s1;
//         let c2 = t;

//         // Separate U and S
//         let d1 = usa.hypot(usc);
//         let mut d2 = usb.hypot(usd);
//         let mut dmax = d1.max(d2);
//         let usmax1 = if d2 > d1 { usd } else { usa };
//         let usmax2 = if d2 > d1 { usb } else { -usc };

//         let signd1 = (x * z).signum();
//         dmax *= if d2 > d1 { signd1 } else { 1. };
//         d2 *= signd1;
//         let rcpdmax = dmax.recip();

//         let c1 = if dmax != 0. { usmax1 * rcpdmax } else { 1. };
//         let s1 = if dmax != 0. { -usmax2 * rcpdmax } else { 0. };

//         // TODO consider return sin/cos of the angle, to avoid unnecessary change between polar
//         // space and cartesian space TODO atan2?
//         let th1 = s1.asin().signum() * c1.acos();
//         //assert_approx_eq!(f64, th1.sin(), s1, epsilon = 1e-13);
//         //assert_approx_eq!(f64, th1.cos(), c1, epsilon = 1e-13);
//         let th2 = s2.asin().signum() * c2.acos();
//         //assert_approx_eq!(f64, th2.sin(), s2, epsilon = 1e-13);
//         //assert_approx_eq!(f64, th2.cos(), c2, epsilon = 1e-13);
//         (th1, d1, d2, th2)
//     }

//     /// Perform an RQ decomposition (useful for accurate SVD decomposition).
//     ///
//     /// R = upper triangular, Q = orthonormal (i.e. rotation)
//     ///
//     /// Returns (`x`, `y`, `z`, `s`, `c`) where
//     ///
//     /// ```text
//     /// R = | x  y | Q = | c -s |
//     ///     | 0  z |     | s  c |
//     /// ```
//     // From https://scicomp.stackexchange.com/questions/8899/robust-algorithm-for-2-times-2-svd/28506#28506
//     fn rq(affine: Affine2d) -> (f32, f32, f32, f32, f32) {
//         let a = affine.0[0];
//         let b = affine.0[2];
//         let c = affine.0[1];
//         let d = affine.0[3];

//         if c == 0. {
//             return (a, b, d, 0., 1.);
//         }

//         // First scale the bottom row to have a max abs value of 1
//         let maxcd = c.abs().max(d.abs());
//         let maxcd_recip = maxcd.recip();
//         let c = c * maxcd_recip;
//         let d = d * maxcd_recip;

//         // Don't use hypot here because we know that the max abs value of c and d is 1, so save a
//         // branch
//         let z = (c * c + d * d).sqrt();
//         let zm1 = z.recip();
//         let x = (a * d - b * c) * zm1;
//         let y = (a * c + b * d) * zm1;
//         let sin = c * zm1;
//         let cos = d * zm1;
//         // at the end we undo the operation on the bottom row, by scaling z by the inverse of the
//         // scales to c and d. The math checks out.
//         (x, y, z * maxcd, sin, cos)
//     }
// }

impl IntoKurbo for QuadBez {
    type Output = kurbo::QuadBez;

    fn into_kurbo(self) -> Self::Output {
        kurbo::QuadBez {
            p0: self.p0.into_kurbo(),
            p1: self.p1.into_kurbo(),
            p2: self.p2.into_kurbo(),
        }
    }
}

impl IntoKurbo for CubicBez {
    type Output = kurbo::CubicBez;

    fn into_kurbo(self) -> Self::Output {
        kurbo::CubicBez {
            p0: self.p0.into_kurbo(),
            p1: self.p1.into_kurbo(),
            p2: self.p2.into_kurbo(),
            p3: self.p3.into_kurbo(),
        }
    }
}
