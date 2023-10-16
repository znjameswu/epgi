use epgi_core::{
    foundation::{Asc, Canvas, PaintContext, Parallel, Protocol},
    tree::{
        ArcChildRenderObject, ChildRenderObject, ComposableChildLayer, LayerFragment, PaintResults,
        StructuredChildLayerOrFragment,
    },
};
use peniko::{kurbo::Shape, BrushRef, Stroke};

use crate::{
    Affine2d, Affine2dCanvas, Affine2dPaintCommand, BlendMode, Fill, Image, VelloEncoding,
};

/// This is the serial version of paint context
pub struct VelloPaintContext<'a> {
    curr_transform: Affine2d,
    // scene: &'a mut vello_encoding::Encoding,
    curr_fragment_encoding: VelloEncoding,
    results: &'a mut PaintResults<Affine2dCanvas>,
}

// We do not need to scan in a serial painter impl. Therefore a unit type with empty methods.
pub struct VelloPaintScanner;

impl<'a> PaintContext for VelloPaintContext<'a> {
    type Canvas = Affine2dCanvas;

    #[inline(always)]
    fn add_command(&mut self, command: Affine2dPaintCommand) {
        use Affine2dPaintCommand::*;
        match command {
            ClipPath { path } => todo!(),
            ClipRect { rect } => todo!(),
            ClipRRect { rect, radius } => todo!(),
            Arc {
                rect,
                start_angle,
                sweep_angle,
                use_center,
                painter,
            } => todo!(),
            Circle {
                center,
                radius,
                use_center,
                painter,
            } => todo!(),
            Color { color, blend_mode } => todo!(),
            DRRect {
                outer,
                inner,
                painter,
            } => todo!(),
            Image { image, top_left } => todo!(),
            ImageRect { image, src, dst } => todo!(),
            Line { p1, p2, painter } => todo!(),
            Oval { rect, painter } => todo!(),
            Paint { painter } => todo!(),
            Paragraph { paragraph, offset } => todo!(),
            Path { path, paint } => todo!(),
            Rect { rect } => todo!(),
            RRect { rect, radius } => todo!(),
        }
    }

    #[inline(always)]
    fn with_transform(
        &mut self,
        transform: <Self::Canvas as Canvas>::Transform,
        op: impl FnOnce(&mut Self),
    ) {
        let new_transform = self.curr_transform * transform;
        let old_transform = std::mem::replace(&mut self.curr_transform, new_transform);
        op(self);
        self.curr_transform = old_transform;
    }

    fn paint<P: Protocol<Canvas = Self::Canvas>>(
        &mut self,
        child: &dyn ChildRenderObject<P>,
        transform: &P::Transform,
    ) {
        child.paint(transform, self)
    }

    fn paint_multiple<'b, P: Protocol<Canvas = Self::Canvas>>(
        &'b mut self,
        child_transform_pairs: impl Parallel<Item = (ArcChildRenderObject<P>, &'b P::Transform)>,
    ) {
        child_transform_pairs
            .into_iter()
            .for_each(|(child, transform)| self.paint(child.as_ref(), transform))
    }

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

    fn add_command(&mut self, command: <Self::Canvas as Canvas>::PaintCommand) {}

    fn with_transform(
        &mut self,
        transform: <Self::Canvas as Canvas>::Transform,
        op: impl FnOnce(&mut Self),
    ) {
    }

    fn paint<P: Protocol<Canvas = Self::Canvas>>(
        &mut self,
        child: &dyn ChildRenderObject<P>,
        transform: &P::Transform,
    ) {
    }

    fn paint_multiple<'b, P: Protocol<Canvas = Self::Canvas>>(
        &'b mut self,
        child_transform_pairs: impl Parallel<Item = (ArcChildRenderObject<P>, &'b P::Transform)>,
    ) {
    }

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
        shape: &impl Shape,
    ) {
        let blend = blend.into();
        self.curr_fragment_encoding.encode_transform(transform);
        self.curr_fragment_encoding.encode_linewidth(-1.0);
        if !self.curr_fragment_encoding.encode_shape(shape, true) {
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
        shape: &impl Shape,
    ) {
        self.curr_fragment_encoding.encode_transform(transform);
        self.curr_fragment_encoding.encode_linewidth(match style {
            Fill::NonZero => -1.0,
            Fill::EvenOdd => -2.0,
        });
        if self.curr_fragment_encoding.encode_shape(shape, true) {
            if let Some(brush_transform) = brush_transform {
                if self
                    .curr_fragment_encoding
                    .encode_transform((transform * brush_transform))
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
        shape: &impl Shape,
    ) {
        self.curr_fragment_encoding.encode_transform(transform);
        self.curr_fragment_encoding.encode_linewidth(style.width);
        if self.curr_fragment_encoding.encode_shape(shape, false) {
            if let Some(brush_transform) = brush_transform {
                if self
                    .curr_fragment_encoding
                    .encode_transform(transform * brush_transform)
                {
                    self.curr_fragment_encoding.swap_last_path_tags();
                }
            }
            self.curr_fragment_encoding.encode_brush(brush, 1.0);
        }
    }

    /// Draws an image at its natural size with the given transform.
    fn draw_image(&mut self, image: &Image, transform: Affine2d) {
        self.fill(
            Fill::NonZero,
            transform,
            image,
            None,
            &peniko::kurbo::Rect::new(0.0, 0.0, image.width as f64, image.height as f64),
        );
    }

    // /// Returns a builder for encoding a glyph run.
    // fn draw_glyphs(&mut self, font: &Font) -> DrawGlyphs {
    //     DrawGlyphs::new(&mut self.scene, font)
    // }

    // /// Appends a fragment to the scene.
    // pub fn append(&mut self, fragment: &SceneFragment, transform: Option<Affine>) {
    //     self.scene.append(
    //         &fragment.data,
    //         &transform.map(|xform| Transform::from_kurbo(&xform)),
    //     );
    // }
}
