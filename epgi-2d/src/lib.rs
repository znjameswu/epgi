mod r#box;
mod paint_command;
mod paint_context;
mod vello;
mod text;

pub use paint_command::*;
pub use paint_context::*;
pub use r#box::*;
pub use vello::*;
pub use text::*;

use epgi_core::foundation::Canvas;

pub type Affine2d = vello_encoding::Transform;

pub type Point2d = BoxOffset;

pub struct Affine2dCanvas;

impl Canvas for Affine2dCanvas {
    type Transformation = Affine2d;

    type PaintCommand = Affine2dPrimitive;

    type DefaultPaintingContext = VelloPaintingContext;

    type DefaultPaintingScanner = VelloPaintingScanner;
}
