use std::{borrow::Cow, sync::Arc};

use epgi_core::foundation::{Asc, SyncMutex};

use crate::{BoxSize, Color};

pub struct TextSpan {
    pub text: Asc<str>,
    pub style: TextStyle,
}

pub struct TextStyle {
    pub background_color: Option<Color>,
    pub color: Option<Color>,
    pub debug_label: &'static str,
    pub decoration: Option<TextDecoration>,
    pub decoration_color: Option<Color>,
    pub decoration_style: Option<TextDecorationStyle>,
    pub decoration_thickness: Option<f32>,
    pub font_family: Option<&'static str>,
    pub font_family_fallback: Option<Vec<&'static str>>,
    pub font_features: Option<FontFeatures>,
    pub font_size: Option<f32>,
    pub font_style: Option<FontStyle>,
    pub font_variations: Option<Vec<FontVariation>>,
    pub height: Option<f32>,
    pub leading_distribution: Option<TextLeadingDistribution>,
    pub letter_spacing: Option<f32>,
    pub local: Option<Locale>,
    pub overflow: Option<TextOverFlow>,
    // pub shadows: Option<Vec<>>,
    pub text_baseline: Option<TextBaseline>,
    pub word_spacing: Option<f32>,
}

pub struct TextDecoration {}

pub enum TextDecorationStyle {
    Solid,
    Double,
    Dotted,
    Dashed,
    Wavy,
}

pub struct FontFeatures {}

pub struct FontStyle{}

pub struct Paragraph {
    text: Cow<'static, str>,
    brush: ParleyBrush,
    font_ctx: FontContext,
}

pub struct FontVariation {

}

pub enum TextLeadingDistribution {
    Proportional,
    Even
}

pub struct Locale {}

pub enum TextOverFlow {
    Clip,
    Fade,
    Ellipsis,
    Visible,
}

pub enum TextBaseline {
    Alphabetic,
    Ideographic,
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
