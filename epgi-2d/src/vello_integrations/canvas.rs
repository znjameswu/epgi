use epgi_core::{
    foundation::{Canvas, LayerProtocol, Transform, TransformHitPosition},
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
        let a = [
            self.matrix[0] * input.x,
            self.matrix[1] * input.x,
            self.matrix[2] * input.y,
            self.matrix[3] * input.y,
        ];
        Point2d {
            x: a[0] + a[2] + self.translation[0],
            y: a[1] + a[3] + self.translation[1],
        }
    }
}

impl Transform<Affine2dCanvas> for Affine2d {
    fn mul(&self, other: &Self) -> Self {
        let a = [
            self.matrix[0] * other.matrix[0],
            self.matrix[1] * other.matrix[0],
            self.matrix[2] * other.matrix[1],
            self.matrix[3] * other.matrix[1],
            self.matrix[0] * other.matrix[2],
            self.matrix[1] * other.matrix[2],
            self.matrix[2] * other.matrix[3],
            self.matrix[3] * other.matrix[4],
            self.matrix[0] * other.translation[0],
            self.matrix[1] * other.translation[0],
            self.matrix[2] * other.translation[1],
            self.matrix[3] * other.translation[2],
        ];
        let b = [
            a[0] + a[2],
            a[1] + a[3],
            a[4] + a[6],
            a[5] + a[7],
            a[8] + a[10],
            a[9] + a[11],
        ];

        Self {
            matrix: [b[0], b[1], b[2], b[3]],
            translation: [b[4] + self.translation[0], b[5] + self.translation[1]],
        }
    }

    fn identity() -> Self {
        Affine2d::IDENTITY
    }
}
