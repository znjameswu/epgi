use epgi_core::{
    foundation::{Canvas, LayerProtocol, Transform, TransformHitPosition},
    tree::{ArcChildRenderObject, ChildLayerOrFragment, LayerCompositionConfig, PaintResults},
};

use crate::{
    Affine2d, Affine2dEncoding, Affine2dPaintCommand, BoxOffset, VelloPaintContext,
    VelloPaintScanner,
};

pub type Point2d = BoxOffset;

#[derive(Clone)]
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
            children: Default::default(),
            orphan_layers: Default::default(),
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
        let new_child = ChildLayerOrFragment::Fragment(paint_ctx.curr_fragment_encoding);
        paint_results.children.push(new_child);
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

    fn inv(&self) -> Option<Self> {
        let det = self.0[0] * self.0[3] - self.0[1] * self.0[2];
        let inv_det = det.recip();
        if !inv_det.is_finite() {
            return None;
        }
        Some(Self([
            inv_det * self.0[3],
            -inv_det * self.0[1],
            -inv_det * self.0[2],
            inv_det * self.0[0],
            inv_det * (self.0[2] * self.0[5] - self.0[3] * self.0[4]),
            inv_det * (self.0[1] * self.0[4] - self.0[0] * self.0[5]),
        ]))
    }
}
