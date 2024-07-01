use epgi_2d::Color;

use crate::{ColorScheme, ProgressIndicatorThemeData, TextTheme, Typography};

#[derive(PartialEq, Clone, Debug)]
pub struct ThemeData {
    pub scaffold_background_color: Color,
    pub color_scheme: ColorScheme,
    pub text_theme: TextTheme,
    pub progress_indicator_theme: ProgressIndicatorThemeData,
}

impl ThemeData {
    pub fn light() -> Self {
        let color_scheme = ColorScheme::from_swatch(Color::BLUE);
        let typography = Typography::material_2021(None);
        let default_text_theme = typography.black;
        let text_theme = default_text_theme;
        ThemeData {
            scaffold_background_color: Color::rgba8(0xFA, 0xFA, 0xFA, 0xFF),
            color_scheme,
            text_theme,
            progress_indicator_theme: Default::default(),
        }
    }
}
