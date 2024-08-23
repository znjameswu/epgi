use epgi_core::foundation::Asc;
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use super::{AlignedRing, ArcRingWidget, RingAlignment};

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<AlignedRing>))]
pub struct CenteredRing {
    #[builder(default)]
    pub angular_factor: Option<f32>,
    #[builder(default)]
    pub radial_factor: Option<f32>,
    pub child: ArcRingWidget,
}

impl Into<Asc<AlignedRing>> for CenteredRing {
    fn into(self) -> Asc<AlignedRing> {
        AlignedRing!(
            alignment = RingAlignment::CENTER,
            angular_factor = self.angular_factor,
            radial_factor = self.radial_factor,
            child = self.child
        )
    }
}
