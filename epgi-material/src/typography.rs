use epgi_2d::{
    Color, FontFamily, FontWeight, TextBaseline, TextDecoration, TextLeadingDistribution, TextStyle,
};

const HELSINKI_FONT_FALLBACKS: &[FontFamily] = &[
    FontFamily::Named("Ubuntu'"),
    FontFamily::Named("Cantarell'"),
    FontFamily::Named("DejaVu Sans"),
    FontFamily::Named("Liberation Sans"),
    FontFamily::Named("Aria"),
];

pub const BLACK_87: Color = Color::rgba8(0, 0, 0, 0xDD);
pub const BLACK_54: Color = Color::rgba8(0, 0, 0, 0x8A);

pub fn black_mountain_view_body_medium() -> TextStyle {
    TextStyle {
        background_color: None,
        color: BLACK_87,
        debug_label: "Black Helsinki",
        decoration: TextDecoration::empty(),
        decoration_color: Color::BLACK, // Flutter's Typography constructor has an apply function
        decoration_thickness: 1.0,      // Flutter's TextStyle default
        font_family: FontFamily::Named("Roboto"),
        font_family_fallback: HELSINKI_FONT_FALLBACKS.to_vec(),
        font_features: Default::default(),
        font_size: 14.0,
        font_style: Default::default(),
        font_variations: Default::default(),
        font_weight: FontWeight::NORMAL,
        height: 1.43,
        leading_distribution: TextLeadingDistribution::Even,
        letter_spacing: 0.25,
        locale: Default::default(),
        overflow: Default::default(),
        text_baseline: TextBaseline::Alphabetic,
        word_spacing: Default::default(),
    }
}
