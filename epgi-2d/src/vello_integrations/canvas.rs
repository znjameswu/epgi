use epgi_core::{
    foundation::{Canvas, LayerProtocol, Transform},
    tree::{ArcChildRenderObject, PaintResults, StructuredChildLayerOrFragment},
};

use crate::{
    Affine2dEncoding, Affine2dPaintCommand, BoxOffset, VelloPaintContext, VelloPaintScanner,
};

pub type Affine2d = vello_encoding::Transform;

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
            curr_transform: Affine2d::IDENTITY,
            curr_fragment_encoding: Affine2dEncoding::new(),
            results: &mut paint_results,
        };
        for render_object in render_objects {
            render_object.paint(&<P::Canvas as Canvas>::identity_transform(), &mut paint_ctx);
        }
        // Save the recordings on the tail
        let new_child = StructuredChildLayerOrFragment::Fragment(paint_ctx.curr_fragment_encoding);
        paint_results.structured_children.push(new_child);
        paint_results
    }

    fn new_encoding() -> Self::Encoding {
        Affine2dEncoding::new()
    }

    fn transform_hit_position(
        transform: &Self::Transform,
        hit_position: &Self::HitPosition,
    ) -> Self::HitPosition {
        todo!()
    }

    fn identity_transform() -> Self::Transform {
        Affine2d::IDENTITY
    }

    fn mul_transform_ref(a: &Self::Transform, b: &Self::Transform) -> Self::Transform {
        todo!()
    }
}

impl Transform<Affine2dCanvas, Affine2dCanvas> for Affine2d {
    fn transform(&self, input: &Point2d) -> <Affine2dCanvas as Canvas>::HitPosition {
        todo!()
    }
}
