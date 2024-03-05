use std::sync::Arc;

use epgi_2d::{BoxConstraints, BoxProtocol, BoxSize};
use epgi_core::{
    nodes::{ProxyWidget, SingleChildRenderObjectElement},
    tree::{ArcChildRenderObject, ArcChildWidget, Element, RenderAction, Widget},
};

#[derive(Debug, optargs::OptStructArc)]
pub struct ConstrainedBox {
    pub constraints: BoxConstraints,
    pub child: ArcChildWidget<BoxProtocol>,
}

impl Widget for ConstrainedBox {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

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

    fn update_render_state(&self, render_state: &mut BoxConstraints) -> RenderAction {
        if render_state != &self.constraints {
            *render_state = self.constraints.clone();
            return RenderAction::Relayout;
        }
        return RenderAction::None;
    }

    fn detach_render_state(_render_state: &mut Self::RenderState) {}

    const NOOP_DETACH: bool = true;

    type LayoutMemo = ();

    fn perform_layout(
        state: &BoxConstraints,
        constraints: &BoxConstraints,
        child: &ArcChildRenderObject<BoxProtocol>,
    ) -> (BoxSize, ()) {
        let child_constraints = state.enforce(constraints);
        if let Some(size) = child_constraints.is_tight() {
            child.layout(&child_constraints);
            return (size, ());
        } else {
            let size = child.layout_use_size(&child_constraints);
            return (size, ());
        }
    }

    type LayerOrUnit = ();
}
