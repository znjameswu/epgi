use std::{borrow::Cow, ops::Range};

use epgi_core::foundation::{Asc, SyncMutex};
use parley::style::{FontStack, StyleProperty};

use crate::{
    LocalTextStyle, MultiLineConstraints, ParleyBrush, SingleLineSize, TextAlign, TextDecoration,
    TextStyle,
};

#[derive(Clone, Debug)]
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
    pub fn new(spans: &[TextSpan], default_style: &TextStyle) -> Self {
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

        layout_builder.push_default(&StyleProperty::Brush(ParleyBrush(peniko::Brush::Solid(
            default_style.color,
        ))));
        if default_style.font_family_fallback.is_empty() {
            layout_builder.push_default(&StyleProperty::FontStack(FontStack::Single(
                default_style.font_family.clone(),
            )));
        } else {
            let font_stack = std::iter::once(default_style.font_family)
                .chain(default_style.font_family_fallback.iter().cloned())
                .collect::<Vec<_>>();
            layout_builder.push_default(&StyleProperty::FontStack(FontStack::List(&font_stack)));
        };
        layout_builder.push_default(&StyleProperty::FontSize(default_style.font_size));
        layout_builder.push_default(&StyleProperty::FontStyle(default_style.font_style));
        layout_builder.push_default(&StyleProperty::FontWeight(default_style.font_weight));
        layout_builder.push_default(&StyleProperty::LineHeight(default_style.height));
        layout_builder.push_default(&StyleProperty::LetterSpacing(default_style.letter_spacing));
        layout_builder.push_default(&StyleProperty::Locale(default_style.locale));
        layout_builder.push_default(&StyleProperty::WordSpacing(default_style.word_spacing));
        if default_style
            .decoration
            .contains(TextDecoration::LINE_THROUGH)
        {
            layout_builder.push_default(&StyleProperty::Strikethrough(true));
        }
        if default_style.decoration.contains(TextDecoration::UNDERLINE) {
            layout_builder.push_default(&StyleProperty::Underline(true));
        }
        if default_style.decoration.contains(TextDecoration::OVERLINE) {
            unimplemented!("Parley does not support overline")
        }
        layout_builder.push_default(&StyleProperty::StrikethroughBrush(Some(ParleyBrush(
            peniko::Brush::Solid(default_style.decoration_color),
        ))));
        layout_builder.push_default(&StyleProperty::UnderlineBrush(Some(ParleyBrush(
            peniko::Brush::Solid(default_style.decoration_color),
        ))));
        layout_builder.push_default(&StyleProperty::StrikethroughSize(Some(
            default_style.decoration_thickness,
        )));
        layout_builder.push_default(&StyleProperty::UnderlineSize(Some(
            default_style.decoration_thickness,
        )));

        let mut position = 0;
        for span in spans {
            let len = span.text.len();
            if let Some(style) = &span.style {
                let range = Range {
                    start: position,
                    end: position + len,
                };
                if let Some(color) = style.color {
                    layout_builder.push(
                        &StyleProperty::Brush(ParleyBrush(peniko::Brush::Solid(color))),
                        range.clone(),
                    );
                }
                if style.font_family.is_some() || style.font_family_fallback.is_some() {
                    let font_family = style.font_family.unwrap_or(default_style.font_family);
                    let font_family_fallback = style
                        .font_family_fallback
                        .as_ref()
                        .unwrap_or(&default_style.font_family_fallback);
                    if font_family_fallback.is_empty() {
                        layout_builder.push(
                            &StyleProperty::FontStack(FontStack::Single(font_family)),
                            range.clone(),
                        );
                    } else {
                        let font_stack = std::iter::once(default_style.font_family)
                            .chain(font_family_fallback.iter().cloned())
                            .collect::<Vec<_>>();
                        layout_builder.push(
                            &StyleProperty::FontStack(FontStack::List(&font_stack)),
                            range.clone(),
                        );
                    };
                }
                if let Some(font_size) = style.font_size {
                    layout_builder.push(&StyleProperty::FontSize(font_size), range.clone());
                }
                if let Some(font_style) = style.font_style {
                    layout_builder.push(&StyleProperty::FontStyle(font_style), range.clone());
                }
                if let Some(font_weight) = style.font_weight {
                    layout_builder.push(&StyleProperty::FontWeight(font_weight), range.clone());
                }
                if let Some(height) = style.height {
                    layout_builder.push(&StyleProperty::LineHeight(height), range.clone());
                }
                if let Some(letter_spacing) = style.letter_spacing {
                    layout_builder
                        .push(&StyleProperty::LetterSpacing(letter_spacing), range.clone());
                }
                if let Some(locale) = style.locale {
                    layout_builder.push(&StyleProperty::Locale(Some(locale)), range.clone());
                }
                if let Some(word_spacing) = style.word_spacing {
                    layout_builder.push(&StyleProperty::WordSpacing(word_spacing), range.clone());
                }
                if let Some(decoration) = style.decoration {
                    if decoration.contains(TextDecoration::LINE_THROUGH) {
                        layout_builder.push(&StyleProperty::Strikethrough(true), range.clone());
                    }
                    if decoration.contains(TextDecoration::UNDERLINE) {
                        layout_builder.push(&StyleProperty::Underline(true), range.clone());
                    }
                    if decoration.contains(TextDecoration::OVERLINE) {
                        unimplemented!("Parley does not support overline")
                    }
                }
                if let Some(decoration_color) = style.decoration_color {
                    layout_builder.push(
                        &StyleProperty::StrikethroughBrush(Some(ParleyBrush(
                            peniko::Brush::Solid(decoration_color),
                        ))),
                        range.clone(),
                    );
                    layout_builder.push(
                        &StyleProperty::UnderlineBrush(Some(ParleyBrush(peniko::Brush::Solid(
                            decoration_color,
                        )))),
                        range.clone(),
                    );
                }
                if let Some(decoration_thickness) = style.decoration_thickness {
                    layout_builder.push(
                        &StyleProperty::StrikethroughSize(Some(decoration_thickness)),
                        range.clone(),
                    );
                    layout_builder.push(
                        &StyleProperty::UnderlineSize(Some(decoration_thickness)),
                        range.clone(),
                    );
                }
            }
            position += len;
        }
        let layout = layout_builder.build();
        drop(font_ctx);
        // layout.break_all_lines(width, alignment);
        Self { layout }
    }

    pub fn layout(&mut self, width: Option<f32>, alignment: TextAlign) -> Vec<SingleLineSize> {
        self.layout.break_all_lines(width, alignment);
        self.layout
            .lines()
            .map(|line| {
                let metrics = line.metrics();
                SingleLineSize {
                    advance: metrics.advance,
                    above: metrics.ascent + metrics.leading * 0.5,
                    below: metrics.descent + metrics.leading * 0.5,
                }
            })
            .collect()
    }

    pub fn layout_single_line(&mut self) -> Vec<SingleLineSize> {
        self.layout.break_all_lines(None, TextAlign::Start);
        self.layout
            .lines()
            .map(|line| {
                let metrics = line.metrics();
                SingleLineSize {
                    advance: metrics.advance,
                    above: metrics.ascent + metrics.leading * 0.5,
                    below: metrics.descent + metrics.leading * 0.5,
                }
            })
            .collect()
    }

    pub fn layout_multi_line(
        &mut self,
        constraints: &MultiLineConstraints,
        alignment: TextAlign,
    ) -> Vec<SingleLineSize> {
        let mut break_lines = self.layout.break_lines();
        if constraints.first_line_existing_advance != 0.0 {
            break_lines.break_next(
                constraints.max_width - constraints.first_line_existing_advance,
                TextAlign::Start,
            );
        }
        while break_lines
            .break_next(constraints.max_width, alignment)
            .is_some()
        {}
        break_lines.finish();
        if constraints.last_line_append_advance != 0.0 {
            let last_line_advance = self.layout.lines().last().unwrap().metrics().advance;
            if last_line_advance >= constraints.max_width - constraints.last_line_append_advance
                && last_line_advance <= constraints.max_width
            {
                tracing::error!(
                    "Parley does not yet have the API to support an unbreakable line end"
                )
            }
        }
        self.layout
            .lines()
            .map(|line| {
                let metrics = line.metrics();
                SingleLineSize {
                    advance: metrics.advance,
                    above: metrics.ascent + metrics.leading * 0.5,
                    below: metrics.descent + metrics.leading * 0.5,
                }
            })
            .collect()
    }
}

lazy_static::lazy_static! {
    // For some reason, parley uses a RefCell in its FontContext
    // We have no other choice but to go for a mutex
    static ref GLOBAL_FONT_CONTEXT: Asc<SyncMutex<parley::FontContext>> = Asc::new(SyncMutex::new(parley::FontContext::default()));
}
