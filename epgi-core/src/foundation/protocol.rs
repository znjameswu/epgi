use std::fmt::Debug;

use crate::rendering::{Affine2d, Affine2dPrimitive, CanvasAffine2d, PaintingContext};

pub trait Protocol: std::fmt::Debug + Copy + Clone + Send + Sync + 'static {
    type Constraints: Constraints<Self::Size>;
    type Size: Debug + Clone + Send + Sync + 'static;
    type Offset: Debug + Clone + Send + Sync + 'static;
    type Intrinsics: Intrinsics;
    type CanvasTransformation: Debug + Clone + Send + Sync + 'static;
    type Canvas: Canvas;
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

pub trait Canvas: Sized {
    type Transformation: Debug + Clone + Send + Sync;
    type PaintCommands: Send + Sync;

    type PaintingContext: PaintingContext<Self>;
    type PaintingContextScanner: PaintingContext<Self>;
}

pub struct Affine2dCanvas;

impl Canvas for Affine2dCanvas {
    type Transformation = Affine2d;

    type PaintCommands = Affine2dPrimitive;

    type PaintingContext = Affine2dPaintingContext;

    type PaintingContextScanner = Affine2dPaintingContextScanner;
}

pub struct Affine2dPaintingContext;

pub struct Affine2dPaintingContextScanner;

impl PaintingContext<Affine2dCanvas> for Affine2dPaintingContext {}

impl PaintingContext<Affine2dCanvas> for Affine2dPaintingContextScanner {}

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
pub enum BoxIntrinsics {
    MinWidth { height: f32, res: Option<f32> },
    MaxWidth { height: f32, res: Option<f32> },
    MinHeight { width: f32, res: Option<f32> },
    MaxHeight { width: f32, res: Option<f32> },
}

pub trait Intrinsics: Debug + Send + Sync {
    fn eq_tag(&self, other: &Self) -> bool;
    fn eq_param(&self, other: &Self) -> bool;
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

struct TagEq<T: Intrinsics>(T);

impl<T> PartialEq<Self> for TagEq<T>
where
    T: Intrinsics,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_tag(&other.0)
    }
}

impl<T> Eq for TagEq<T>
where
    T: Intrinsics,
{
    fn assert_receiver_is_total_eq(&self) {}
}

struct ParamEq<T: Intrinsics>(T);

impl<T> PartialEq<Self> for ParamEq<T>
where
    T: Intrinsics,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_param(&other.0)
    }
}

impl Protocol for BoxProtocol {
    type Constraints = BoxConstraints;

    type Size = BoxSize;

    type Offset = BoxOffset;

    type Intrinsics = BoxIntrinsics;

    type CanvasTransformation = Affine2d;

    type Canvas = Affine2dCanvas;
}
