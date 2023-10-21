use epgi_core::{
    foundation::{Asc, Canvas, PaintContext, Parallel, Protocol},
    tree::{
        ArcChildRenderObject, ChildRenderObject, ComposableChildLayer, LayerFragment, PaintResults,
        StructuredChildLayerOrFragment,
    },
};
use peniko::{kurbo::Shape, BrushRef, Stroke};

use crate::{
    into_kurbo_rrect, Affine2d, Affine2dCanvas, Affine2dPaintCommand, BlendMode, Fill, Image,
    IntoKurbo, Painter, VelloEncoding, KURBO_RECT_ALL,
};

/// This is the serial version of paint context
pub struct VelloPaintContext<'a> {
    /// Vello does not store transform on stack (i.e. transform is absolute).
    /// However, our canvas design requires a transform stack. Hence we store it here.
    pub(crate) curr_transform: Affine2d,
    // scene: &'a mut vello_encoding::Encoding,
    pub(crate) curr_fragment_encoding: VelloEncoding,
    pub(crate) results: &'a mut PaintResults<Affine2dCanvas>,
}

// We do not need to scan in a serial painter impl. Therefore a unit type with empty methods.
pub struct VelloPaintScanner;

const BLEND_SRC_OVER: BlendMode = BlendMode {
    mix: peniko::Mix::Normal,
    compose: peniko::Compose::SrcOver,
};

impl<'a> PaintContext for VelloPaintContext<'a> {
    type Canvas = Affine2dCanvas;

    #[inline(always)]
    fn add_command(&mut self, command: Affine2dPaintCommand) {
        use Affine2dPaintCommand::*;
        match command {
            ClipPath { path } => {
                self.push_layer(BLEND_SRC_OVER, 1.0, None, &path.path_els.as_slice())
            }
            ClipRect { rect } => self.push_layer(BLEND_SRC_OVER, 1.0, None, &rect.into_kurbo()),
            ClipRRect { rect, radius } => {
                self.push_layer(BLEND_SRC_OVER, 1.0, None, &into_kurbo_rrect(rect, radius))
            }
            Arc {
                rect,
                start_angle,
                sweep_angle,
                use_center,
                painter,
            } => {
                if use_center {
                    assert!(
                        rect.is_sqaure(),
                        "Elliptical circle segment support is unimplemented"
                    );
                    let shape = peniko::kurbo::CircleSegment {
                        center: rect.center().into_kurbo(),
                        outer_radius: (rect.height() / 2.0) as _,
                        inner_radius: 0.0,
                        start_angle: start_angle as _,
                        sweep_angle: sweep_angle as _,
                    };
                    match painter {
                        Painter::Fill(painter) => self.fill(
                            painter.fill,
                            None,
                            &painter.brush,
                            painter.transform,
                            &shape,
                        ),
                        Painter::Stroke(painter) => self.stroke(
                            &painter.stroke,
                            None,
                            &painter.brush,
                            painter.transform,
                            &shape,
                        ),
                    }
                } else {
                    let shape = peniko::kurbo::Arc {
                        center: rect.center().into_kurbo(),
                        radii: peniko::kurbo::Vec2 {
                            x: (rect.width() / 2.0) as _,
                            y: (rect.height() / 2.0) as _,
                        },
                        start_angle: start_angle as _,
                        sweep_angle: sweep_angle as _,
                        x_rotation: 0.0,
                    };
                    match painter {
                        Painter::Fill(painter) => self.fill(
                            painter.fill,
                            None,
                            &painter.brush,
                            painter.transform,
                            &shape,
                        ),
                        Painter::Stroke(painter) => self.stroke(
                            &painter.stroke,
                            None,
                            &painter.brush,
                            painter.transform,
                            &shape,
                        ),
                    }
                }
            }
            Circle {
                center,
                radius,
                painter,
            } => {
                let shape = peniko::kurbo::Circle {
                    center: center.into_kurbo(),
                    radius: radius as _,
                };
                match painter {
                    Painter::Fill(painter) => self.fill(
                        painter.fill,
                        None,
                        &painter.brush,
                        painter.transform,
                        &shape,
                    ),
                    Painter::Stroke(painter) => self.stroke(
                        &painter.stroke,
                        None,
                        &painter.brush,
                        painter.transform,
                        &shape,
                    ),
                }
            }
            Color { color } => self.fill(
                Fill::EvenOdd,
                None,
                &peniko::Brush::Solid(color),
                None,
                &KURBO_RECT_ALL,
            ),
            DRRect {
                outer,
                inner,
                painter,
            } => todo!(),
            Image { image, top_left } => todo!(),
            ImageRect { image, src, dst } => todo!(),
            Line { p1, p2, painter } => self.stroke(
                &painter.stroke,
                None,
                &painter.brush,
                painter.transform,
                &peniko::kurbo::Line {
                    p0: p1.into_kurbo(),
                    p1: p2.into_kurbo(),
                },
            ),
            Oval { rect, painter } => {
                let shape = peniko::kurbo::Ellipse::new(
                    rect.center().into_kurbo(),
                    peniko::kurbo::Vec2 {
                        x: (rect.width() / 2.0) as _,
                        y: (rect.height() / 2.0) as _,
                    },
                    0.0,
                );
                match painter {
                    Painter::Fill(painter) => self.fill(
                        painter.fill,
                        None,
                        &painter.brush,
                        painter.transform,
                        &shape,
                    ),
                    Painter::Stroke(painter) => self.stroke(
                        &painter.stroke,
                        None,
                        &painter.brush,
                        painter.transform,
                        &shape,
                    ),
                }
            }
            Paint { painter } => todo!(),
            DrawParagraph { paragraph, offset } => todo!(),
            Path { path, painter } => {
                let shape = path.path_els.as_slice();
                match painter {
                    Painter::Fill(painter) => self.fill(
                        painter.fill,
                        None,
                        &painter.brush,
                        painter.transform,
                        &shape,
                    ),
                    Painter::Stroke(painter) => self.stroke(
                        &painter.stroke,
                        None,
                        &painter.brush,
                        painter.transform,
                        &shape,
                    ),
                }
            }
            Rect { rect, painter } => {
                let shape = rect.into_kurbo();
                match painter {
                    Painter::Fill(painter) => self.fill(
                        painter.fill,
                        None,
                        &painter.brush,
                        painter.transform,
                        &shape,
                    ),
                    Painter::Stroke(painter) => self.stroke(
                        &painter.stroke,
                        None,
                        &painter.brush,
                        painter.transform,
                        &shape,
                    ),
                }
            }
            RRect {
                rect,
                radius,
                painter,
            } => {
                let shape = into_kurbo_rrect(rect, radius);
                match painter {
                    Painter::Fill(painter) => self.fill(
                        painter.fill,
                        None,
                        &painter.brush,
                        painter.transform,
                        &shape,
                    ),
                    Painter::Stroke(painter) => self.stroke(
                        &painter.stroke,
                        None,
                        &painter.brush,
                        painter.transform,
                        &shape,
                    ),
                }
            }
            PopClip => self.pop_layer(),
            Transform { transform } => {},
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
        transform: Option<Affine2d>,
        shape: &impl Shape,
    ) {
        let blend = blend.into();
        if let Some(transform) = transform {
            self.curr_transform = transform;
        }
        self.curr_fragment_encoding
            .encode_transform(self.curr_transform);
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
        transform: Option<Affine2d>,
        brush: impl Into<BrushRef<'b>>,
        brush_transform: Option<Affine2d>,
        shape: &impl Shape,
    ) {
        if let Some(transform) = transform {
            self.curr_transform = transform;
        }
        self.curr_fragment_encoding
            .encode_transform(self.curr_transform);
        self.curr_fragment_encoding.encode_linewidth(match style {
            Fill::NonZero => -1.0,
            Fill::EvenOdd => -2.0,
        });
        if self.curr_fragment_encoding.encode_shape(shape, true) {
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
        transform: Option<Affine2d>,
        brush: impl Into<BrushRef<'b>>,
        brush_transform: Option<Affine2d>,
        shape: &impl Shape,
    ) {
        if let Some(transform) = transform {
            self.curr_transform = transform;
        }
        self.curr_fragment_encoding
            .encode_transform(self.curr_transform);
        self.curr_fragment_encoding.encode_linewidth(style.width);
        if self.curr_fragment_encoding.encode_shape(shape, false) {
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

    /// Draws an image at its natural size with the given transform.
    fn draw_image(&mut self, image: &Image, transform: Option<Affine2d>) {
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
