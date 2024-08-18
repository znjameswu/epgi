use epgi_2d::BoxProtocol;
use epgi_common::{CrossAxisAlignment, Flexible, MainAxisAlignment, MainAxisSize};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;



#[derive(Debug, Declarative, TypedBuilder)]
pub struct RingSlice {
    /// How the children should be placed along the main axis.
    #[builder(default = MainAxisAlignment::Start)]
    pub main_axis_alignment: MainAxisAlignment,
    /// How much space should be occupied in the main axis.
    ///
    /// After allocating space to children, there might be some remaining free
    /// space. This value controls whether to maximize or minimize the amount of
    /// free space, subject to the incoming layout constraints.
    ///
    /// If some children have a non-zero flex factors (and none have a fit of
    /// [FlexFit::Loose]), they will expand to consume all the available space and
    /// there will be no remaining free space to maximize or minimize, making this
    /// value irrelevant to the final layout.
    #[builder(default = MainAxisSize::Max)]
    pub main_axis_size: MainAxisSize,
    /// How the children should be placed along the cross axis.
    #[builder(default = CrossAxisAlignment::Center)]
    pub cross_axis_alignment: CrossAxisAlignment,
    #[builder(default = false)]
    pub flip_horizontal: bool,
    #[builder(default = false)]
    pub flip_vertical: bool,
    pub children: Vec<Flexible<BoxProtocol>>,
}