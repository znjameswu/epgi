use crate::{
    common::{
        ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, Element, GetSuspense,
        PerformLayout, Reconciler, Render, RenderElement, RenderObject, Widget,
    },
    foundation::{
        Arc, Asc, BuildSuspendedError, EitherParallel, InlinableDwsizeVec, Key, Never, Protocol,
        Provide,
    }, rendering::PaintingContext,
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
    type Element = SuspenseElement<P>;

    fn key(&self) -> &dyn Key {
        todo!()
    }

    fn create_element(self: Asc<Self>) -> Self::Element {
        todo!()
    }

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as Element>::ArcWidget {
        todo!()
    }
}

impl<P: Protocol> Element for SuspenseElement<P> {
    type ArcWidget = Asc<Suspense<P>>;

    type SelfProtocol = P;

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

impl<P> RenderElement for SuspenseElement<P>
where
    P: Protocol,
{
    type Render = RenderSuspense<P>;

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
    }

    fn try_update_render_object_children(
        &self,
        render_object: &Arc<RenderObject<Self::Render>>,
    ) -> Result<(), ()> {
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
        render_object.inner.lock().render.child = child_render_object;
        Ok(())
    }

    fn detach_render_object(render_object: &Arc<RenderObject<Self::Render>>) {
        todo!()
    }

    const GET_SUSPENSE: Option<GetSuspense<Self>> = Some(GetSuspense {
        get_suspense_element_mut: |x| x,
        get_suspense_widget_ref: |x| x,
        get_suspense_render_object: |x| x,
        into_arc_render_object: |x| x,
    });
}

impl<P> SuspenseElement<P>
where
    P: Protocol,
{
    // fn
}

pub struct RenderSuspense<P: Protocol> {
    pub(crate) child: ArcChildRenderObject<P>,
    fallback: ArcChildWidget<P>,
    is_suspended: bool,
}

impl<P: Protocol> Render for RenderSuspense<P> {
    type Element = SuspenseElement<P>;

    type ChildIter = [ArcChildRenderObject<P>; 1];

    fn get_children(&self) -> Self::ChildIter {
        todo!()
    }

    fn set_children(&mut self, new_children: Self::ChildIter) {
        todo!()
    }

    type LayoutMemo = ();

    // fn perform_layout(&self, constraints: &P::Constraints) -> (P::Size, Self::LayoutMemo) {
    //     todo!()
    // }

    const PERFORM_LAYOUT: PerformLayout<Self> = PerformLayout::DryLayout {
        compute_dry_layout: todo!(),
        perform_layout: todo!(),
    };

    fn perform_paint(
        &self,
        size: &<<Self::Element as Element>::SelfProtocol as Protocol>::Size,
        transformation: &<<Self::Element as Element>::SelfProtocol as Protocol>::CanvasTransformation,
        memo: &Self::LayoutMemo,
        paint_ctx: &mut impl PaintingContext<<<Self::Element as Element>::SelfProtocol as Protocol>::Canvas>,
    ) {
        todo!()
    }
}
