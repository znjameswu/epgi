use epgi_core::foundation::{Canvas, PaintingContext};

use crate::{Affine2d, Affine2dPrimitive};

pub trait Affine2dPaintingContextExt {}

impl<T> Affine2dPaintingContextExt for T
where
    T: PaintingContext,
    T::Canvas: Canvas<Transformation = Affine2d, PaintCommand = Affine2dPrimitive>,
{
    
}
