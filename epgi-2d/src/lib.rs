mod r#box;
mod paint;
mod vello;
mod text;

pub use paint::*;
pub use r#box::*;
pub use vello::*;
pub use text::*;

use epgi_core::foundation::Canvas;

pub type Affine2d = vello_encoding::Transform;

pub type Point2d = BoxOffset;

pub struct Affine2dCanvas;

impl Canvas for Affine2dCanvas {
    type Transform = Affine2d;

    type PaintCommand = Affine2dPaintCommand;

    type DefaultPaintContext<'a> = VelloPaintContext<'a>;

    type DefaultPaintScanner<'a> = VelloPaintScanner<'a>;
}
