use crate::Color;

#[derive(Clone, Debug)]
pub struct TextStyle {
    pub background_color: Option<Color>,
    pub color: Color,
    pub debug_label: &'static str,
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

#[derive(Clone, Debug)]
pub struct LocalTextStyle {
    pub background_color: Option<Option<Color>>,
    pub color: Option<Color>,
    pub debug_label: &'static str,
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
    pub locale: Option<&'static str>,
    pub overflow: Option<TextOverFlow>,
    // pub shadows: Option<Vec<>>,
    pub text_baseline: Option<TextBaseline>,
    pub word_spacing: Option<f32>,
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
