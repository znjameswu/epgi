use epgi_2d::BoxProtocol;
use epgi_core::{
    foundation::{Asc, Protocol},
    tree::ArcChildWidget,
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use super::{Align, Alignment};

pub type CenteredBox = Center<BoxProtocol>;

pub type CenteredBoxBuilder = CenterBuilder<BoxProtocol>;

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Align<P>>))]
pub struct Center<P: Protocol> {
    #[builder(default)]
    pub width_factor: Option<f32>,
    #[builder(default)]
    pub height_factor: Option<f32>,
    pub child: ArcChildWidget<P>,
}

impl<P: Protocol> Into<Asc<Align<P>>> for Center<P> {
    fn into(self) -> Asc<Align<P>> {
        Align!(
            alignment = Alignment::CENTER,
            width_factor = self.width_factor,
            height_factor = self.height_factor,
            child = self.child
        )
    }
}
