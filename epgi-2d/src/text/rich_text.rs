use std::sync::Arc;

use epgi_core::{
    foundation::{Asc, BuildSuspendedError, InlinableDwsizeVec, PaintContext, Provide},
    template::{ImplByTemplate, LeafElement, LeafElementTemplate, LeafRender, LeafRenderTemplate},
    tree::{BuildContext, ElementBase, RenderAction, Widget},
};

use crate::{
    Affine2dCanvas, Affine2dPaintContextExt, MultiLineConstraints, MultiLineOffset,
    MultiLineProtocol, MultiLineSize, Paragraph, TextAlign, TextSpan, TextStyle,
};

#[derive(Clone, Debug)]
pub struct RichText {
    /// Single item optimization
    pub text: TextSpan,
    pub text_spans: Vec<TextSpan>,
    pub style: TextStyle,
}

impl Widget for RichText {
    type ParentProtocol = MultiLineProtocol;

    type ChildProtocol = MultiLineProtocol;

    type Element = RichTextElement;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
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
        todo!()
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        todo!()
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
        paint_ctx.draw_paragraph(&self.paragraph, offset)
    }
}
