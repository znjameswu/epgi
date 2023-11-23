use std::{borrow::Cow, sync::Arc};

use epgi_core::foundation::SyncMutex;

pub struct Paragraph {
    text: Cow<'static, str>,
    brush: ParleyBrush,
    font_ctx: FontContext,
}

pub struct ParagraphLayout(pub(crate)parley::Layout<ParleyBrush>);

// For some reason, parley uses a RefCell in its FontContext
// We have no other choice but to go for a mutex
pub struct FontContext(Arc<SyncMutex<parley::FontContext>>);

#[derive(Clone, PartialEq, Debug)]
pub struct ParleyBrush(pub vello::peniko::Brush);

pub type TextAlign = parley::layout::Alignment;

impl Default for ParleyBrush {
    fn default() -> ParleyBrush {
        ParleyBrush(vello::peniko::Brush::Solid(vello::peniko::Color::rgb8(
            0, 0, 0,
        )))
    }
}

impl parley::style::Brush for ParleyBrush {}

impl Paragraph {
    pub fn _new(
        text: impl Into<Cow<'static, str>>,
        brush: ParleyBrush,
        font_ctx: FontContext,
    ) -> Self {
        Self {
            text: text.into(),
            brush,
            font_ctx,
        }
    }

    fn layout(&self, width: Option<f32>, alignment: TextAlign) -> ParagraphLayout {
        let mut layout_ctx = parley::LayoutContext::new();
        let mut font_ctx = self.font_ctx.0.lock();
        let mut layout_builder = layout_ctx.ranged_builder(&mut font_ctx, &self.text, 1.0);
        layout_builder.push_default(&parley::style::StyleProperty::Brush(self.brush.clone()));
        let mut layout = layout_builder.build();
        drop(font_ctx);
        layout.break_all_lines(width, alignment);
        ParagraphLayout(layout)
    }
}

// pub struct ParagraphBuilder {}

// impl ParagraphBuilder {
//     fn new() -> Self {
//         todo!()
//     }
//     fn add_text(&mut self, text: &str) {}

//     fn push_style(&mut self, style: TextStyle) {}

//     fn pop_style(&mut self) {}

//     fn build(self) -> Paragraph {
//         todo!()
//     }
// }

// pub struct TextStyle {}
