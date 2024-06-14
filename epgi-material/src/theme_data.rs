use epgi_2d::Color;

use crate::{ColorScheme, ProgressIndicatorThemeData};

#[derive(PartialEq, Clone, Debug)]
pub struct ThemeData {
    pub scaffold_background_color: Color,
    pub color_scheme: ColorScheme,
    pub progress_indicator_theme: ProgressIndicatorThemeData,
}

impl ThemeData {
    pub fn light() -> Self {
        let color_scheme = ColorScheme::from_swatch(Color::BLUE);
        ThemeData {
            scaffold_background_color: Color::rgba8(0xFA, 0xFA, 0xFA, 0xFF),
            color_scheme,
            progress_indicator_theme: Default::default(),
        }
    }
}
