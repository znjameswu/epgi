use epgi_core::{
    foundation::{Canvas, LayerProtocol, Transform, TransformHitPosition},
    tree::{
        ArcChildRenderObject, LayerCompositionConfig, PaintResults, StructuredChildLayerOrFragment,
    },
};

use crate::{
    Affine2d, Affine2dEncoding, Affine2dPaintCommand, BoxOffset, VelloPaintContext,
    VelloPaintScanner,
};

pub type Point2d = BoxOffset;

pub struct Affine2dCanvas;

impl Canvas for Affine2dCanvas {
    type Transform = Affine2d;

    type PaintCommand<'a> = Affine2dPaintCommand<'a>;

    type PaintContext<'a> = VelloPaintContext<'a>;

    type PaintScanner<'a> = VelloPaintScanner;

    type Encoding = Affine2dEncoding;

    type HitPosition = Point2d;

    fn composite_encoding(
        dst: &mut Self::Encoding,
        src: &Self::Encoding,
        transform: Option<&Self::Transform>,
    ) {
        // TODO: Vello API design issue.
        dst.append(src, &transform.map(|transform| (*transform).into()))
    }

    // fn paint_layer(
    //     layer: ArcParentLayer<Self>,
    //     scan: impl FnOnce(&mut Self::PaintScanner<'_>),
    //     paint: impl FnOnce(&mut Self::PaintContext<'_>),
    // ) {
    //     todo!()
    // }

    fn clear(this: &mut Self::Encoding) {
        this.reset()
    }

    fn paint_render_objects<P: LayerProtocol<Canvas = Self>>(
        render_objects: impl IntoIterator<Item = ArcChildRenderObject<P>>,
    ) -> PaintResults<Self> {
        let mut paint_results = PaintResults {
            structured_children: Default::default(),
            detached_children: Default::default(),
        };
        let mut paint_ctx = VelloPaintContext {
            curr_config: LayerCompositionConfig {
                transform: Affine2d::IDENTITY,
            },
            curr_fragment_encoding: Affine2dEncoding::new(),
            results: &mut paint_results,
        };
        for render_object in render_objects {
            render_object.paint(&P::zero_offset(), &mut paint_ctx);
        }
        // Save the recordings on the tail
        let new_child = StructuredChildLayerOrFragment::Fragment(paint_ctx.curr_fragment_encoding);
        paint_results.structured_children.push(new_child);
        paint_results
    }

    fn new_encoding() -> Self::Encoding {
        Affine2dEncoding::new()
    }
}

impl TransformHitPosition<Affine2dCanvas, Affine2dCanvas> for Affine2d {
    fn transform(&self, input: &Point2d) -> Point2d {
        self * (*input)
    }
}

impl Transform<Affine2dCanvas> for Affine2d {
    fn mul(&self, other: &Self) -> Self {
        self * other
    }

    fn identity() -> Self {
        Affine2d::IDENTITY
    }
}
