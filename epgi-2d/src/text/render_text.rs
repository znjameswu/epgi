use vello::kurbo::Stroke;

use crate::{
    Affine2d, Affine2dPaintContextExt, Fill, IntoKurbo, Line, MultiLineOffset, Paragraph, Point2d,
    StrokePainter, VelloPaintContext,
};

// Adapted from masonry::text_helper.rs
pub(crate) fn render_text<'a>(
    paint_ctx: &mut VelloPaintContext<'a>,
    // scratch_scene: &mut Scene,
    transform: Affine2d,
    paragraph: &Paragraph, // layout: &Layout<TextBrush>,
    offset: &MultiLineOffset,
) {
    // scratch_scene.reset();
    debug_assert_eq!(
        offset.offsets.len(),
        paragraph.layout.len(),
        "A paragraph should receive the same number of offsets as its line count"
    );
    for (line, offset) in std::iter::zip(paragraph.layout.lines(), offset.offsets.iter()) {
        let metrics = line.metrics();
        let baseline_shift = offset.y - (metrics.baseline - metrics.ascent - metrics.leading * 0.5);
        for glyph_run in line.glyph_runs() {
            // let mut x = glyph_run.offset();
            // let y = glyph_run.baseline(); // The glyph baseline is generated from line baseline as the glyph is generated from the iterator
            let mut x = offset.x;
            let y = glyph_run.baseline() + baseline_shift;
            let run = glyph_run.run();
            let font = run.font();
            let font_size = run.font_size();
            let synthesis = run.synthesis();
            let glyph_xform = synthesis
                .skew()
                .map(|angle| vello::kurbo::Affine::skew(angle.to_radians().tan() as f64, 0.0));
            let style = glyph_run.style();
            let coords = run
                .normalized_coords()
                .iter()
                .map(|coord| vello::skrifa::instance::NormalizedCoord::from_bits(*coord))
                .collect::<Vec<_>>();
            // let text_brush = match &style.brush {
            //     TextBrush::Normal(text_brush) => text_brush,
            //     TextBrush::Highlight { text, fill } => {
            //         encoding.fill(
            //             Fill::EvenOdd,
            //             transform,
            //             fill,
            //             None,
            //             &Rect::from_origin_size(
            //                 (
            //                     glyph_run.offset() as f64,
            //                     // The y coordinate is on the baseline. We want to draw from the top of the line
            //                     // (Note that we are in a y-down coordinate system)
            //                     (y - metrics.ascent - metrics.leading) as f64,
            //                 ),
            //                 (glyph_run.advance() as f64, metrics.size() as f64),
            //             ),
            //         );

            //         text
            //     }
            // };
            vello::DrawGlyphs::new(&mut paint_ctx.curr_fragment_encoding, &font)
                .brush(&style.brush.0)
                .transform(transform.into_kurbo())
                .glyph_transform(glyph_xform)
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
            if let Some(underline) = &style.underline {
                let underline_brush = &underline.brush;
                let run_metrics = glyph_run.run().metrics();
                let offset = match underline.offset {
                    Some(offset) => offset,
                    None => run_metrics.underline_offset,
                };
                let width = match underline.size {
                    Some(size) => size,
                    None => run_metrics.underline_size,
                };
                // The `offset` is the distance from the baseline to the *top* of the underline
                // so we move the line down by half the width
                // Remember that we are using a y-down coordinate system
                let y = glyph_run.baseline() - offset + width / 2.;

                paint_ctx.stroke_line(
                    Line {
                        p0: Point2d {
                            x: glyph_run.offset(),
                            y,
                        },
                        p1: Point2d {
                            x: glyph_run.offset() + glyph_run.advance(),
                            y,
                        },
                    },
                    StrokePainter {
                        stroke: Stroke::new(width.into()),
                        brush: underline_brush.0.clone(),
                        transform: None,
                    },
                );
            }
            if let Some(strikethrough) = &style.strikethrough {
                let strikethrough_brush = &strikethrough.brush;
                let run_metrics = glyph_run.run().metrics();
                let offset = match strikethrough.offset {
                    Some(offset) => offset,
                    None => run_metrics.strikethrough_offset,
                };
                let width = match strikethrough.size {
                    Some(size) => size,
                    None => run_metrics.strikethrough_size,
                };
                // The `offset` is the distance from the baseline to the *top* of the strikethrough
                // so we move the line down by half the width
                // Remember that we are using a y-down coordinate system
                let y = glyph_run.baseline() - offset + width / 2.;

                paint_ctx.stroke_line(
                    Line {
                        p0: Point2d {
                            x: glyph_run.offset(),
                            y,
                        },
                        p1: Point2d {
                            x: glyph_run.offset() + glyph_run.advance(),
                            y,
                        },
                    },
                    StrokePainter {
                        stroke: Stroke::new(width.into()),
                        brush: strikethrough_brush.0.clone(),
                        transform: None,
                    },
                );
            }
        }
    }
}
