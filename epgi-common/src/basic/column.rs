use std::marker::PhantomData;

use epgi_2d::{ArcBoxWidget, BoxProtocol};
use epgi_core::{
    foundation::{set_if_changed, Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Provide},
    template::{ImplByTemplate, MultiChildElement, MultiChildElementTemplate},
    tree::{BuildContext, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::{Axis, CrossAxisAlignment, MainAxisAlignment, MainAxisSize, RenderFlex};

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Column>))]
pub struct Column {
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
    pub children: Vec<ArcBoxWidget>,
}

impl Widget for Column {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type Element = ColumnElement;

    fn into_arc_widget(self: Asc<Self>) -> Asc<Self> {
        self
    }
}

#[derive(Clone)]
pub struct ColumnElement {}

impl ImplByTemplate for ColumnElement {
    type Template = MultiChildElementTemplate<false>;
}

impl MultiChildElement for ColumnElement {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type ArcWidget = Asc<Column>;
    type Render = RenderFlex<BoxProtocol>;

    fn get_child_widgets(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<Vec<ArcBoxWidget>, BuildSuspendedError> {
        Ok(widget.children.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {}
    }

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderFlex {
            direction: Axis::Vertical,
            main_axis_alignment: widget.main_axis_alignment,
            main_axis_size: widget.main_axis_size,
            cross_axis_alignment: widget.cross_axis_alignment,
            flip_main_axis: widget.flip_vertical,
            flip_cross_axis: widget.flip_horizontal,
            phantom: PhantomData,
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        [
            set_if_changed(&mut render.main_axis_alignment, widget.main_axis_alignment),
            set_if_changed(&mut render.main_axis_size, widget.main_axis_size),
            set_if_changed(
                &mut render.cross_axis_alignment,
                widget.cross_axis_alignment,
            ),
            set_if_changed(&mut render.flip_main_axis, widget.flip_vertical),
            set_if_changed(&mut render.flip_cross_axis, widget.flip_horizontal),
        ]
        .iter()
        .any(|&changed| changed)
        .then_some(RenderAction::Relayout)
    }
}
