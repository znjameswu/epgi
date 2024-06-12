use epgi_2d::Color;

use crate::{ColorScheme, ProgressIndicatorThemeData};

#[derive(PartialEq, Clone, Debug)]
pub struct ThemeData {
    pub color_scheme: ColorScheme,
    pub progress_indicator_theme: ProgressIndicatorThemeData,
}

impl ThemeData {
    pub fn light() -> Self {
        ThemeData {
            color_scheme: ColorScheme::from_swatch(Color::BLUE),
            progress_indicator_theme: Default::default(),
        }
    }
}
