use crate::foundation::{
    Arc, BuildSuspendedError, Canvas, InlinableDwsizeVec, Never, PaintContext, Protocol, Provide,
};

use crate::tree::{
    ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, ArcElementContextNode, ArcLayerOf,
    AscRenderContextNode, Element, LayerPaint, LayerScope, Reconciler, Render, RenderObject,
    RenderObjectUpdateResult, Widget,
};

#[derive(Debug)]
pub struct RepaintBoundary<P: Protocol> {
    child: ArcChildWidget<P>,
}

impl<P> Widget for RepaintBoundary<P>
where
    P: Protocol<Transform = <<P as Protocol>::Canvas as Canvas>::Transform>,
{
    type Element = RepaintBoundaryElement<P>;

    fn into_arc_widget(self: Arc<Self>) -> <Self::Element as Element>::ArcWidget {
        self
    }
}

#[derive(Clone)]
pub struct RepaintBoundaryElement<P: Protocol> {
    child: ArcChildElementNode<P>,
}

impl<P> Element for RepaintBoundaryElement<P>
where
    P: Protocol<Transform = <<P as Protocol>::Canvas as Canvas>::Transform>,
{
    type ArcWidget = Arc<RepaintBoundary<P>>;

    type ParentProtocol = P;

    type ChildProtocol = P;

    type Provided = Never;

    fn perform_rebuild_element(
        self,
        widget: &Self::ArcWidget,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        reconciler: impl Reconciler<Self::ChildProtocol>,
    ) -> Result<Self, (Self, BuildSuspendedError)> {
        todo!()
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        reconciler: impl Reconciler<Self::ChildProtocol>,
    ) -> Result<Self, BuildSuspendedError> {
        todo!()
    }

    type ChildIter = [ArcChildElementNode<P>; 1];

    fn children(&self) -> Self::ChildIter {
        [self.child.clone()]
    }

    type ArcRenderObject = Arc<RenderObject<RenderRepaintBoundary<P>>>;
}

pub struct RenderRepaintBoundary<P: Protocol> {
    layer: Option<Arc<LayerScope<P::Canvas>>>,
    child: ArcChildRenderObject<P>,
}

impl<P> Render for RenderRepaintBoundary<P>
where
    P: Protocol<Transform = <<P as Protocol>::Canvas as Canvas>::Transform>,
{
    type Element = RepaintBoundaryElement<P>;

    type ChildIter = [ArcChildRenderObject<P>; 1];

    fn children(&self) -> Self::ChildIter {
        todo!()
    }

    fn try_create_render_object_from_element(
        element: &Self::Element,
        widget: &<Self::Element as Element>::ArcWidget,
    ) -> Option<Self> {
        todo!()
    }

    fn update_render_object(
        &mut self,
        widget: &<Self::Element as Element>::ArcWidget,
    ) -> RenderObjectUpdateResult {
        todo!()
    }

    fn try_update_render_object_children(&mut self, element: &Self::Element) -> Result<(), ()> {
        todo!()
    }

    const NOOP_DETACH: bool = true;

    type LayoutMemo = ();

    fn perform_layout<'a, 'layout>(
        &'a self,
        constraints: &'a <<Self::Element as Element>::ParentProtocol as Protocol>::Constraints,
    ) -> (
        <<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        Self::LayoutMemo,
    ) {
        todo!()
    }

    fn perform_paint(
        &self,
        size: &<<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        transform: &<<Self::Element as Element>::ParentProtocol as Protocol>::Transform,
        memo: &Self::LayoutMemo,
        paint_ctx: &mut impl PaintContext<
            Canvas = <<Self::Element as Element>::ParentProtocol as Protocol>::Canvas,
        >,
    ) {
        todo!()
    }
}

impl<P: Protocol> LayerPaint for RenderRepaintBoundary<P>
where
    P: Protocol<Transform = <<P as Protocol>::Canvas as Canvas>::Transform>,
{
    fn get_layer_or_insert(
        &mut self,
        size: &<<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        transform: &<<Self::Element as Element>::ParentProtocol as Protocol>::Transform,
        memo: &Self::LayoutMemo,
        context: &AscRenderContextNode,
        transform_parent: &<<<Self::Element as Element>::ParentProtocol as Protocol>::Canvas as Canvas>::Transform,
    ) -> ArcLayerOf<Self> {
        match &self.layer {
            Some(layer) => todo!(),
            None => {
                let layer = LayerScope::new_structured(
                    context.clone(),
                    <<Self::Element as Element>::ParentProtocol as Protocol>::transform_canvas(
                        transform,
                        transform_parent,
                    ),
                );
                let layer = Arc::new(layer);
                self.layer = Some(layer.clone());
                layer
            }
        }
    }

    fn get_layer(&mut self) -> Option<ArcLayerOf<Self>> {
        self.layer.clone().map(|x| x as _)
    }
}