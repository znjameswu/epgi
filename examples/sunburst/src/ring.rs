mod adapter;
pub use adapter::*;

use std::f32::consts::{PI, TAU};

use epgi_2d::{Affine2dCanvas, Point2d};
use epgi_core::foundation::{Intrinsics, Protocol};

#[derive(Clone, Copy, Debug)]
pub struct RingProtocol;

#[derive(derive_more::Mul, derive_more::Div, PartialEq, Clone, Copy, Debug)]
pub struct RingConstraints {
    pub min_dr: f32,
    pub max_dr: f32,
    pub min_dtheta: f32,
    pub max_dtheta: f32,
}

#[derive(PartialEq, Default, Clone, Copy, Debug)]
pub struct RingOffset {
    pub r: f32,
    pub theta: f32,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct RingSize {
    pub dr: f32,
    pub dtheta: f32,
}

#[derive(Debug)]
pub struct RingIntrinsics;

impl Intrinsics for RingIntrinsics {
    fn eq_tag(&self, other: &Self) -> bool {
        true
    }

    fn eq_param(&self, other: &Self) -> bool {
        true
    }
}

impl Protocol for RingProtocol {
    type Constraints = RingConstraints;
    type Offset = RingOffset;
    type Size = RingSize;
    type Intrinsics = RingIntrinsics;
    type Canvas = Affine2dCanvas;

    fn position_in_shape(position: &Point2d, offset: &RingOffset, size: &RingSize) -> bool {
        let Point2d { x, y } = *position;
        let RingOffset {
            r: r1,
            theta: theta1,
        } = *offset;
        let RingSize { dr, dtheta } = *size;
        let r2 = r1 + dr;
        let r_sq = x * x + y * y;

        let r1_norm = r1.max(0.0);
        let r2_norm = r2.max(0.0);
        if (r_sq - r1_norm * r1_norm) * (r_sq - r2_norm * r2_norm) <= 0.0 {
            //NOT XOR
            return false;
        }

        let dtheta_abs = dtheta.abs();
        if dtheta_abs >= TAU {
            // TAU=2*PI
            return true;
        }
        let theta2 = theta1 + dtheta;

        let (theta_start, theta_end) = if dtheta >= 0.0 {
            (theta1, theta2)
        } else {
            (theta2, theta1)
        };

        let (sin_theta_start, cos_theta_start) = theta_start.sin_cos();
        let (sin_theta_end, cos_theta_end) = theta_end.sin_cos();
        let cross_start = y * cos_theta_start - x * sin_theta_start;
        let cross_end = y * cos_theta_end - x * sin_theta_end;

        return if dtheta_abs <= PI {
            cross_start >= 0.0 && cross_end <= 0.0
        } else {
            cross_start >= 0.0 || cross_end <= 0.0
        };
    }
}
