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

impl BoxConstraints {
    pub fn new_tight(width: f32, height: f32) -> Self {
        Self {
            min_width: width,
            max_width: width,
            min_height: height,
            max_height: height,
        }
    }
    #[inline(always)]
    pub fn enforce(&self, other: &Self) -> Self {
        Self {
            min_width: self.min_width.clamp(other.min_width, other.max_width),
            max_width: self.max_width.clamp(other.min_width, other.max_width),
            min_height: self.min_height.clamp(other.min_height, other.max_height),
            max_height: self.max_height.clamp(other.min_height, other.max_height),
        }
    }

    pub fn biggest(&self) -> BoxSize {
        self.constrains(BoxSize {
            width: f32::INFINITY,
            height: f32::INFINITY,
        })
    }

    pub fn smallest(&self) -> BoxSize {
        self.constrains(BoxSize::ZERO)
    }
}

impl Default for BoxConstraints {
    fn default() -> Self {
        Self {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BoxSize {
    pub width: f32,
    pub height: f32,
}

impl BoxSize {
    const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };
}

#[derive(Clone, Copy, Debug)]
pub struct BoxOffset {
    pub x: f32,
    pub y: f32,
}

impl BoxOffset {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
}

impl From<[f32; 2]> for BoxOffset {
    fn from(value: [f32; 2]) -> Self {
        Self {
            x: value[0],
            y: value[1],
        }
    }
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

    type Transform = Affine2d;

    type Canvas = Affine2dCanvas;

    #[inline]
    fn transform_canvas(
        transform: &Self::Transform,
        transform_canvas: &<Self::Canvas as epgi_core::foundation::Canvas>::Transform,
    ) -> Self::Transform {
        *transform_canvas * *transform
    }
}
