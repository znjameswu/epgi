use epgi_core::{
    foundation::{Canvas, Identity, Protocol},
    tree::{ArcChildRenderObject, PaintResults},
};

use crate::{
    Affine2dEncoding, Affine2dPaintCommand, BoxOffset, VelloPaintContext, VelloPaintScanner,
};

pub type Affine2d = vello_encoding::Transform;

pub type Point2d = BoxOffset;

pub struct Affine2dCanvas;

impl Canvas for Affine2dCanvas {
    type Transform = Affine2d;

    type PaintCommand = Affine2dPaintCommand;

    type PaintContext<'a> = VelloPaintContext<'a>;

    type PaintScanner<'a> = VelloPaintScanner;

    type Encoding = Affine2dEncoding;

    fn composite_encoding(
        dst: &mut Self::Encoding,
        src: &Self::Encoding,
        transform: Option<&Self::Transform>,
    ) {
        // TODO: Vello API design issue.
        dst.append(src, &transform.cloned())
    }

    // fn paint_layer(
    //     layer: ArcParentLayer<Self>,
    //     scan: impl FnOnce(&mut Self::PaintScanner<'_>),
    //     paint: impl FnOnce(&mut Self::PaintContext<'_>),
    // ) {
    //     todo!()
    // }

    fn clear(this: &mut Self::Encoding) {
        this.reset(true)
    }

    fn paint_render_objects<P: Protocol<Canvas = Self>>(
        render_objects: impl IntoIterator<Item = ArcChildRenderObject<P>>,
    ) -> PaintResults<Self> {
        let mut paint_results = PaintResults {
            structured_children: Default::default(),
            detached_children: Default::default(),
        };
        let mut paint_ctx = VelloPaintContext {
            curr_transform: Affine2d::IDENTITY,
            curr_fragment_encoding: Affine2dEncoding::new(),
            results: &mut paint_results,
        };
        for render_object in render_objects {
            render_object.paint(&<P::Transform as Identity>::IDENTITY, &mut paint_ctx);
        }
        paint_results
    }

    fn new_encoding() -> Self::Encoding {
        Affine2dEncoding::new()
    }
}
