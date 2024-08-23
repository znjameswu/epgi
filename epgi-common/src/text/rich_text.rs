use std::sync::Arc;

use epgi_2d::{
    Affine2dCanvas, Affine2dPaintContextExt, MultiLineConstraints, MultiLineIntrinsics,
    MultiLineOffset, MultiLineProtocol, MultiLineSize, Paragraph, TextAlign, TextSpan, TextStyle,
};
use epgi_core::{
    foundation::{Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Provide},
    template::{ImplByTemplate, LeafElement, LeafElementTemplate, LeafRender, LeafRenderTemplate},
    tree::{BuildContext, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

#[derive(Clone, Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<RichText>))]
pub struct RichText {
    /// Single item optimization. If `text` is filled, then `text_spans` will be ignored
    #[builder(default, setter(strip_option))]
    pub text: Option<TextSpan>,
    /// If `text` is filled, then `text_spans` will be ignored
    #[builder(default)]
    pub text_spans: Vec<TextSpan>,
    pub style: TextStyle,
    #[builder(default = TextAlign::Start)]
    pub text_align: TextAlign,
}

impl Widget for RichText {
    type ParentProtocol = MultiLineProtocol;

    type ChildProtocol = MultiLineProtocol;

    type Element = RichTextElement;

    fn into_arc_widget(self: Asc<Self>) -> Asc<Self> {
        self
    }
}

#[derive(Clone, Debug)]
pub struct RichTextElement {}

impl ImplByTemplate for RichTextElement {
    type Template = LeafElementTemplate;
}

impl LeafElement for RichTextElement {
    type Render = RenderRichText;

    type Protocol = MultiLineProtocol;

    type ArcWidget = Asc<RichText>;

    fn create_element(
        _widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<Self, BuildSuspendedError> {
        Ok(Self {})
    }

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        let spans = widget
            .text
            .as_ref()
            .map(std::slice::from_ref)
            .unwrap_or(widget.text_spans.as_slice());
        RenderRichText {
            paragraph: Paragraph::new(spans, &widget.style),
            text_align: widget.text_align,
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        let spans = widget
            .text
            .as_ref()
            .map(std::slice::from_ref)
            .unwrap_or(widget.text_spans.as_slice());
        render.paragraph = Paragraph::new(spans, &widget.style);
        render.text_align = widget.text_align;
        Some(RenderAction::Relayout)
    }
}

pub struct RenderRichText {
    paragraph: Paragraph,
    text_align: TextAlign,
}

impl ImplByTemplate for RenderRichText {
    type Template = LeafRenderTemplate;
}

impl LeafRender for RenderRichText {
    type Protocol = MultiLineProtocol;

    fn perform_layout(&mut self, constraints: &MultiLineConstraints) -> MultiLineSize {
        let sizes = self
            .paragraph
            .layout_multi_line(constraints, self.text_align);
        MultiLineSize { sizes }
    }

    fn perform_paint(
        &self,
        _size: &MultiLineSize,
        offset: &MultiLineOffset,
        paint_ctx: &mut impl PaintContext<Canvas = Affine2dCanvas>,
    ) {
        paint_ctx.draw_paragraph(&self.paragraph, &offset.offsets)
    }

    fn compute_intrinsics(render: &mut Self, intrinsics: &mut MultiLineIntrinsics) {
        unimplemented!()
    }
}
