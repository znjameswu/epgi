use epgi_2d::ArcBoxWidget;
use epgi_core::foundation::Asc;
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use super::{Align, Alignment};

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Align>))]
pub struct Center {
    #[builder(default)]
    pub width_factor: Option<f32>,
    #[builder(default)]
    pub height_factor: Option<f32>,
    pub child: ArcBoxWidget,
}

impl Into<Asc<Align>> for Center {
    fn into(self) -> Asc<Align> {
        Align!(
            alignment = Alignment::CENTER,
            width_factor = self.width_factor,
            height_factor = self.height_factor,
            child = self.child
        )
    }
}