use epgi_core::{
    foundation::{Canvas, PaintResults, Protocol},
    nodes::Provider,
    tree::{ArcChildRenderObject, ArcParentLayer},
};

use crate::{Affine2dPaintCommand, BoxOffset, VelloEncoding, VelloPaintContext, VelloPaintScanner};

pub type Affine2d = vello_encoding::Transform;

pub type Point2d = BoxOffset;

pub struct Affine2dCanvas;

impl Canvas for Affine2dCanvas {
    type Transform = Affine2d;

    type PaintCommand = Affine2dPaintCommand;

    type PaintContext<'a> = VelloPaintContext<'a>;

    type PaintScanner<'a> = VelloPaintScanner;

    type Encoding = VelloEncoding;

    fn composite(
        dst: &mut Self::Encoding,
        src: &Self::Encoding,
        transform: Option<&Self::Transform>,
    ) {
        // TODO: Vello API design issue.
        dst.append(src, &transform.cloned())
    }

    fn paint_layer(
        layer: ArcParentLayer<Self>,
        scan: impl FnOnce(&mut Self::PaintScanner<'_>),
        paint: impl FnOnce(&mut Self::PaintContext<'_>),
    ) {
        todo!()
    }

    fn clear(this: &mut Self::Encoding) {
        this.reset(true)
    }

    fn paint_render_object<P: Protocol<Canvas = Self>>(
        render_object: ArcChildRenderObject<P>,
    ) -> PaintResults<Self> {
        todo!()
    }
}
