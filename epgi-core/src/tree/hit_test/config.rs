use crate::{
    foundation::{Arc, Canvas, Protocol, Transform},
    tree::ArcChildRenderObject,
};

use super::HitTestNodeChild;

pub struct HitTestConfig<PP: Protocol, CP: Protocol> {
    pub(crate) self_is_hit: bool,
    pub(crate) children: Vec<(
        ArcChildRenderObject<CP>,
        CP::Transform,
        Option<<CP::Canvas as Canvas>::Transform>,
    )>,
    pub(crate) layer_transform: HitTestLayerTransform<PP::Canvas, CP::Canvas>,
}

pub enum HitTestLayerTransform<PC: Canvas, CC: Canvas> {
    None {
        cast_hit_position_ref: fn(&PC::HitPosition) -> &CC::HitPosition,
        cast_hit_test_node_child: fn(HitTestNodeChild<CC>) -> HitTestNodeChild<PC>,
    },
    Layer {
        transform: Arc<dyn Transform<PC, CC>>,
    },
}

impl<PP, CP> HitTestConfig<PP, CP>
where
    PP: Protocol,
    CP: Protocol<Canvas = PP::Canvas>,
{
    pub const fn empty() -> Self {
        Self {
            self_is_hit: false,
            children: Vec::new(),
            layer_transform: HitTestLayerTransform::None {
                cast_hit_position_ref: |x| x,
                cast_hit_test_node_child: |x| x,
            },
        }
    }

    pub fn new_in_layer(
        self_is_hit: bool,
        children: impl IntoIterator<
            Item = (
                ArcChildRenderObject<CP>,
                CP::Transform,
                Option<<CP::Canvas as Canvas>::Transform>,
            ),
        >,
    ) -> Self {
        Self {
            self_is_hit,
            children: children.into_iter().collect(),
            layer_transform: HitTestLayerTransform::None {
                cast_hit_position_ref: |x| x,
                cast_hit_test_node_child: |x| x,
            },
        }
    }

    pub fn new_single_in_layer(
        self_is_hit: bool,
        child: ArcChildRenderObject<CP>,
        transform: CP::Transform,
        canvas_transform: Option<<CP::Canvas as Canvas>::Transform>,
    ) -> Self {
        Self {
            self_is_hit,
            children: [(child, transform, canvas_transform)].into(),
            layer_transform: HitTestLayerTransform::None {
                cast_hit_position_ref: |x| x,
                cast_hit_test_node_child: |x| x,
            },
        }
    }
}

impl<PP, CP> HitTestConfig<PP, CP>
where
    PP: Protocol,
    CP: Protocol,
{
    pub fn is_empty(&self) -> bool {
        !self.self_is_hit && self.children.is_empty()
    }
}
