use std::marker::PhantomData;

use epgi_common::{Axis, RenderFlex};
use epgi_core::{
    foundation::{set_if_changed, Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Provide},
    template::{
        ImplByTemplate, MultiChildElement, MultiChildElementTemplate, MultiChildRenderElement,
    },
    tree::{BuildContext, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use super::{
    get_flexible_configs, ArcRingWidget, CrossAxisAlignment, Flexible, MainAxisAlignment,
    MainAxisSize, RingProtocol,
};

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<RingSlice>))]
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
    pub flip_angular: bool,
    #[builder(default = false)]
    pub flip_radial: bool,
    pub children: Vec<Flexible<RingProtocol>>,
}

impl Widget for RingSlice {
    type ParentProtocol = RingProtocol;
    type ChildProtocol = RingProtocol;
    type Element = RingSliceElement;

    fn into_arc_widget(self: Asc<Self>) -> Asc<Self> {
        self
    }
}

#[derive(Clone, Debug)]
pub struct RingSliceElement {}

impl ImplByTemplate for RingSliceElement {
    type Template = MultiChildElementTemplate<true, false>;
}

impl MultiChildElement for RingSliceElement {
    type ParentProtocol = RingProtocol;
    type ChildProtocol = RingProtocol;
    type ArcWidget = Asc<RingSlice>;

    fn get_child_widgets(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<Vec<ArcRingWidget>, BuildSuspendedError> {
        Ok(widget
            .children
            .iter()
            .map(|flexible| flexible.child.clone())
            .collect())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {}
    }
}

impl MultiChildRenderElement for RingSliceElement {
    type Render = RenderFlex<RingProtocol>;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderFlex {
            direction: Axis::Vertical,
            main_axis_alignment: widget.main_axis_alignment,
            main_axis_size: widget.main_axis_size,
            cross_axis_alignment: widget.cross_axis_alignment,
            flexible_configs: get_flexible_configs(&widget.children),
            flip_main_axis: widget.flip_radial,
            flip_cross_axis: widget.flip_angular,
            phantom: PhantomData,
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        [
            // set_if_changed(&mut render.direction, widget.direction.into()),
            set_if_changed(&mut render.main_axis_alignment, widget.main_axis_alignment),
            set_if_changed(&mut render.main_axis_size, widget.main_axis_size),
            set_if_changed(
                &mut render.cross_axis_alignment,
                widget.cross_axis_alignment,
            ),
            set_if_changed(
                &mut render.flexible_configs,
                get_flexible_configs(&widget.children),
            ),
            set_if_changed(&mut render.flip_main_axis, widget.flip_radial),
            set_if_changed(&mut render.flip_cross_axis, widget.flip_angular),
        ]
        .iter()
        .any(|&changed| changed)
        .then_some(RenderAction::Relayout)
    }
}
