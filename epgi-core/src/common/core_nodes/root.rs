use vello::util::RenderContext;

use crate::{
    common::{
        ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, Element, GetSuspense,
        PerformLayout, Reconciler, Render, RenderElement, RenderObject, Widget,
    },
    foundation::{
        Arc, Asc, BoxProtocol, BuildSuspendedError, InlinableDwsizeVec, Key, Never, Protocol,
        Provide,
    }, integrations::RenderRootView,
};

#[derive(Debug)]
pub struct RootView {
    child: ArcChildWidget<BoxProtocol>,
}

impl Widget for RootView {
    type Element = RootViewElement;

    fn key(&self) -> &dyn Key {
        todo!()
    }

    fn create_element(self: crate::foundation::Asc<Self>) -> Self::Element {
        todo!()
    }

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as Element>::ArcWidget {
        todo!()
    }
}

#[derive(Clone)]
pub struct RootViewElement {}

impl Element for RootViewElement {
    type ArcWidget = Asc<RootView>;

    type SelfProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Provided = Never;

    fn perform_rebuild_element(
        // Rational for a moving self: Allows users to destructure the self without needing to fill in a placeholder value.
        self,
        widget: &Self::ArcWidget,
        provider_values: InlinableDwsizeVec<crate::foundation::Arc<dyn Provide>>,
        reconciler: impl Reconciler<Self::ChildProtocol>,
    ) -> Result<Self, (Self, BuildSuspendedError)> {
        todo!()
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        reconciler: impl Reconciler<Self::ChildProtocol>, // TODO: A specialized reconciler for inflate, to save passing &JobIds
    ) -> Result<Self, BuildSuspendedError> {
        todo!()
    }

    type ChildIter = [ArcChildElementNode<BoxProtocol>; 1];

    fn children(&self) -> Self::ChildIter {
        todo!()
    }

    type ArcRenderObject = Arc<RenderObject<RenderRootView>>;
}

impl RenderElement for RootViewElement {
    type Render = RenderRootView;

    fn try_create_render_object(
        &self,
        widget: &Self::ArcWidget,
    ) -> Option<Arc<RenderObject<Self::Render>>> {
        todo!()
    }

    fn update_render_object_widget(
        widget: &Self::ArcWidget,
        render_object: &Arc<RenderObject<Self::Render>>,
    ) {
        todo!()
    }

    fn try_update_render_object_children(
        &self,
        render_object: &Arc<RenderObject<Self::Render>>,
    ) -> Result<(), ()> {
        todo!()
    }

    fn detach_render_object(render_object: &Arc<RenderObject<Self::Render>>) {
        todo!()
    }

    const GET_SUSPENSE: Option<GetSuspense<Self>> = None;
}

