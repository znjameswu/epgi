use epgi_2d::{
    ArcBoxRenderObject, ArcBoxWidget, BoxConstraints, BoxOffset, BoxProtocol,
    BoxSingleChildElement, BoxSingleChildElementTemplate, BoxSingleChildRenderElement, BoxSize,
    ShiftedBoxRender, ShiftedBoxRenderTemplate,
};
use epgi_core::{
    foundation::{set_if_changed, Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Provide},
    max,
    template::ImplByTemplate,
    tree::{ArcChildWidget, BuildContext, ElementBase, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::Lerp;

#[derive(Debug, Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Align>))]
pub struct Align {
    pub alignment: Alignment,
    #[builder(default)]
    pub width_factor: Option<f32>,
    #[builder(default)]
    pub height_factor: Option<f32>,
    pub child: ArcBoxWidget,
}

#[derive(Lerp, PartialEq, Clone, Copy, Debug)]
pub struct Alignment {
    pub x: f32,
    pub y: f32,
}

impl Alignment {
    pub const TOP_LEFT: Self = Self { x: -1.0, y: -1.0 };
    pub const TOP_CENTER: Self = Self { x: 0.0, y: -1.0 };
    pub const TOP_RIGHT: Self = Self { x: 1.0, y: -1.0 };
    pub const CENTER_LEFT: Self = Self { x: -1.0, y: 0.0 };
    pub const CENTER: Self = Self { x: 0.0, y: 0.0 };
    pub const CENTER_RIGHT: Self = Self { x: 1.0, y: 0.0 };
    pub const BOTTOM_LEFT: Self = Self { x: -1.0, y: 1.0 };
    pub const BOTTOM_CENTER: Self = Self { x: 0.0, y: 1.0 };
    pub const BOTTOM_RIGHT: Self = Self { x: 1.0, y: 1.0 };

    pub fn along_offset(&self, offset: BoxOffset) -> BoxOffset {
        let center_x = offset.x / 2.0;
        let center_y = offset.y / 2.0;
        return BoxOffset {
            x: center_x + center_x * self.x,
            y: center_y + center_y * self.y,
        };
    }
}

impl Widget for Align {
    type ParentProtocol = BoxProtocol;
    type ChildProtocol = BoxProtocol;
    type Element = AlignElement;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

#[derive(Clone)]
pub struct AlignElement {}

impl ImplByTemplate for AlignElement {
    type Template = BoxSingleChildElementTemplate<true, false>;
}

impl BoxSingleChildElement for AlignElement {
    type ArcWidget = Asc<Align>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<epgi_2d::BoxProtocol>, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self {}
    }
}

impl BoxSingleChildRenderElement for AlignElement {
    type Render = RenderPositionedBox;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderPositionedBox {
            alignment: widget.alignment,
            width_factor: widget.width_factor,
            height_factor: widget.height_factor,
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        max!(
            set_if_changed(&mut render.alignment, widget.alignment)
                .then_some(RenderAction::Relayout),
            set_if_changed(&mut render.width_factor, widget.width_factor)
                .then_some(RenderAction::Relayout),
            set_if_changed(&mut render.height_factor, widget.height_factor)
                .then_some(RenderAction::Relayout),
        )
    }
}

pub struct RenderPositionedBox {
    pub alignment: Alignment,
    pub width_factor: Option<f32>,
    pub height_factor: Option<f32>,
}

impl ImplByTemplate for RenderPositionedBox {
    type Template = ShiftedBoxRenderTemplate;
}

impl ShiftedBoxRender for RenderPositionedBox {
    type LayoutMemo = BoxOffset;

    fn get_child_offset(
        &self,
        _size: &BoxSize,
        &offset: &BoxOffset,
        &child_extra_offset: &BoxOffset,
    ) -> BoxOffset {
        offset + child_extra_offset
    }

    fn perform_layout(
        &mut self,
        constraints: &BoxConstraints,
        child: &ArcBoxRenderObject,
    ) -> (BoxSize, Self::LayoutMemo) {
        let shrink_warp_width =
            self.width_factor.is_some() || constraints.max_width == f32::INFINITY;
        let shrink_warp_height =
            self.height_factor.is_some() || constraints.max_height == f32::INFINITY;

        let child_size = child.layout_use_size(&constraints.loosen());

        let size = constraints.constrain(BoxSize {
            width: if shrink_warp_width {
                child_size.width * self.width_factor.unwrap_or(1.0)
            } else {
                f32::INFINITY
            },
            height: if shrink_warp_height {
                child_size.height * self.height_factor.unwrap_or(1.0)
            } else {
                f32::INFINITY
            },
        });

        let child_extra_offset = self.alignment.along_offset(BoxOffset {
            x: size.width - child_size.width,
            y: size.height - child_size.height,
        });
        (size, child_extra_offset)
    }
}