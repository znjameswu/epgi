use epgi_2d::{Affine2dCanvas, BoxConstraints, BoxOffset};
use epgi_core::foundation::{Intrinsics, Protocol};

#[derive(Clone, Copy, Debug)]
pub struct SingleLineProtocol;

impl Protocol for SingleLineProtocol {
    type Constraints = SingleLineConstraints;

    type Size = SingleLineSize;

    type Offset = SingleLineOffset;

    type Intrinsics = SingleLineIntrinsics;

    type Canvas = Affine2dCanvas;
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
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct MultiLineConstraints {
    first_line_max_width: f32,
    max_width: f32,
    last_line_append_advance: f32,
    max_height: f32,
}

#[derive(Clone, Debug)]
pub struct MultiLineSize {
    sizes: smallvec::SmallVec<[SingleLineSize; 1]>,
}

#[derive(Clone, Debug)]
pub struct MultiLineOffset {
    offsets: smallvec::SmallVec<[BoxOffset; 2]>,
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

#[macro_export]
macro_rules! MultiLineTransform {
    () => {};
}

MultiLineTransform!();
// pub struct Text {
//     str: Cow<'static, str>,
//     text_style:
// }

// pub struct SingleLineText {

// }
