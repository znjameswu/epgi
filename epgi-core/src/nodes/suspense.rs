use crate::{
    foundation::{
        Arc, Asc, BuildSuspendedError, EitherParallel, InlinableDwsizeVec, Key, Never,
        PaintContext, Protocol, Provide,
    },
    tree::{
        ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, Element, GetSuspense,
        Reconciler, Render, RenderElement, RenderObject, RenderObjectUpdateResult, Widget,
    },
};

#[derive(Clone)]
pub struct SuspenseElement<P: Protocol> {
    pub(crate) child: ArcChildElementNode<P>,
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

    type Provided = Never;

    // type ReturnResults = BoxFuture<'static, BuildResults<Self>>;

    fn perform_rebuild_element(
        self,
        widget: &Self::ArcWidget,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        reconciler: impl Reconciler<Self::ChildProtocol>,
    ) -> Result<Self, (Self, BuildSuspendedError)> {
        // Suspense needs to reconcile fallback if it is available. To avoid missing propagation rebuilds.
        todo!()
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        reconciler: impl Reconciler<Self::ChildProtocol>,
    ) -> Result<Self, BuildSuspendedError> {
        todo!()
    }

    type ChildIter = EitherParallel<[ArcChildElementNode<P>; 1], [ArcChildElementNode<P>; 2]>;

    fn children(&self) -> Self::ChildIter {
        todo!()
    }

    type ArcRenderObject = Arc<RenderObject<RenderSuspense<P>>>;
}

impl<P: Protocol> RenderElement for SuspenseElement<P> {
    type Render = RenderSuspense<P>;

    fn try_create_render_object(&self, widget: &Self::ArcWidget) -> Option<Self::Render> {
        todo!()
    }

    fn update_render_object(
        render_object: &mut Self::Render,
        widget: &Self::ArcWidget,
    ) -> RenderObjectUpdateResult {
        todo!()
    }

    fn try_update_render_object_children(&self, render: &mut Self::Render) -> Result<(), ()> {
        let child_render_object = self
            .fallback
            .as_ref()
            .unwrap_or(&self.child)
            .get_current_subtree_render_object()
            .expect(if self.fallback.is_some() {
                "Fallback must never suspend"
            } else {
                "Child subtree must not suspend if fallback path is not inflated"
            });
        render.child = child_render_object;
        Ok(())
    }

    const GET_SUSPENSE: Option<GetSuspense<Self>> = Some(GetSuspense {
        get_suspense_element_mut: |x| x,
        get_suspense_widget_ref: |x| x,
        get_suspense_render_object: |x| x,
        into_arc_render_object: |x| x,
    });
}

pub struct RenderSuspense<P: Protocol> {
    pub(crate) child: ArcChildRenderObject<P>,
    fallback: ArcChildWidget<P>,
    is_suspended: bool,
}

impl<P: Protocol> Render for RenderSuspense<P> {
    type ParentProtocol = P;

    type ChildProtocol = P;

    type ChildIter = [ArcChildRenderObject<P>; 1];

    fn children(&self) -> Self::ChildIter {
        todo!()
    }

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

    type ArcLayerNode = ();
}
