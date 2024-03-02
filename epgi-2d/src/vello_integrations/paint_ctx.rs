use epgi_core::{
    foundation::{Canvas, PaintContext, Protocol},
    tree::{
        ArcChildRenderObject, ComposableChildLayer, PaintResults, StructuredChildLayerOrFragment,
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
    pub(crate) curr_transform: Affine2d,
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
            DrawShape {
                shape,
                transform,
                painter,
            } => match painter {
                Painter::Fill(painter) => self.fill(
                    painter.fill,
                    transform,
                    &painter.brush,
                    painter.transform,
                    shape,
                ),
                Painter::Stroke(painter) => self.stroke(
                    &painter.stroke,
                    transform,
                    &painter.brush,
                    painter.transform,
                    shape,
                ),
            },
            ClipShape {
                shape,
                transform,
                blend,
                alpha,
            } => self.push_layer(blend, alpha, transform, shape),
            PopClip => self.pop_layer(),
            DrawParagraph {
                paragraph,
                transform,
            } => render_text(&mut self.curr_fragment_encoding, transform, &paragraph),
        }
    }

    fn paint<P: Protocol<Canvas = Self::Canvas>>(
        &mut self,
        child: &ArcChildRenderObject<P>,
        transform: &P::Transform,
    ) {
        child.clone().paint(transform, self)
    }

    // fn paint_multiple<'b, P: Protocol<Canvas = Self::Canvas>>(
    //     &'b mut self,
    //     child_transform_pairs: impl IntoIterator<Item = (&'b, ArcChildRenderObject<P>, &'b P::Transform)>,
    // ) {
    //     child_transform_pairs
    //         .into_iter()
    //         .for_each(|(child, transform)| self.paint(child, transform))
    // }

    fn add_layer(&mut self, op: impl FnOnce() -> ComposableChildLayer<Self::Canvas>) {
        if !self.curr_fragment_encoding.is_empty() {
            let encoding = std::mem::take(&mut self.curr_fragment_encoding);
            self.results
                .structured_children
                .push(StructuredChildLayerOrFragment::Fragment(encoding));
        }
        self.results
            .structured_children
            .push(StructuredChildLayerOrFragment::StructuredChild(op()));
    }
}

impl PaintContext for VelloPaintScanner {
    type Canvas = Affine2dCanvas;

    fn add_command(&mut self, command: <Self::Canvas as Canvas>::PaintCommand<'_>) {}

    fn paint<P: Protocol<Canvas = Self::Canvas>>(
        &mut self,
        child: &ArcChildRenderObject<P>,
        transform: &P::Transform,
    ) {
    }

    // fn paint_multiple<'b, P: Protocol<Canvas = Self::Canvas>>(
    //     &'b mut self,
    //     child_transform_pairs: impl Container<Item = (ArcChildRenderObject<P>, &'b P::Transform)>,
    // ) {
    // }

    fn add_layer(&mut self, op: impl FnOnce() -> ComposableChildLayer<Self::Canvas>) {}
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
        self.curr_fragment_encoding.encode_transform(transform);
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
        self.curr_fragment_encoding.encode_transform(transform);
        self.curr_fragment_encoding.encode_fill_style(style);
        if self.encode_shape(shape, true) {
            if let Some(brush_transform) = brush_transform {
                // Only encode transform after we can confirm shape encoding success
                if self
                    .curr_fragment_encoding
                    .encode_transform((self.curr_transform * brush_transform))
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
        self.curr_fragment_encoding.encode_transform(transform);
        self.curr_fragment_encoding.encode_stroke_style(style);
        if self.encode_shape(shape, false) {
            if let Some(brush_transform) = brush_transform {
                if self
                    .curr_fragment_encoding
                    .encode_transform(self.curr_transform * brush_transform)
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
                .transform(transform.to_kurbo())
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
