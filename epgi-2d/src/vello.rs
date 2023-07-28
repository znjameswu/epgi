mod root;
pub use root::*;

use epgi_core::foundation::{Canvas, PaintingContext};

use crate::{Affine2d, Affine2dCanvas, Affine2dPrimitive};

pub struct VelloPaintingContext;

pub struct VelloPaintingScanner;

impl PaintingContext for VelloPaintingContext {
    type Canvas = Affine2dCanvas;

    fn add_command(&mut self, command: <Self::Canvas as Canvas>::PaintCommand) {
        todo!()
    }

    fn with_transform(
        &mut self,
        transform: <Self::Canvas as Canvas>::Transformation,
        op: impl FnOnce(&mut Self),
    ) {
        todo!()
    }
}

impl PaintingContext for VelloPaintingScanner {
    type Canvas = Affine2dCanvas;

    fn add_command(&mut self, command: <Self::Canvas as Canvas>::PaintCommand) {
        todo!()
    }

    fn with_transform(
        &mut self,
        transform: <Self::Canvas as Canvas>::Transformation,
        op: impl FnOnce(&mut Self),
    ) {
        todo!()
    }
}
