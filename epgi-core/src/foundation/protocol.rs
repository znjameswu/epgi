use std::fmt::Debug;

use crate::painting::{Affine2d, CanvasAffine2d};

pub trait Protocol: std::fmt::Debug + Copy + Clone + Send + Sync + 'static {
    type Constraints: Constraints<Self::Size>;
    type Size: Debug + Clone + Send + Sync + 'static;
    type Offset: Debug + Clone + Send + Sync + 'static;
    type Intrinsics<'a>: Debug + Send + Sync;
    type CanvasTransformation: Debug + Clone + Send + Sync + 'static;
    type Canvas: Send + Sync;
    // fn point_in_area(
    //     size: Self::Size,
    //     transform: Self::CanvasTransformation,
    //     point_on_canvas: BoxOffset,
    // ) -> bool;
}

pub trait Constraints<Size>: Debug + PartialEq + Clone + Send + Sync + 'static {
    fn is_tight(&self) -> bool;
    fn constrains(&self, size: Size) -> Size;
}

pub trait Canvas {
    type RecoderSize: Clone + Send + Sync + 'static;
    type RecoderOffset: Clone + Send + Sync + 'static;
}

//// Sample implementation of BoxProtocol, which is fundamental to the crate as a result of the rect canvas from WebGPU

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
pub enum BoxIntrinsics<'a> {
    MinWidth {
        height: f32,
        res: &'a mut Option<f32>,
    },
    MaxWidth {
        height: f32,
        res: &'a mut Option<f32>,
    },
    MinHeight {
        width: f32,
        res: &'a mut Option<f32>,
    },
    MaxHeight {
        width: f32,
        res: &'a mut Option<f32>,
    },
}

impl Protocol for BoxProtocol {
    type Constraints = BoxConstraints;

    type Size = BoxSize;

    type Offset = BoxOffset;

    type Intrinsics<'a> = BoxIntrinsics<'a>;

    type CanvasTransformation = Affine2d;

    type Canvas = CanvasAffine2d;
}
