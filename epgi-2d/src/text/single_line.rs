use epgi_core::foundation::{Intrinsics, Protocol};

use crate::{Affine2dCanvas, BoxConstraints, BoxOffset, BoxProtocol, BoxSize, Point2d};

#[derive(Clone, Copy, Debug)]
pub struct SingleLineProtocol;

impl Protocol for SingleLineProtocol {
    type Constraints = SingleLineConstraints;

    type Size = SingleLineSize;

    type Offset = SingleLineOffset;

    type Intrinsics = SingleLineIntrinsics;

    type Canvas = Affine2dCanvas;

    fn position_in_shape(
        position: &Point2d,
        offset: &SingleLineOffset,
        size: &SingleLineSize,
    ) -> bool {
        BoxProtocol::position_in_shape(
            position,
            offset,
            &BoxSize {
                width: size.advance,
                height: size.above + size.below,
            },
        )
    }
}

pub type SingleLineConstraints = BoxConstraints;

#[derive(Clone, Copy, Debug)]
pub struct SingleLineSize {
    // /// Typographic ascent.
    // pub ascent: f32,
    // /// Typographic descent.
    // pub descent: f32,
    // /// Typographic leading. // Which is distributed 50-50 in above & below
    // pub leading: f32,
    /// Full advance of the line.
    pub advance: f32,
    /// Total height above the baseline
    pub above: f32,
    /// Total height below the baseline
    pub below: f32,
    // /// Advance of trailing whitespace.
    // pub trailing_whitespace: f32,
}

pub type SingleLineOffset = BoxOffset;

#[derive(Clone, Copy, Debug)]
pub struct SingleLineIntrinsics;

impl Intrinsics for SingleLineIntrinsics {
    fn eq_tag(&self, other: &Self) -> bool {
        true
    }

    fn eq_param(&self, other: &Self) -> bool {
        true
    }
}
