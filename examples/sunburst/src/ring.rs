mod adapter;
pub use adapter::*;

mod align;
pub use align::*;

mod center;
pub use center::*;

mod colored_ring;
pub use colored_ring::*;

mod constrained_ring;
pub use constrained_ring::*;

mod flex;
pub use flex::*;

mod padding;
pub use padding::*;

mod phantom_ring;
pub use phantom_ring::*;

mod slice;
pub use slice::*;

mod track;
pub use track::*;

use std::f32::consts::{PI, TAU};

use epgi_2d::{Affine2dCanvas, Point2d};
use epgi_core::{
    foundation::{Intrinsics, Protocol},
    tree::{ArcChildElementNode, ArcChildRenderObject, ArcChildWidget},
    Provider,
};

#[derive(Clone, Copy, Debug)]
pub struct RingProtocol {}

#[derive(derive_more::Mul, derive_more::Div, PartialEq, Clone, Copy, Debug)]
pub struct RingConstraints {
    pub min_dr: f32,
    pub max_dr: f32,
    pub min_dtheta: f32,
    pub max_dtheta: f32,
}

pub type ArcRingWidget = ArcChildWidget<RingProtocol>;
pub type ArcRingElementNode = ArcChildElementNode<RingProtocol>;
pub type ArcRingRenderObject = ArcChildRenderObject<RingProtocol>;
pub type RingProvider<T> = Provider<T, RingProtocol>;

impl RingConstraints {
    pub fn new_tight(dr: f32, dtheta: f32) -> Self {
        Self {
            min_dr: dr,
            max_dr: dr,
            min_dtheta: dtheta,
            max_dtheta: dtheta,
        }
    }

    pub fn new_tight_dr(dr: f32) -> Self {
        Self {
            min_dr: dr,
            max_dr: dr,
            min_dtheta: 0.0,
            max_dtheta: f32::INFINITY,
        }
    }

    pub fn new_tight_dtheta(dtheta: f32) -> Self {
        Self {
            min_dr: 0.0,
            max_dr: f32::INFINITY,
            min_dtheta: dtheta,
            max_dtheta: dtheta,
        }
    }

    pub fn new_tight_for(dr: Option<f32>, dtheta: Option<f32>) -> Self {
        Self {
            min_dr: dr.unwrap_or(0.0),
            max_dr: dr.unwrap_or(f32::INFINITY),
            min_dtheta: dtheta.unwrap_or(0.0),
            max_dtheta: dtheta.unwrap_or(f32::INFINITY),
        }
    }

    pub fn new_max_dr(dr: f32) -> Self {
        Self {
            min_dr: 0.0,
            max_dr: dr,
            min_dtheta: 0.0,
            max_dtheta: f32::INFINITY,
        }
    }

    pub fn new_max_dtheta(dtheta: f32) -> Self {
        Self {
            min_dr: 0.0,
            max_dr: f32::INFINITY,
            min_dtheta: 0.0,
            max_dtheta: dtheta,
        }
    }

    #[inline(always)]
    pub fn enforce(&self, other: &Self) -> Self {
        Self {
            min_dr: self.min_dr.clamp(other.min_dr, other.max_dr),
            max_dr: self.max_dr.clamp(other.min_dr, other.max_dr),
            min_dtheta: self.min_dtheta.clamp(other.min_dtheta, other.max_dtheta),
            max_dtheta: self.max_dtheta.clamp(other.min_dtheta, other.max_dtheta),
        }
    }

    pub fn biggest(&self) -> RingSize {
        self.constrain(RingSize {
            dr: f32::INFINITY,
            dtheta: f32::INFINITY,
        })
    }

    pub fn smallest(&self) -> RingSize {
        self.constrain(RingSize::ZERO)
    }

    pub fn is_tight(&self) -> Option<RingSize> {
        if self.min_dr == self.max_dr && self.min_dtheta == self.max_dtheta {
            Some(RingSize {
                dr: self.min_dr,
                dtheta: self.min_dtheta,
            })
        } else {
            None
        }
    }

    pub fn constrain(&self, size: RingSize) -> RingSize {
        RingSize {
            dr: size.dr.clamp(self.min_dr, self.max_dr),
            dtheta: size.dtheta.clamp(self.min_dtheta, self.max_dtheta),
        }
    }

    pub fn loosen(&self) -> Self {
        Self {
            min_dr: 0.0,
            max_dr: self.max_dr,
            min_dtheta: 0.0,
            max_dtheta: self.max_dtheta,
        }
    }

    pub fn tighten(&self, dr: Option<f32>, dtheta: Option<f32>) -> Self {
        let dr = dr.map(|dr| dr.clamp(self.min_dr, self.max_dr));
        let dtheta = dtheta.map(|dtheta| dtheta.clamp(self.min_dtheta, self.max_dtheta));
        Self {
            min_dr: dr.unwrap_or(self.min_dr),
            max_dr: dr.unwrap_or(self.max_dr),
            min_dtheta: dtheta.unwrap_or(self.min_dtheta),
            max_dtheta: dtheta.unwrap_or(self.max_dtheta),
        }
    }

    pub fn tighten_dr(&self, dr: f32) -> Self {
        let dr = dr.clamp(self.min_dr, self.max_dr);
        Self {
            min_dr: dr,
            max_dr: dr,
            min_dtheta: self.min_dtheta,
            max_dtheta: self.max_dtheta,
        }
    }

    pub fn tighten_dtheta(&self, dtheta: f32) -> Self {
        let dtheta = dtheta.clamp(self.min_dtheta, self.max_dtheta);
        Self {
            min_dr: self.min_dr,
            max_dr: self.max_dr,
            min_dtheta: dtheta,
            max_dtheta: dtheta,
        }
    }
}

impl Default for RingConstraints {
    fn default() -> Self {
        Self {
            min_dr: 0.0,
            max_dr: f32::INFINITY,
            min_dtheta: 0.0,
            max_dtheta: f32::INFINITY,
        }
    }
}

#[derive(
    derive_more::Add, derive_more::Mul, derive_more::Div, PartialEq, Default, Clone, Copy, Debug,
)]
pub struct RingOffset {
    pub r: f32,
    pub theta: f32,
}

impl RingOffset {
    pub const ZERO: Self = Self { r: 0.0, theta: 0.0 };
}

#[derive(derive_more::Mul, derive_more::Div, PartialEq, Clone, Copy, Debug)]
pub struct RingSize {
    pub dr: f32,
    pub dtheta: f32,
}

impl RingSize {
    pub const ZERO: Self = Self {
        dr: 0.0,
        dtheta: 0.0,
    };

    pub const INFINITY: Self = Self {
        dr: f32::INFINITY,
        dtheta: f32::INFINITY,
    };

    pub fn is_finite(&self) -> bool {
        self.dr.is_finite() && self.dtheta.is_finite()
    }
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
