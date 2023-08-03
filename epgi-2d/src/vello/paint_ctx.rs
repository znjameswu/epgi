use epgi_core::foundation::{Canvas, PaintContext};
use peniko::{kurbo::Shape, BrushRef};

use crate::{Affine2d, Affine2dCanvas, Affine2dPaintCommand, BlendMode, Fill, Image, Stroke};

pub struct VelloPaintContext<'a> {
    curr_transform: Affine2d,
    scene: &'a mut vello_encoding::Encoding,
}

pub struct VelloPaintScanner<'a> {
    a: &'a mut i32,
}

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

    fn with_layer(&mut self, op: impl FnOnce(&epgi_core::common::ArcParentLayer<Self::Canvas>)) {
        todo!()
    }
}

impl<'a> PaintContext for VelloPaintScanner<'a> {
    type Canvas = Affine2dCanvas;

    #[inline(always)]
    fn add_command(&mut self, command: <Self::Canvas as Canvas>::PaintCommand) {
        todo!()
    }

    fn with_transform(
        &mut self,
        transform: <Self::Canvas as Canvas>::Transform,
        op: impl FnOnce(&mut Self),
    ) {
        todo!()
    }

    fn with_layer(&mut self, op: impl FnOnce(&epgi_core::common::ArcParentLayer<Self::Canvas>)) {
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
        shape: &impl Shape,
    ) {
        let blend = blend.into();
        self.scene.encode_transform(transform);
        self.scene.encode_linewidth(-1.0);
        if !self.scene.encode_shape(shape, true) {
            // If the layer shape is invalid, encode a valid empty path. This suppresses
            // all drawing until the layer is popped.
            self.scene
                .encode_shape(&peniko::kurbo::Rect::new(0.0, 0.0, 0.0, 0.0), true);
        }
        self.scene.encode_begin_clip(blend, alpha.clamp(0.0, 1.0));
    }

    /// Pops the current layer.
    fn pop_layer(&mut self) {
        self.scene.encode_end_clip();
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
        self.scene.encode_transform(transform);
        self.scene.encode_linewidth(match style {
            Fill::NonZero => -1.0,
            Fill::EvenOdd => -2.0,
        });
        if self.scene.encode_shape(shape, true) {
            if let Some(brush_transform) = brush_transform {
                if self.scene.encode_transform((transform * brush_transform)) {
                    self.scene.swap_last_path_tags();
                }
            }
            self.scene.encode_brush(brush, 1.0);
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
        self.scene.encode_transform(transform);
        self.scene.encode_linewidth(style.width);
        if self.scene.encode_shape(shape, false) {
            if let Some(brush_transform) = brush_transform {
                if self.scene.encode_transform(transform * brush_transform) {
                    self.scene.swap_last_path_tags();
                }
            }
            self.scene.encode_brush(brush, 1.0);
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
