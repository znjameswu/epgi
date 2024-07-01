use std::sync::Arc;

use epgi_2d::{
    BoxConstraints, BoxProtocol, BoxSingleChildElement, BoxSingleChildElementTemplate,
    BoxSingleChildRenderElement, BoxSize,
};
use epgi_core::{
    foundation::{set_if_changed, Asc, BuildSuspendedError, InlinableDwsizeVec, Provide},
    template::{ImplByTemplate, ProxyRender, ProxyRenderTemplate},
    tree::{ArcChildRenderObject, ArcChildWidget, BuildContext, ElementBase, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::ARC_PHANTOM_BOX;

#[derive(Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<ConstrainedBox>))]
#[derive(Debug)]
pub struct ConstrainedBox {
    pub constraints: BoxConstraints,
    #[builder(default=ARC_PHANTOM_BOX.clone())]
    pub child: ArcChildWidget<BoxProtocol>,
}

impl Widget for ConstrainedBox {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Element = ConstrainedBoxElement;

    fn into_arc_widget(self: Arc<Self>) -> <Self::Element as ElementBase>::ArcWidget {
        self
    }
}

#[derive(Clone)]
pub struct ConstrainedBoxElement;

impl ImplByTemplate for ConstrainedBoxElement {
    type Template = BoxSingleChildElementTemplate<true, false>;
}

impl BoxSingleChildElement for ConstrainedBoxElement {
    type ArcWidget = Asc<ConstrainedBox>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcChildWidget<BoxProtocol>, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self
    }
}

impl BoxSingleChildRenderElement for ConstrainedBoxElement {
    type Render = RenderConstrainedBox;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderConstrainedBox {
            constraints: widget.constraints.clone(),
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        set_if_changed(&mut render.constraints, widget.constraints)
            .then_some(RenderAction::Relayout)
    }
}

pub struct RenderConstrainedBox {
    constraints: BoxConstraints,
}

impl ImplByTemplate for RenderConstrainedBox {
    type Template = ProxyRenderTemplate;
}

impl ProxyRender for RenderConstrainedBox {
    type Protocol = BoxProtocol;

    const NOOP_DETACH: bool = true;

    fn perform_layout(
        &mut self,
        constraints: &BoxConstraints,
        child: &ArcChildRenderObject<BoxProtocol>,
    ) -> BoxSize {
        let child_constraints = self.constraints.enforce(constraints);
        if let Some(size) = child_constraints.is_tight() {
            child.layout(&child_constraints);
            return size;
        } else {
            let size = child.layout_use_size(&child_constraints);
            return size;
        }
    }
}
