mod paragraph;
pub use paragraph::*;

use std::{borrow::Cow, sync::Arc};

use epgi_core::foundation::{Asc, SyncMutex};

use crate::{BoxSize, Color};

pub struct TextSpan {
    pub text: Asc<str>,
    pub style: LocalTextStyle,
}

pub struct LocalTextStyle {
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

pub struct FontStyle {}

pub struct FontVariation {}

pub enum TextLeadingDistribution {
    Proportional,
    Even,
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
