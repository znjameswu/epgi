mod r#box;
mod paint;
mod text;
mod vello;

pub use paint::*;
pub use r#box::*;
pub use text::*;
pub use vello::*;

use epgi_core::foundation::Canvas;

pub type Affine2d = vello_encoding::Transform;

pub type Point2d = BoxOffset;

pub struct Affine2dCanvas;

impl Canvas for Affine2dCanvas {
    type Transform = Affine2d;

    type PaintCommand = Affine2dPaintCommand;

    type PaintContext<'a> = VelloPaintContext<'a>;

    type PaintScanner<'a> = VelloPaintScanner<'a>;

    type Encoding = VelloEncoding;
}
