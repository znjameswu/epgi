pub use vello::peniko::{
    BlendMode, Brush, Cap, Color, ColorStops, Dashes, Extend, Format, Gradient, GradientKind,
    Image, Join, Stroke,
};



pub enum Affine2dPrimitiveTag {
    ClipPath,
    ClipRect,
    ClipRRect,
    Arc,
    Atlas,
    Circle,
    Color,
    DRRect,
    Image,
    ImageNine,
    ImageRect,
    Line,
    Oval,
    Paint,
    Paragraph,
    Path,
    // Picture,
    Points,
    Rect,
    RRect,
    Shadow,
    Vertices,
}



// // TODO
// #[derive(Clone, Copy, Debug)]
// pub struct Affine2d;

// pub struct CanvasAffine2d<'a> {
//     transform: Affine2d,
//     scene: vello::SceneBuilder<'a>,
// }

// impl<'a> CanvasAffine2d<'a> {
    // /// Pushes a new layer bound by the specifed shape and composed with
    // /// previous layers using the specified blend mode.
    // fn push_layer(
    //     &mut self,
    //     blend: impl Into<BlendMode>,
    //     alpha: f32,
    //     transform: Affine2d,
    //     shape: &impl Shape,
    // ) {
    //     let blend = blend.into();
    //     self.scene.encode_transform(transform);
    //     self.scene.encode_linewidth(-1.0);
    //     if !self.scene.encode_shape(shape, true) {
    //         // If the layer shape is invalid, encode a valid empty path. This suppresses
    //         // all drawing until the layer is popped.
    //         self.scene
    //             .encode_shape(&Rect::new(0.0, 0.0, 0.0, 0.0), true);
    //     }
    //     self.scene.encode_begin_clip(blend, alpha.clamp(0.0, 1.0));
    // }

    // /// Pops the current layer.
    // fn pop_layer(&mut self) {
    //     self.scene.encode_end_clip();
    // }

    // /// Fills a shape using the specified style and brush.
    // fn fill<'b>(
    //     &mut self,
    //     style: Fill,
    //     transform: Affine2d,
    //     brush: impl Into<BrushRef<'b>>,
    //     brush_transform: Option<Affine2d>,
    //     shape: &impl Shape,
    // ) {
    //     self.scene.encode_transform(transform);
    //     self.scene.encode_linewidth(match style {
    //         Fill::NonZero => -1.0,
    //         Fill::EvenOdd => -2.0,
    //     });
    //     if self.scene.encode_shape(shape, true) {
    //         if let Some(brush_transform) = brush_transform {
    //             if self.scene.encode_transform((transform * brush_transform)) {
    //                 self.scene.swap_last_path_tags();
    //             }
    //         }
    //         self.scene.encode_brush(brush, 1.0);
    //     }
    // }

    // /// Strokes a shape using the specified style and brush.
    // fn stroke<'b>(
    //     &mut self,
    //     style: &Stroke,
    //     transform: Affine2d,
    //     brush: impl Into<BrushRef<'b>>,
    //     brush_transform: Option<Affine2d>,
    //     shape: &impl Shape,
    // ) {
    //     self.scene.encode_transform(transform);
    //     self.scene.encode_linewidth(style.width);
    //     if self.scene.encode_shape(shape, false) {
    //         if let Some(brush_transform) = brush_transform {
    //             if self.scene.encode_transform(transform * brush_transform) {
    //                 self.scene.swap_last_path_tags();
    //             }
    //         }
    //         self.scene.encode_brush(brush, 1.0);
    //     }
    // }

    // /// Draws an image at its natural size with the given transform.
    // fn draw_image(&mut self, image: &Image, transform: Affine2d) {
    //     self.fill(
    //         Fill::NonZero,
    //         transform,
    //         image,
    //         None,
    //         &Rect::new(0.0, 0.0, image.width as f64, image.height as f64),
    //     );
    // }

    // /// Returns a builder for encoding a glyph run.
    // fn draw_glyphs(&mut self, font: &Font) -> DrawGlyphs {
    //     DrawGlyphs::new(&mut self.scene, font)
    // }

    // // /// Appends a fragment to the scene.
    // // pub fn append(&mut self, fragment: &SceneFragment, transform: Option<Affine>) {
    // //     self.scene.append(
    // //         &fragment.data,
    // //         &transform.map(|xform| Transform::from_kurbo(&xform)),
    // //     );
    // // }
// }
