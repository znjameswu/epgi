use epgi_core::foundation::{Intrinsics, Protocol};

use crate::{Affine2dCanvas, Point2d, SingleLineOffset, SingleLineProtocol, SingleLineSize};

/// A simplistic multi-line text protocol.
///
/// This protocol does not support kerning between sibling nodes. Kerning, considering its inherent complexity, should be done a dedicated text layout crate, rather than handling to the general GUI layout protocols.
///
/// This protocol also does not support precise line break resolution. A spec-compliant line break algorithm would at least require resolving char class pairs to determine line break candidates. Here we require the line break to be pre-computed and assigned to nodes as properties.
#[derive(Clone, Copy, Debug)]
pub struct MultiLineProtocol;

impl Protocol for MultiLineProtocol {
    type Constraints = MultiLineConstraints;

    type Size = MultiLineSize;

    type Offset = MultiLineOffset;

    type Intrinsics = MultiLineIntrinsics;

    type Canvas = Affine2dCanvas;

    fn position_in_shape(
        position: &Point2d,
        offset: &MultiLineOffset,
        size: &MultiLineSize,
    ) -> bool {
        debug_assert_eq!(
            offset.offsets.len(),
            size.sizes.len(),
            "Multi-line layout should generate the same number of offset entry as the line count"
        );
        std::iter::zip(offset.offsets.iter(), size.sizes.iter())
            .any(|(offset, size)| SingleLineProtocol::position_in_shape(position, offset, size))
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct MultiLineConstraints {
    pub first_line_existing_advance: f32,
    pub max_width: f32,
    pub last_line_append_advance: f32,
    pub max_height: f32,
}

#[derive(Clone, Debug)]
pub struct MultiLineSize {
    pub sizes: Vec<SingleLineSize>,
}

#[derive(Clone, Debug)]
pub struct MultiLineOffset {
    pub offsets: Vec<SingleLineOffset>,
}

#[derive(Clone, Copy, Debug)]
pub enum MultiLineIntrinsics {
    MinWidth { height: f32, res: Option<f32> },
    MaxWidth { height: f32, res: Option<f32> },
    MinHeight { width: f32, res: Option<f32> },
    MaxHeight { width: f32, res: Option<f32> },
    AdvanceBeforeFirstBreak { res: Option<f32> },
    EndWithBreak { res: Option<bool> },
}

impl Intrinsics for MultiLineIntrinsics {
    fn eq_tag(&self, other: &Self) -> bool {
        todo!()
    }

    fn eq_param(&self, other: &Self) -> bool {
        todo!()
    }
}
