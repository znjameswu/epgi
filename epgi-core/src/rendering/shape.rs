// use std::ops::BitAnd;

// use crate::foundation::{BoxOffset, BoxSize};







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
