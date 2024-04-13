use epgi_core::{
    foundation::{Asc, Canvas, Key, LayerProtocol, PaintContext, Protocol, Transform},
    tree::{
        ArcAnyLayerRenderObject, ArcChildLayerRenderObject, ArcChildRenderObject,
        ChildLayerOrFragment, ComposableChildLayer, ComposableUnadoptedLayer,
        LayerCompositionConfig, PaintResults,
    },
};
use peniko::{kurbo::Stroke, BrushRef};

use crate::{
    Affine2d, Affine2dCanvas, Affine2dCanvasShape, Affine2dEncoding, Affine2dPaintCommand,
    BlendMode, Fill, IntoKurbo, Painter, ParagraphLayout,
};

/// This is the serial version of paint context
pub struct VelloPaintContext<'a> {
    /// Vello does not store transform on stack (i.e. transform is absolute).
    /// However, our canvas design requires a transform stack. Hence we store it here.
    pub(crate) curr_config: LayerCompositionConfig<Affine2dCanvas>,
    // scene: &'a mut vello_encoding::Encoding,
    pub(crate) curr_fragment_encoding: Affine2dEncoding,
    pub(crate) results: &'a mut PaintResults<Affine2dCanvas>,
}

// We do not need to scan in a serial painter impl. Therefore a unit type with empty methods.
pub struct VelloPaintScanner;

pub const BLEND_SRC_OVER: BlendMode = BlendMode {
    mix: peniko::Mix::Normal,
    compose: peniko::Compose::SrcOver,
};

impl<'a> PaintContext for VelloPaintContext<'a> {
    type Canvas = Affine2dCanvas;

    #[inline(always)]
    fn add_command(&mut self, command: Affine2dPaintCommand) {
        use Affine2dPaintCommand::*;
        match command {
            DrawShape { shape, painter } => match painter {
                Painter::Fill(painter) => self.fill(
                    painter.fill,
                    self.curr_config.transform,
                    &painter.brush,
                    painter.transform,
                    shape,
                ),
                Painter::Stroke(painter) => self.stroke(
                    &painter.stroke,
                    self.curr_config.transform,
                    &painter.brush,
                    painter.transform,
                    shape,
                ),
            },
            ClipShape {
                shape,
                blend,
                alpha,
            } => self.push_layer(blend, alpha, self.curr_config.transform, shape),
            PopClip => self.pop_layer(),
            DrawParagraph { paragraph } => render_text(
                &mut self.curr_fragment_encoding,
                self.curr_config.transform,
                &paragraph,
            ),
        }
    }

    fn paint<P: Protocol<Canvas = Self::Canvas>>(
        &mut self,
        child: &ArcChildRenderObject<P>,
        offset: &P::Offset,
    ) {
        child.clone().paint(offset, self)
    }

    // fn add_layer(
    //     &mut self,
    //     layer: epgi_core::tree::ArcChildLayerRenderObject<Self::Canvas>,
    //     config: epgi_core::tree::LayerCompositionConfig<Self::Canvas>,
    // ) {
    //     if !self.curr_fragment_encoding.is_empty() {
    //         let encoding = std::mem::take(&mut self.curr_fragment_encoding);
    //         self.results
    //             .structured_children
    //             .push(StructuredChildLayerOrFragment::Fragment(encoding));
    //     }
    //     self.results
    //         .structured_children
    //         .push(StructuredChildLayerOrFragment::StructuredChild(
    //             ComposableChildLayer { config, layer },
    //         ));
    // }

    fn add_layer<P: LayerProtocol<Canvas = Affine2dCanvas>>(
        &mut self,
        layer: ArcChildLayerRenderObject<Self::Canvas>,
        offset: &P::Offset,
    ) {
        if !self.curr_fragment_encoding.is_empty() {
            let encoding = std::mem::take(&mut self.curr_fragment_encoding);
            self.results
                .children
                .push(ChildLayerOrFragment::Fragment(encoding));
        }
        self.results
            .children
            .push(ChildLayerOrFragment::Layer(ComposableChildLayer {
                config: P::offset_layer_composition_config(offset, &self.curr_config),
                layer,
            }));
    }

    fn add_orphan_layer<P: LayerProtocol<Canvas = Affine2dCanvas>>(
        &mut self,
        layer: ArcAnyLayerRenderObject,
        adopter_key: Asc<dyn Key>,
        offset: &P::Offset,
    ) {
        self.results.orphan_layers.push(ComposableUnadoptedLayer {
            config: P::offset_layer_composition_config(offset, &self.curr_config),
            layer,
            adopter_key,
        });
    }

    fn with_transform(
        &mut self,
        transform: <Self::Canvas as Canvas>::Transform,
        op: impl FnOnce(&mut Self),
    ) {
        let new_transform = Transform::mul(&self.curr_config.transform, &transform);
        let old_transform = std::mem::replace(&mut self.curr_config.transform, new_transform);
        op(self);
        self.curr_config.transform = old_transform;
    }
}

#[allow(unused_variables)]
impl PaintContext for VelloPaintScanner {
    type Canvas = Affine2dCanvas;

    fn add_command(&mut self, command: <Self::Canvas as Canvas>::PaintCommand<'_>) {}

    fn paint<P: Protocol<Canvas = Self::Canvas>>(
        &mut self,
        child: &ArcChildRenderObject<P>,
        offset: &P::Offset,
    ) {
    }

    fn add_layer<P: LayerProtocol<Canvas = Affine2dCanvas>>(
        &mut self,
        layer: ArcChildLayerRenderObject<Self::Canvas>,
        offset: &P::Offset,
    ) {
        todo!()
    }

    fn add_orphan_layer<P: LayerProtocol<Canvas = Affine2dCanvas>>(
        &mut self,
        layer: ArcAnyLayerRenderObject,
        adopter_key: Asc<dyn Key>,
        offset: &P::Offset,
    ) {
        todo!()
    }

    fn with_transform(
        &mut self,
        offset: <Self::Canvas as Canvas>::Transform,
        op: impl FnOnce(&mut Self),
    ) {
        todo!()
    }
}

impl<'a> VelloPaintContext<'a> {
    /// Pushes a new layer bound by the specifed shape and composed with
    /// previous layers using the specified blend mode.
    fn push_layer(
        &mut self,
        blend: impl Into<BlendMode>,
        alpha: f32,
        transform: Affine2d,
        shape: Affine2dCanvasShape,
    ) {
        let blend = blend.into();
        self.curr_fragment_encoding
            .encode_transform(transform.into());
        self.curr_fragment_encoding.encode_fill_style(Fill::NonZero);
        if !self.encode_shape(shape, true) {
            // If the layer shape is invalid, encode a valid empty path. This suppresses
            // all drawing until the layer is popped.
            self.curr_fragment_encoding
                .encode_shape(&peniko::kurbo::Rect::new(0.0, 0.0, 0.0, 0.0), true);
        }
        self.curr_fragment_encoding
            .encode_begin_clip(blend, alpha.clamp(0.0, 1.0));
    }

    /// Pops the current layer.
    fn pop_layer(&mut self) {
        self.curr_fragment_encoding.encode_end_clip();
    }

    /// Fills a shape using the specified style and brush.
    fn fill<'b>(
        &mut self,
        style: Fill,
        transform: Affine2d,
        brush: impl Into<BrushRef<'b>>,
        brush_transform: Option<Affine2d>,
        shape: Affine2dCanvasShape,
    ) {
        self.curr_fragment_encoding
            .encode_transform(transform.into());
        self.curr_fragment_encoding.encode_fill_style(style);
        if self.encode_shape(shape, true) {
            if let Some(brush_transform) = brush_transform {
                // Only encode transform after we can confirm shape encoding success
                if self
                    .curr_fragment_encoding
                    .encode_transform((transform * brush_transform).into())
                {
                    self.curr_fragment_encoding.swap_last_path_tags();
                }
            }
            self.curr_fragment_encoding.encode_brush(brush, 1.0);
        }
    }

    /// Strokes a shape using the specified style and brush.
    fn stroke<'b>(
        &mut self,
        style: &Stroke,
        transform: Affine2d,
        brush: impl Into<BrushRef<'b>>,
        brush_transform: Option<Affine2d>,
        shape: Affine2dCanvasShape,
    ) {
        // TODO: catch up with vello support for dash style
        self.curr_fragment_encoding
            .encode_transform(transform.into());
        self.curr_fragment_encoding.encode_stroke_style(style);
        if self.encode_shape(shape, false) {
            if let Some(brush_transform) = brush_transform {
                if self
                    .curr_fragment_encoding
                    .encode_transform((transform * brush_transform).into())
                {
                    self.curr_fragment_encoding.swap_last_path_tags();
                }
            }
            self.curr_fragment_encoding.encode_brush(brush, 1.0);
        }
    }

    #[inline(always)]
    fn encode_shape(&mut self, shape: Affine2dCanvasShape, is_fill: bool) -> bool {
        let encoding = &mut self.curr_fragment_encoding;
        use Affine2dCanvasShape::*;
        match shape {
            Rect(x) => encoding.encode_shape(&x.into_kurbo(), is_fill),
            RRect(x) => encoding.encode_shape(&x.into_kurbo(), is_fill),
            RRectPadding { rrect, padding } => todo!(),
            Circle(x) => encoding.encode_shape(&x.into_kurbo(), is_fill),
            Ellipse(x) => encoding.encode_shape(&x.into_kurbo(), is_fill),
            RingSector(x) => encoding.encode_shape(&x.into_kurbo(), is_fill),
            Triangle(_, _, _) => todo!(),
            Polygon(_) => todo!(),
            Path(x) => todo!(),
            Line(x) => encoding.encode_shape(&x.into_kurbo(), is_fill),
            CircularArc(x) => encoding.encode_shape(&x.into_kurbo(), is_fill),
            EllipticalArc(x) => encoding.encode_shape(&x.into_kurbo(), is_fill),
            QuadBez(x) => encoding.encode_shape(&x.into_kurbo(), is_fill),
            CubicBez(x) => encoding.encode_shape(&x.into_kurbo(), is_fill),
        }
    }
}

pub fn render_text(encoding: &mut Affine2dEncoding, transform: Affine2d, layout: &ParagraphLayout) {
    for line in layout.0.lines() {
        for glyph_run in line.glyph_runs() {
            let mut x = glyph_run.offset();
            let y = glyph_run.baseline();
            let run = glyph_run.run();
            let font = run.font();
            let font_size = run.font_size();
            let font = vello::peniko::Font::new(font.data().0.clone(), font.index());
            let style = glyph_run.style();
            let coords = run
                .normalized_coords()
                .iter()
                .map(|coord| vello::skrifa::instance::NormalizedCoord::from_bits(*coord))
                .collect::<Vec<_>>();
            vello::DrawGlyphs::new(encoding, &font)
                .brush(&style.brush.0)
                .transform(transform.into_kurbo())
                .font_size(font_size)
                .normalized_coords(&coords)
                .draw(
                    Fill::NonZero,
                    glyph_run.glyphs().map(|glyph| {
                        let gx = x + glyph.x;
                        let gy = y - glyph.y;
                        x += glyph.advance;
                        vello::glyph::Glyph {
                            id: glyph.id as _,
                            x: gx,
                            y: gy,
                        }
                    }),
                );
        }
    }
}
