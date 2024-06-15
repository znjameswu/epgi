use std::{borrow::Cow, default, ops::Range};

use epgi_core::foundation::{Arc, Asc, SyncMutex};
use parley::style::StyleProperty;

use crate::{BoxSize, LocalTextStyle, ParleyBrush, TextAlign, TextStyle};

pub struct ParagraphBuilder {
    text: Cow<'static, str>,
    current_styles: Vec<(StyleProperty<'static, ParleyBrush>, usize)>,
    default_styles: Vec<StyleProperty<'static, ParleyBrush>>,
    styles: Vec<(StyleProperty<'static, ParleyBrush>, Range<usize>)>,
}

impl ParagraphBuilder {
    pub fn new(text: impl Into<Cow<'static, str>>, default_style: &TextStyle) -> Self {
        let mut default_styles = Vec::new();

        StyleProperty::Locale(default_style.locale);
        StyleProperty::LetterSpacing(default_style.letter_spacing);
        StyleProperty::WordSpacing(default_style.word_spacing);
        vec![
            StyleProperty::Brush(ParleyBrush(vello::peniko::Brush::Solid(
                default_style.color,
            ))),
            StyleProperty::FontSize(default_style.font_size),
            StyleProperty::FontStyle(default_style.font_style),
            StyleProperty::FontWeight(default_style.font_weight),
            StyleProperty::LineHeight(default_style.height),
        ];
        Self {
            text: text.into(),
            current_styles: Default::default(),
            default_styles,
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
    styles: Vec<(StyleProperty<'static, ParleyBrush>, Range<usize>)>,
    default_styles: Vec<StyleProperty<'static, ParleyBrush>>,
}

impl Paragraph {
    pub fn layout(&self, width: Option<f32>, alignment: TextAlign) -> ParagraphLayout {
        let mut layout_ctx = parley::LayoutContext::new();
        let mut font_ctx = GLOBAL_FONT_CONTEXT.lock();
        let mut layout_builder = layout_ctx.ranged_builder(&mut font_ctx, &self.inner.text, 1.0);
        for default_style in self.inner.default_styles.iter() {
            layout_builder.push_default(default_style)
        }
        for (style, range) in self.inner.styles.iter() {
            layout_builder.push(style, range.clone())
        }
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

lazy_static::lazy_static! {
    // For some reason, parley uses a RefCell in its FontContext
    // We have no other choice but to go for a mutex
    static ref GLOBAL_FONT_CONTEXT: Asc<SyncMutex<parley::FontContext>> = Asc::new(SyncMutex::new(parley::FontContext::default()));
}
