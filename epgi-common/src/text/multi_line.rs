use epgi_2d::{
    Affine2dCanvas, Affine2dMultiChildHitTest, Affine2dMultiChildLayout, Affine2dMultiChildPaint,
    Affine2dMultiChildRender, Affine2dMultiChildRenderTemplate, MultiLineConstraints,
    MultiLineOffset, MultiLineProtocol, MultiLineSize, SingleLineSize,
};
use epgi_core::{
    foundation::{Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Provide},
    template::{ImplByTemplate, MultiChildElement, MultiChildElementTemplate},
    tree::{ArcChildRenderObject, ArcChildWidget, BuildContext, ElementBase, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<MultiLine>))]
pub struct MultiLine {
    pub children: Vec<ArcChildWidget<MultiLineProtocol>>,
}

impl Widget for MultiLine {
    type ParentProtocol = MultiLineProtocol;
    type ChildProtocol = MultiLineProtocol;
    type Element = MultiLineElement;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

#[derive(Clone)]
pub struct MultiLineElement {}

impl ImplByTemplate for MultiLineElement {
    type Template = MultiChildElementTemplate<false>;
}

impl MultiChildElement for MultiLineElement {
    type ParentProtocol = MultiLineProtocol;
    type ChildProtocol = MultiLineProtocol;
    type ArcWidget = Asc<MultiLine>;
    type Render = RenderMultiLine;

    fn get_child_widgets(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<Vec<ArcChildWidget<MultiLineProtocol>>, BuildSuspendedError> {
        Ok(widget.children.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {}
    }

    fn create_render(&self, _widget: &Self::ArcWidget) -> Self::Render {
        RenderMultiLine {}
    }

    fn update_render(
        _render: &mut Self::Render,
        _widget: &Self::ArcWidget,
    ) -> Option<RenderAction> {
        None
    }
}

pub struct RenderMultiLine {}

impl ImplByTemplate for RenderMultiLine {
    type Template = Affine2dMultiChildRenderTemplate<false, false, false, false>;
}

impl Affine2dMultiChildRender for RenderMultiLine {
    type ParentProtocol = MultiLineProtocol;
    type ChildProtocol = MultiLineProtocol;
    type LayoutMemo = _RenderMultiLineLayoutMemo;
}

pub struct _RenderMultiLineLayoutMemo {
    results: Vec<MultiLineChildLayoutResult>,
}

struct MultiLineChildLayoutResult {
    first_line_advance: f32,
    line_count: u32,
}

impl Affine2dMultiChildLayout for RenderMultiLine {
    fn perform_layout(
        &mut self,
        constraints: &MultiLineConstraints,
        children: &Vec<ArcChildRenderObject<MultiLineProtocol>>,
    ) -> (MultiLineSize, Self::LayoutMemo) {
        let mut advance = constraints.first_line_existing_advance;
        let mut above = 0.0f32;
        let mut below = 0.0f32;
        let mut height = 0.0f32;
        let mut sizes = Vec::new();
        let mut memo = Vec::new();

        let mut it = children.iter().peekable();
        while let Some(child) = it.next() {
            let is_last = it.peek().is_none();

            let size = child.layout_use_size(&MultiLineConstraints {
                first_line_existing_advance: advance,
                max_width: constraints.max_width,
                last_line_append_advance: if is_last {
                    constraints.last_line_append_advance
                } else {
                    0.0
                },
                max_height: constraints.max_height - height,
            });

            // TODO: Prototype algorithm. We haven't consider about "Break at the start" case for child render objects.
            // To properly impl the algorithm, we need to impl multi-line protocol intrinsics.
            memo.push(MultiLineChildLayoutResult {
                first_line_advance: advance,
                line_count: size.sizes.len() as u32,
            });
            match size.sizes.as_slice() {
                [] => {}
                [size] => {
                    above = above.max(size.above);
                    below = below.max(size.below);
                    advance += size.advance;
                }
                [first_size, mid_sizes @ .., last_size] => {
                    // height
                    above = above.max(first_size.above);
                    below = below.max(first_size.below);
                    advance += first_size.advance;
                    sizes.push(SingleLineSize {
                        advance,
                        above,
                        below,
                    });
                    height += above + below;

                    sizes.extend(mid_sizes.iter().map(|size| {
                        height += size.above + size.below;
                        size
                    }));

                    above = last_size.above;
                    below = last_size.below;
                    advance = last_size.advance;
                }
            }
        }
        sizes.push(SingleLineSize {
            advance,
            above,
            below,
        });
        (
            MultiLineSize { sizes },
            _RenderMultiLineLayoutMemo { results: memo },
        )
    }
}

impl Affine2dMultiChildPaint for RenderMultiLine {
    fn perform_paint(
        &self,
        size: &MultiLineSize,
        offset: &MultiLineOffset,
        memo: &Self::LayoutMemo,
        children: &Vec<ArcChildRenderObject<MultiLineProtocol>>,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        debug_assert_eq!(size.sizes.len(), offset.offsets.len());
        debug_assert_eq!(children.len(), memo.results.len());
        let mut lines = std::iter::zip(size.sizes.iter(), offset.offsets.iter()).peekable();

        for (child, result) in std::iter::zip(children, memo.results.iter()) {
            let MultiLineChildLayoutResult {
                first_line_advance,
                line_count,
            } = *result;
            let line_count = line_count as usize;
            let child_offset = if line_count > 0 {
                let mut offsets = Vec::with_capacity(line_count);

                offsets.extend(
                    lines
                        .by_ref()
                        .take(line_count - 1) // `take` could return fewer items if there is not enough items, but we follow it with a `peek` and assert to make sure we get exactly the number of items we want
                        .map(|(_size, offset)| *offset),
                );
                offsets.push(
                    *lines
                        .peek()
                        .expect(
                            "Received line metrics should not be running out before all children has been painted",
                        )
                        .1,
                );
                debug_assert_eq!(offsets.len(), line_count, "Impossible to fail");
                offsets[0].advance = first_line_advance;
                MultiLineOffset { offsets }
            } else {
                MultiLineOffset {
                    offsets: Default::default(),
                }
            };

            paint_ctx.paint(child, &child_offset)
        }

        debug_assert_eq!(
            lines.len(),
            1,
            "Paint should have processed all incoming line metrics"
        ) // Remaining the last peeked line, ~~or there is no lines nor children to begin with~~
    }
}

impl Affine2dMultiChildHitTest for RenderMultiLine {}
