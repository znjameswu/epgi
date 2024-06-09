use epgi_2d::{
    ArcBoxRenderObject, ArcBoxWidget, BoxConstraints, BoxOffset, BoxSize, ShiftedBoxRender,
    ShiftedBoxRenderTemplate,
};
use epgi_core::{foundation::Asc, template::ImplByTemplate};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

#[derive(Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<Align>))]
pub struct Align {
    pub alignment: Alignment,
    #[builder(default)]
    pub width_factor: Option<f32>,
    #[builder(default)]
    pub height_factor: Option<f32>,
    pub child: ArcBoxWidget,
}

pub struct Alignment {
    pub x: f32,
    pub y: f32,
}

impl Alignment {
    pub fn along_offset(&self, offset: BoxOffset) -> BoxOffset {
        let center_x = offset.x / 2.0;
        let center_y = offset.y / 2.0;
        return BoxOffset {
            x: center_x + center_x * self.x,
            y: center_y + center_y * self.y,
        };
    }
}

pub struct AlignElement {}

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
        offset: &BoxOffset,
        child_extra_offset: &Self::LayoutMemo,
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
