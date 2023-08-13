mod r#box;
mod paint;
mod text;
mod vello;

pub use paint::*;
pub use r#box::*;
pub use text::*;
pub use vello::*;

use epgi_core::{foundation::Canvas, nodes::Provider, tree::ArcParentLayer};

pub type Affine2d = vello_encoding::Transform;

pub type Point2d = BoxOffset;

pub struct Affine2dCanvas;

impl Canvas for Affine2dCanvas {
    type Transform = Affine2d;

    type PaintCommand = Affine2dPaintCommand;

    type PaintContext<'a> = VelloPaintContext<'a>;

    type PaintScanner<'a> = VelloPaintScanner<'a>;

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
        scan: impl FnOnce(Self::PaintScanner<'_>),
        paint: impl FnOnce(Self::PaintContext<'_>),
    ) {
        todo!()
    }

    fn clear(this: &mut Self::Encoding) {
        this.reset(true)
    }
}

pub type BoxProvider<T> = Provider<T, BoxProtocol>;
