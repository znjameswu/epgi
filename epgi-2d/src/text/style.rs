use std::borrow::Cow;

use crate::Color;

#[derive(PartialEq, Clone, Debug)]
pub struct TextStyle {
    pub background_color: Option<Color>,
    pub color: Color,
    pub debug_label: Option<Cow<'static, str>>,
    pub decoration: TextDecoration,
    pub decoration_color: Color,
    // pub decoration_style: TextDecorationStyle,
    pub decoration_thickness: f32,
    pub font_family: FontFamily,
    pub font_family_fallback: Vec<FontFamily>,
    pub font_features: FontFeatures,
    pub font_size: f32,
    pub font_style: FontStyle,
    pub font_variations: Vec<FontVariation>,
    pub font_weight: FontWeight,
    pub height: f32,
    pub leading_distribution: TextLeadingDistribution,
    pub letter_spacing: f32,
    pub locale: Option<&'static str>,
    pub overflow: TextOverFlow,
    // pub shadows: Vec<>,
    pub text_baseline: TextBaseline,
    pub word_spacing: f32,
}

#[derive(PartialEq, Clone, Debug, Default)]
pub struct LocalTextStyle {
    pub background_color: Option<Option<Color>>,
    pub color: Option<Color>,
    pub debug_label: Option<Cow<'static, str>>,
    pub decoration: Option<TextDecoration>,
    pub decoration_color: Option<Color>,
    pub decoration_style: Option<TextDecorationStyle>,
    pub decoration_thickness: Option<f32>,
    pub font_family: Option<FontFamily>,
    pub font_family_fallback: Option<Vec<FontFamily>>,
    pub font_features: Option<FontFeatures>,
    pub font_size: Option<f32>,
    pub font_style: Option<FontStyle>,
    pub font_variations: Option<Vec<FontVariation>>,
    pub font_weight: Option<FontWeight>,
    pub height: Option<f32>,
    pub leading_distribution: Option<TextLeadingDistribution>,
    pub letter_spacing: Option<f32>,
    pub locale: Option<Option<&'static str>>,
    pub overflow: Option<TextOverFlow>,
    // pub shadows: Option<Vec<>>,
    pub text_baseline: Option<TextBaseline>,
    pub word_spacing: Option<f32>,
}

impl TextStyle {
    pub fn merge(&self, style: LocalTextStyle) -> Self {
        let debug_label = match (self.debug_label.as_ref(), style.debug_label) {
            (None, None) => None,
            (None, b @ Some(_)) => b,
            (a @ Some(_), None) => a.cloned(),
            (Some(a), Some(b)) => Some(Cow::Owned(format!("{a} + {b}"))),
        };
        Self {
            background_color: style.background_color.unwrap_or(self.background_color),
            color: style.color.unwrap_or(self.color),
            debug_label,
            decoration: style.decoration.unwrap_or(self.decoration),
            decoration_color: style.decoration_color.unwrap_or(self.decoration_color),
            decoration_thickness: style
                .decoration_thickness
                .unwrap_or(self.decoration_thickness),
            font_family: style.font_family.unwrap_or(self.font_family),
            font_family_fallback: style
                .font_family_fallback
                .unwrap_or(self.font_family_fallback.clone()),
            font_features: style.font_features.unwrap_or(self.font_features),
            font_size: style.font_size.unwrap_or(self.font_size),
            font_style: style.font_style.unwrap_or(self.font_style),
            font_variations: style
                .font_variations
                .unwrap_or(self.font_variations.clone()),
            font_weight: style.font_weight.unwrap_or(self.font_weight),
            height: style.height.unwrap_or(self.height),
            leading_distribution: style
                .leading_distribution
                .unwrap_or(self.leading_distribution),
            letter_spacing: style.letter_spacing.unwrap_or(self.letter_spacing),
            locale: style.locale.unwrap_or(self.locale),
            overflow: style.overflow.unwrap_or(self.overflow),
            text_baseline: style.text_baseline.unwrap_or(self.text_baseline),
            word_spacing: style.word_spacing.unwrap_or(self.word_spacing),
        }
    }
}

bitflags::bitflags! {
    #[derive(PartialEq, Eq, Clone, Copy, Debug)]
    pub struct TextDecoration: u8 {
        const LINE_THROUGH = 1;
        const OVERLINE = 0b10;
        const UNDERLINE = 0b100;
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TextDecorationStyle {
    Solid,
    Double,
    Dotted,
    Dashed,
    Wavy,
}

pub type FontFamily = parley::style::FontFamily<'static>;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct FontFeatures {}

impl Default for FontFeatures {
    fn default() -> Self {
        Self {}
    }
}

pub type FontStyle = parley::style::FontStyle;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct FontVariation {}

pub type FontWeight = parley::style::FontWeight;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TextLeadingDistribution {
    Proportional,
    Even,
}

// #[derive(PartialEq, Eq, Clone, Copy, Debug)]
// pub struct Locale {}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TextOverFlow {
    Clip,
    Fade,
    Ellipsis,
    Visible,
}

impl Default for TextOverFlow {
    fn default() -> Self {
        TextOverFlow::Clip
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
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
