use std::{borrow::Cow, default, ops::Range};

use epgi_core::foundation::{Arc, Asc, SyncMutex};
use parley::style::{FontStack, StyleProperty};

use crate::{BoxSize, LocalTextStyle, ParleyBrush, TextAlign, TextStyle};

pub struct TextSpan {
    text: Cow<'static, str>,
    style: Option<LocalTextStyle>,
}

// pub struct ParagraphBuilder {
//     pub spans: Vec<TextSpan>,
//     pub default_style: TextStyle,
//     // text: Cow<'a, str>,

//     // current_styles: Vec<(StyleProperty<'a, ParleyBrush>, usize)>,
//     // default_styles: Vec<StyleProperty<'a, ParleyBrush>>,
//     // styles: Vec<(StyleProperty<'a, ParleyBrush>, Range<usize>)>,
// }

// impl<'a> ParagraphBuilder<'a> {
//     pub fn new_empty(default_style: TextStyle) -> Self {
//         Self {
//             spans: Vec::new(),
//             default_style,
//         }
//     }
//     pub fn new(text: impl Into<Cow<'a, str>>, default_style: TextStyle) -> Self {
//         // let font_stack = if default_style.font_family_fallback.is_empty() {
//         //     FontStack::Single(default_style.font_family.clone())
//         // } else {
//         //     FontStack::List()
//         // };
//         // let mut default_styles = vec![
//         //     StyleProperty::FontStack(FontStack::),
//         //     StyleProperty::FontSize(default_style.font_size),
//         //     StyleProperty::Locale(default_style.locale),
//         //     StyleProperty::LetterSpacing(default_style.letter_spacing),
//         //     StyleProperty::WordSpacing(default_style.word_spacing),
//         //     StyleProperty::Brush(ParleyBrush(vello::peniko::Brush::Solid(
//         //         default_style.color,
//         //     ))),
//         //     StyleProperty::FontStyle(default_style.font_style),
//         //     StyleProperty::FontWeight(default_style.font_weight),
//         //     StyleProperty::LineHeight(default_style.height),
//         // ];
//         // Self {
//         //     text: text.into(),
//         //     current_styles: Default::default(),
//         //     default_styles,
//         //     styles: Default::default(),
//         // }
//     }

//     pub fn push(&mut self, text: impl Into<Cow<'static, str>>, style: Option<LocalTextStyle>) {
//         self.text.to_mut().push_str(&text.into());
//         if let Some(style) = style {}
//     }

//     pub fn build(self) -> Paragraph {
//         todo!()
//     }
// }

pub struct Paragraph {
    pub(crate) layout: parley::Layout<ParleyBrush>,
}

impl Paragraph {
    pub fn new(spans: &[TextSpan], default_style: TextStyle) -> Self {
        let mut layout_ctx = parley::LayoutContext::new();
        let mut font_ctx = GLOBAL_FONT_CONTEXT.lock();

        let text = match spans {
            [] => Cow::Borrowed(""),
            [span] => Cow::Borrowed(span.text.as_ref()),
            spans => {
                let str_len = spans.iter().map(|span| span.text.len()).sum();
                let mut text = String::with_capacity(str_len);
                spans.iter().for_each(|span| text.push_str(&span.text));
                Cow::Owned(text)
            }
        };
        let mut layout_builder = layout_ctx.ranged_builder(&mut font_ctx, &text, 1.0);
        // for default_style in self.inner.default_styles.iter() {
        //     layout_builder.push_default(default_style)
        // }
        // for (style, range) in self.inner.styles.iter() {
        //     layout_builder.push(style, range.clone())
        // }
        let mut layout = layout_builder.build();
        drop(font_ctx);
        // layout.break_all_lines(width, alignment);
        Self { layout }
    }
    pub fn layout(&mut self, width: Option<f32>, alignment: TextAlign) {
        self.layout.break_all_lines(width, alignment);
    }
}

lazy_static::lazy_static! {
    // For some reason, parley uses a RefCell in its FontContext
    // We have no other choice but to go for a mutex
    static ref GLOBAL_FONT_CONTEXT: Asc<SyncMutex<parley::FontContext>> = Asc::new(SyncMutex::new(parley::FontContext::default()));
}
