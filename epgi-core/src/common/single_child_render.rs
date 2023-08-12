use crate::foundation::{
    Arc, Asc, BuildSuspendedError, InlinableDwsizeVec, Never, PaintContext, Protocol, Provide,
};

use super::{
    ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, Element, PerformDryLayout,
    PerformLayerPaint, ReconcileItem, Reconciler, Render, RenderObject, RenderObjectUpdateResult,
    Widget,
};

pub trait SingleChildRenderObjectWidget:
    Widget<Element = SingleChildRenderObjectElement<Self>> + Sized
{
    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol;
    type RenderState: Send + Sync;

    fn child(&self) -> &ArcChildWidget<Self::ChildProtocol>;

    fn create_render_state(&self) -> Self::RenderState;

    fn update_render_state(&self, render_state: &mut Self::RenderState)
        -> RenderObjectUpdateResult;

    const NOOP_UPDATE_RENDER_OBJECT: bool = false;

    fn detach_render_state(render_state: &mut Self::RenderState);

    const NOOP_DETACH: bool = false;

    type LayoutMemo: Send + Sync + 'static;

    fn perform_layout(
        state: &Self::RenderState,
        child: &ArcChildRenderObject<Self::ChildProtocol>,
        constraints: &<<Self::Element as Element>::ParentProtocol as Protocol>::Constraints,
    ) -> (
        <<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        Self::LayoutMemo,
    );

    /// If this is not None, then [`Self::perform_layout`]'s implementation will be ignored.
    const PERFORM_DRY_LAYOUT: Option<PerformDryLayout<SingleChildRenderObject<Self>>> = None;

    // We don't make perform paint into an associated constant because it has an generic paramter
    // Then we have to go to associated generic type, which makes the boilerplate explodes.
    fn perform_paint(
        state: &Self::RenderState,
        child: &ArcChildRenderObject<Self::ChildProtocol>,
        size: &<<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        transform: &<<Self::Element as Element>::ParentProtocol as Protocol>::Transform,
        memo: &Self::LayoutMemo,
        paint_ctx: &mut impl PaintContext<
            Canvas = <<Self::Element as Element>::ParentProtocol as Protocol>::Canvas,
        >,
    );

    /// If this is not None, then [`Self::perform_paint`]'s implementation will be ignored.
    const PERFORM_LAYER_PAINT: Option<PerformLayerPaint<SingleChildRenderObject<Self>>> = None;
}

pub struct SingleChildRenderObjectElement<W: SingleChildRenderObjectWidget> {
    pub child: ArcChildElementNode<W::ChildProtocol>,
}

impl<W> Clone for SingleChildRenderObjectElement<W>
where
    W: SingleChildRenderObjectWidget,
{
    fn clone(&self) -> Self {
        Self {
            child: self.child.clone(),
        }
    }
}

impl<W> Element for SingleChildRenderObjectElement<W>
where
    W: SingleChildRenderObjectWidget<Element = Self>,
{
    type ArcWidget = Asc<W>;

    type ParentProtocol = W::ParentProtocol;

    type ChildProtocol = W::ChildProtocol;

    type Provided = Never;

    #[inline(always)]
    fn perform_rebuild_element(
        self,
        widget: &Self::ArcWidget,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        mut reconciler: impl Reconciler<Self::ChildProtocol>,
    ) -> Result<Self, (Self, BuildSuspendedError)> {
        match self.child.can_rebuild_with(widget.child().clone()) {
            Ok(item) => {
                let [child] = reconciler.into_reconcile([item]);
                Ok(Self { child })
            }
            Err((child, child_widget)) => {
                reconciler.nodes_needing_unmount_mut().push(child);
                let [child] = reconciler.into_reconcile([ReconcileItem::new_inflate(child_widget)]);
                Ok(Self { child })
            }
        }
    }

    #[inline(always)]
    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        reconciler: impl Reconciler<Self::ChildProtocol>, // TODO: A specialized reconciler for inflate, to save passing &JobIds
    ) -> Result<Self, BuildSuspendedError> {
        let [child] =
            reconciler.into_reconcile([ReconcileItem::new_inflate(widget.child().clone())]);
        Ok(Self { child })
    }

    type ChildIter = [ArcChildElementNode<W::ChildProtocol>; 1];

    #[inline(always)]
    fn children(&self) -> Self::ChildIter {
        [self.child.clone()]
    }

    type ArcRenderObject = Arc<RenderObject<SingleChildRenderObject<W>>>;
}

pub struct SingleChildRenderObject<W: SingleChildRenderObjectWidget> {
    pub state: W::RenderState,
    pub child: ArcChildRenderObject<W::ChildProtocol>,
}

impl<W> Render for SingleChildRenderObject<W>
where
    W: SingleChildRenderObjectWidget,
{
    type Element = W::Element;

    type ChildIter = [ArcChildRenderObject<W::ChildProtocol>; 1];

    #[inline(always)]
    fn children(&self) -> Self::ChildIter {
        [self.child.clone()]
    }

    #[inline(always)]
    fn try_create_render_object_from_element(
        element: &Self::Element,
        widget: &<Self::Element as Element>::ArcWidget,
    ) -> Option<Self> {
        let child = element.child.get_current_subtree_render_object()?;
        Some(Self {
            state: W::create_render_state(widget),
            child,
        })
    }

    #[inline(always)]
    fn update_render_object(
        &mut self,
        widget: &<Self::Element as Element>::ArcWidget,
    ) -> RenderObjectUpdateResult {
        W::update_render_state(widget, &mut self.state)
    }
    const NOOP_UPDATE_RENDER_OBJECT: bool = W::NOOP_UPDATE_RENDER_OBJECT;

    #[inline(always)]
    fn try_update_render_object_children(&mut self, element: &Self::Element) -> Result<(), ()> {
        let child = element
            .child
            .get_current_subtree_render_object()
            .ok_or(())?;
        self.child = child;
        Ok(())
    }
    const NOOP_DETACH: bool = W::NOOP_DETACH;

    type LayoutMemo = W::LayoutMemo;

    #[inline(always)]
    fn perform_layout(
        &self,
        constraints: &<<Self::Element as Element>::ParentProtocol as Protocol>::Constraints,
    ) -> (
        <<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        Self::LayoutMemo,
    ) {
        W::perform_layout(&self.state, &self.child, constraints)
    }

    #[inline(always)]
    fn perform_paint(
        &self,
        size: &<<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        transform: &<<Self::Element as Element>::ParentProtocol as Protocol>::Transform,
        memo: &Self::LayoutMemo,
        paint_ctx: &mut impl PaintContext<
            Canvas = <<Self::Element as Element>::ParentProtocol as Protocol>::Canvas,
        >,
    ) {
        W::perform_paint(&self.state, &self.child, size, transform, memo, paint_ctx)
    }
}
