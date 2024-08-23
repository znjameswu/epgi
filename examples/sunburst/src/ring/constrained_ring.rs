use std::sync::Arc;

use epgi_core::{
    foundation::{set_if_changed, Asc, BuildSuspendedError, InlinableDwsizeVec, Provide},
    template::{
        ImplByTemplate, ProxyRender, ProxyRenderTemplate, SingleChildElement,
        SingleChildElementTemplate, SingleChildRenderElement,
    },
    tree::{BuildContext, RenderAction, Widget},
};
use epgi_macro::Declarative;
use typed_builder::TypedBuilder;

use crate::ARC_PHANTOM_RING;

use super::{ArcRingRenderObject, ArcRingWidget, RingConstraints, RingProtocol, RingSize};

#[derive(Declarative, TypedBuilder)]
#[builder(build_method(into=Asc<ConstrainedRing>))]
#[derive(Debug)]
pub struct ConstrainedRing {
    pub constraints: RingConstraints,
    #[builder(default=ARC_PHANTOM_RING.clone())]
    pub child: ArcRingWidget,
}

impl Widget for ConstrainedRing {
    type ParentProtocol = RingProtocol;
    type ChildProtocol = RingProtocol;
    type Element = ConstrainedRingElement;

    fn into_arc_widget(self: Arc<Self>) -> Asc<Self> {
        self
    }
}

#[derive(Clone)]
pub struct ConstrainedRingElement;

impl ImplByTemplate for ConstrainedRingElement {
    type Template = SingleChildElementTemplate<true, false>;
}

impl SingleChildElement for ConstrainedRingElement {
    type ParentProtocol = RingProtocol;
    type ChildProtocol = RingProtocol;
    type ArcWidget = Asc<ConstrainedRing>;

    fn get_child_widget(
        _element: Option<&mut Self>,
        widget: &Self::ArcWidget,
        _ctx: &mut BuildContext<'_>,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<ArcRingWidget, BuildSuspendedError> {
        Ok(widget.child.clone())
    }

    fn create_element(_widget: &Self::ArcWidget) -> Self {
        Self
    }
}

impl SingleChildRenderElement for ConstrainedRingElement {
    type Render = RenderConstrainedRing;

    fn create_render(&self, widget: &Self::ArcWidget) -> Self::Render {
        RenderConstrainedRing {
            constraints: widget.constraints.clone(),
        }
    }

    fn update_render(render: &mut Self::Render, widget: &Self::ArcWidget) -> Option<RenderAction> {
        set_if_changed(&mut render.constraints, widget.constraints)
            .then_some(RenderAction::Relayout)
    }
}

pub struct RenderConstrainedRing {
    constraints: RingConstraints,
}

impl ImplByTemplate for RenderConstrainedRing {
    type Template = ProxyRenderTemplate;
}

impl ProxyRender for RenderConstrainedRing {
    type Protocol = RingProtocol;

    const NOOP_DETACH: bool = true;

    fn perform_layout(
        &mut self,
        constraints: &RingConstraints,
        child: &ArcRingRenderObject,
    ) -> RingSize {
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
