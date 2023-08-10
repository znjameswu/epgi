use std::sync::Arc;

use epgi_2d::{BoxConstraints, BoxProtocol, BoxSize};
use epgi_core::common::{
    ArcChildRenderObject, ArcChildWidget, Element, ProxyWidget, RenderObjectUpdateResult,
    SingleChildRenderObjectElement, Widget,
};

#[derive(Debug)]
pub struct ConstrainedBox {
    constraints: BoxConstraints,
    child: ArcChildWidget<BoxProtocol>,
}

impl Widget for ConstrainedBox {
    type Element = SingleChildRenderObjectElement<Self>;

    fn into_arc_widget(self: Arc<Self>) -> <Self::Element as Element>::ArcWidget {
        self
    }
}

impl ProxyWidget for ConstrainedBox {
    type Protocol = BoxProtocol;

    type RenderState = BoxConstraints;

    fn child(&self) -> &ArcChildWidget<BoxProtocol> {
        &self.child
    }

    fn create_render_state(&self) -> BoxConstraints {
        self.constraints.clone()
    }

    fn update_render_state(&self, render_state: &mut BoxConstraints) -> RenderObjectUpdateResult {
        if render_state != &self.constraints {
            *render_state = self.constraints.clone();
            return RenderObjectUpdateResult::MarkNeedsLayout;
        }
        return RenderObjectUpdateResult::None;
    }

    fn detach_render_state(_render_state: &mut Self::RenderState) {}

    const NOOP_DETACH: bool = true;

    type LayoutMemo = ();

    fn perform_layout(
        state: &BoxConstraints,
        child: &ArcChildRenderObject<BoxProtocol>,
        constraints: &BoxConstraints,
    ) -> (BoxSize, ()) {
        let child_constraints = state.enforce(constraints);
        let size = child.layout_use_size(&child_constraints);
        (size, ())
    }
}
