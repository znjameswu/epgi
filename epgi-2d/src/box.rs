use epgi_core::foundation::{Constraints, Intrinsics, Protocol};

use crate::{Affine2d, Affine2dCanvas};

#[derive(Clone, Copy, Debug)]
pub struct BoxProtocol {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxConstraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl Constraints<BoxSize> for BoxConstraints {
    fn is_tight(&self) -> bool {
        self.min_width == self.max_width && self.min_height == self.max_height
    }

    fn constrains(&self, size: BoxSize) -> BoxSize {
        BoxSize {
            width: size.width.clamp(self.min_width, self.max_width),
            height: size.height.clamp(self.min_height, self.max_height),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BoxSize {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct BoxOffset {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug)]
pub enum BoxIntrinsics {
    MinWidth { height: f32, res: Option<f32> },
    MaxWidth { height: f32, res: Option<f32> },
    MinHeight { width: f32, res: Option<f32> },
    MaxHeight { width: f32, res: Option<f32> },
}

impl Intrinsics for BoxIntrinsics {
    fn eq_tag(&self, other: &Self) -> bool {
        use BoxIntrinsics::*;
        match (self, other) {
            (MinWidth { .. }, MinWidth { .. })
            | (MaxWidth { .. }, MaxWidth { .. })
            | (MinHeight { .. }, MinHeight { .. })
            | (MaxHeight { .. }, MaxHeight { .. }) => true,
            _ => false,
        }
    }

    fn eq_param(&self, other: &Self) -> bool {
        use BoxIntrinsics::*;
        match (self, other) {
            (MinWidth { height: x, .. }, MinWidth { height: y, .. })
            | (MaxWidth { height: x, .. }, MaxWidth { height: y, .. })
            | (MinHeight { width: x, .. }, MinHeight { width: y, .. })
            | (MaxHeight { width: x, .. }, MaxHeight { width: y, .. }) => x == y,
            _ => false,
        }
    }
}

impl Protocol for BoxProtocol {
    type Constraints = BoxConstraints;

    type Size = BoxSize;

    type Offset = BoxOffset;

    type Intrinsics = BoxIntrinsics;

    type SelfTransform = Affine2d;

    type Canvas = Affine2dCanvas;
}
