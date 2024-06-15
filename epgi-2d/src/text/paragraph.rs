use std::{borrow::Cow, ops::Range};

use epgi_core::foundation::{Arc, Asc, SyncMutex};

use crate::{BoxSize, LocalTextStyle, ParleyBrush, TextAlign};

pub struct ParagraphBuilder {
    text: Cow<'static, str>,
    current_styles: Vec<(parley::style::StyleProperty<'static, ParleyBrush>, usize)>,

    styles: Vec<(
        parley::style::StyleProperty<'static, ParleyBrush>,
        Range<usize>,
    )>,
}

impl ParagraphBuilder {
    pub fn new(text: impl Into<Cow<'static, str>>) -> Self {
        Self {
            text: text.into(),
            current_styles: Default::default(),
            styles: Default::default(),
        }
    }

    pub fn push(&mut self, text: impl Into<Cow<'static, str>>, style: Option<LocalTextStyle>) {
        self.text.to_mut().push_str(&text.into());
        if let Some(style) = style {}
    }

    pub fn build(self) -> Paragraph {
        todo!()
    }
}

pub struct Paragraph {
    inner: Asc<ParagraphInner>,
}

struct ParagraphInner {
    text: Cow<'static, str>,
    styles: Vec<(
        parley::style::StyleProperty<'static, ParleyBrush>,
        Range<usize>,
    )>,
    font_ctx: FontContext,
}

impl Paragraph {
    fn layout(&self, width: Option<f32>, alignment: TextAlign) -> ParagraphLayout {
        let mut layout_ctx = parley::LayoutContext::new();
        let mut font_ctx = self.inner.font_ctx.0.lock();
        let mut layout_builder = layout_ctx.ranged_builder(&mut font_ctx, &self.inner.text, 1.0);
        // layout_builder.push_default(&parley::style::StyleProperty::Brush(self.brush.clone()));
        // builder.push_default(&StyleProperty::FontSize(self.text_size));
        // builder.push_default(&StyleProperty::FontStack(self.font));
        // builder.push_default(&StyleProperty::FontWeight(self.weight));
        // builder.push_default(&StyleProperty::FontStyle(self.style));
        let mut layout = layout_builder.build();
        drop(font_ctx);
        layout.break_all_lines(width, alignment);
        ParagraphLayout(layout)
    }
}

pub struct ParagraphLayout(pub(crate) parley::Layout<ParleyBrush>);

impl ParagraphLayout {
    pub fn size(&self) -> BoxSize {
        BoxSize {
            width: self.0.width(),
            height: self.0.height(),
        }
    }
}

// For some reason, parley uses a RefCell in its FontContext
// We have no other choice but to go for a mutex
pub struct FontContext(Asc<SyncMutex<parley::FontContext>>);
