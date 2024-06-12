use epgi_2d::Color;

#[derive(PartialEq, Clone, Debug)]
pub struct ColorScheme {
    pub primary: Color,
}

impl ColorScheme {
    pub const fn from_swatch(primary_swatch: Color) -> Self {
        Self {
            primary: primary_swatch,
        }
    }
}
