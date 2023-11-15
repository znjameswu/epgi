use crate::{
    foundation::{
        Arc, ArrayContainer, Asc, BuildSuspendedError, InlinableDwsizeVec, Key, Never,
        PaintContext, Protocol, Provide,
    },
    tree::{
        ArcChildElementNode, ArcChildWidget, Element, Render, RenderAction, RenderElement,
        SuspenseElementFunctionTable, Widget,
    },
};

#[derive(Clone)]
pub struct SuspenseElement<P: Protocol> {
    pub(crate) fallback_widget: ArcChildWidget<P>,
    pub(crate) fallback: Option<ArcChildElementNode<P>>,
}

#[derive(Debug)]
pub struct Suspense<P: Protocol> {
    child: ArcChildWidget<P>,
    fallback: ArcChildWidget<P>,
    key: Option<Box<dyn Key>>,
}

impl<P: Protocol> Widget for Suspense<P> {
    type ParentProtocol = P;
    type ChildProtocol = P;
    type Element = SuspenseElement<P>;

    fn key(&self) -> Option<&dyn Key> {
        todo!()
    }

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as Element>::ArcWidget {
        todo!()
    }
}

impl<P: Protocol> Element for SuspenseElement<P> {
    type ArcWidget = Asc<Suspense<P>>;

    type ParentProtocol = P;

    type ChildProtocol = P;

    type ChildContainer = ArrayContainer<1>;

    type Provided = Never;

    // type ReturnResults = BoxFuture<'static, BuildResults<Self>>;

    fn perform_rebuild_element(
        &mut self,
        widget: &Self::ArcWidget,
        ctx: crate::tree::BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        children: crate::tree::ContainerOf<Self, ArcChildElementNode<Self::ChildProtocol>>,
        nodes_needing_unmount: &mut InlinableDwsizeVec<ArcChildElementNode<Self::ChildProtocol>>,
    ) -> Result<
        (
            crate::tree::ContainerOf<Self, crate::tree::ElementReconcileItem<Self::ChildProtocol>>,
            Option<crate::tree::ChildRenderObjectsUpdateCallback<Self>>,
        ),
        (
            crate::tree::ContainerOf<Self, ArcChildElementNode<Self::ChildProtocol>>,
            BuildSuspendedError,
        ),
    > {
        todo!()
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        ctx: crate::tree::BuildContext<'_>,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
    ) -> Result<
        (
            Self,
            crate::tree::ContainerOf<Self, ArcChildWidget<Self::ChildProtocol>>,
        ),
        BuildSuspendedError,
    > {
        todo!()
    }

    type RenderOrUnit = RenderSuspense<P>;
}

impl<P: Protocol> RenderElement for SuspenseElement<P> {
    type Render = RenderSuspense<P>;

    fn create_render(&self, widget: &Self::ArcWidget) -> RenderSuspense<P> {
        todo!()
    }

    fn update_render(
        render_object: &mut RenderSuspense<P>,
        widget: &Self::ArcWidget,
    ) -> RenderAction {
        todo!()
    }

    const SUSPENSE_ELEMENT_FUNCTION_TABLE: Option<SuspenseElementFunctionTable<Self>> =
        Some(SuspenseElementFunctionTable {
            get_suspense_element_mut: |x| x,
            get_suspense_widget_ref: |x| x,
            get_suspense_render_object: |x| x,
            into_arc_render_object: |x| x,
        });

    fn element_render_children_mapping<T: Send + Sync>(
        &self,
        element_children: <Self::ChildContainer as crate::foundation::HktContainer>::Container<T>,
    ) -> <<RenderSuspense<P> as Render>::ChildContainer as crate::foundation::HktContainer>::Container<T>{
        element_children
    }
}

pub struct RenderSuspense<P: Protocol> {
    fallback: ArcChildWidget<P>,
    is_suspended: bool,
}

impl<P: Protocol> Render for RenderSuspense<P> {
    type ParentProtocol = P;

    type ChildProtocol = P;

    type ChildContainer = ArrayContainer<1>;

    const NOOP_DETACH: bool = true;

    type LayoutMemo = ();

    fn perform_layout<'a, 'layout>(
        &'a self,
        constraints: &'a <Self::ParentProtocol as Protocol>::Constraints,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo) {
        unreachable!()
    }

    fn perform_paint(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        transform: &<Self::ParentProtocol as Protocol>::Transform,
        memo: &Self::LayoutMemo,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) {
        todo!()
    }

    type LayerOrUnit = ();
}
