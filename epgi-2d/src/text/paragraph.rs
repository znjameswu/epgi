use std::{borrow::Cow, ops::Range};

use crate::{LocalTextStyle, Paragraph, ParleyBrush};

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
        if let Some(style) = style {

        }
    }

    pub fn build(self) -> Paragraph {
        todo!()
    }
}
