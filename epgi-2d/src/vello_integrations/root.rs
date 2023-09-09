use epgi_core::{
    foundation::{
        Arc, Asc, BuildSuspendedError, Canvas, InlinableDwsizeVec, LayerProtocol, Never,
        PaintContext, Protocol, Provide, SyncMutex,
    },
    nodes::RepaintBoundaryLayer,
    tree::{
        AnyLayer, ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, ArcElementContextNode,
        ArcLayerOf, AscLayerContextNode, AscRenderContextNode, BuildContext, ChildLayer,
        ChildLayerOrFragment, DryLayout, Element, Layer, LayerCompositionConfig, LayerPaint,
        PaintResults, ParentLayer, ReconcileItem, Reconciler, Render, RenderObject,
        RenderObjectUpdateResult, Widget,
    },
};

use crate::{Affine2dCanvas, BoxProtocol};

pub struct RootView {
    pub build: Box<dyn Fn(BuildContext) -> Option<ArcChildWidget<BoxProtocol>> + Send + Sync>,
}

impl std::fmt::Debug for RootView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RootView")
            // .field("child", &self.child)
            .finish()
    }
}

impl Widget for RootView {
    type Element = RootViewElement;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as Element>::ArcWidget {
        self
    }
}

#[derive(Clone)]
pub struct RootViewElement {
    pub child: Option<ArcChildElementNode<BoxProtocol>>,
}

impl Element for RootViewElement {
    type ArcWidget = Asc<RootView>;

    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Provided = Never;

    fn perform_rebuild_element(
        // Rational for a moving self: Allows users to destructure the self without needing to fill in a placeholder value.
        self,
        widget: &Self::ArcWidget,
        _provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        mut reconciler: impl Reconciler<Self::ChildProtocol>,
    ) -> Result<Self, (Self, BuildSuspendedError)> {
        let child_widget = (widget.build)(reconciler.build_context());
        match (child_widget, self.child) {
            (None, None) => Ok(Self { child: None }),
            (None, Some(child)) => {
                reconciler.nodes_needing_unmount_mut().push(child.clone());
                Ok(Self { child: None })
            }
            (Some(child_widget), None) => {
                let [child] = reconciler.into_reconcile([ReconcileItem::new_inflate(child_widget)]);
                Ok(Self { child: Some(child) })
            }
            (Some(child_widget), Some(child)) => match child.can_rebuild_with(child_widget) {
                Ok(item) => {
                    let [child] = reconciler.into_reconcile([item]);
                    Ok(Self { child: Some(child) })
                }
                Err((child, child_widget)) => {
                    reconciler.nodes_needing_unmount_mut().push(child);
                    let [child] =
                        reconciler.into_reconcile([ReconcileItem::new_inflate(child_widget)]);
                    Ok(Self { child: Some(child) })
                }
            },
        }
    }

    fn perform_inflate_element(
        widget: &Self::ArcWidget,
        provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
        mut reconciler: impl Reconciler<Self::ChildProtocol>, // TODO: A specialized reconciler for inflate, to save passing &JobIds
    ) -> Result<Self, BuildSuspendedError> {
        let child_widget = (widget.build)(reconciler.build_context());
        if let Some(child_widget) = child_widget {
            let [child] = reconciler.into_reconcile([ReconcileItem::new_inflate(child_widget)]);
            Ok(Self { child: Some(child) })
        } else {
            Ok(Self { child: None })
        }
    }

    type ChildIter = Option<ArcChildElementNode<BoxProtocol>>;

    fn children(&self) -> Self::ChildIter {
        self.child.clone()
    }

    type ArcRenderObject = Arc<RenderObject<RenderRootView>>;
}

pub struct RenderRootView {
    pub layer: Asc<RootLayer<BoxProtocol>>, //TODO!()
    pub child: Option<ArcChildRenderObject<BoxProtocol>>,
}

impl Render for RenderRootView {
    type Element = RootViewElement;

    type ChildIter = Option<ArcChildRenderObject<BoxProtocol>>;

    fn children(&self) -> Self::ChildIter {
        self.child.clone()
    }

    fn try_create_render_object_from_element(
        element: &Self::Element,
        widget: &<Self::Element as Element>::ArcWidget,
        context: &AscRenderContextNode,
    ) -> Option<Self> {
        todo!()
        // Some(Self {
        //     layer: Asc::new(LayerScope::new_structured(
        //         element_context,
        //         Affine2d::IDENTITY,
        //     )),
        //     child: element.child.map(|child| {
        //         child
        //             .get_current_subtree_render_object()
        //             .expect("Root ElementNode should never receive suspense event")
        //     }),
        // })
    }

    fn update_render_object(
        &mut self,
        _widget: &<Self::Element as Element>::ArcWidget,
    ) -> RenderObjectUpdateResult {
        RenderObjectUpdateResult::None
    }
    const NOOP_UPDATE_RENDER_OBJECT: bool = true;

    fn try_update_render_object_children(&mut self, element: &Self::Element) -> Result<(), ()> {
        self.child = element.child.as_ref().map(|child| {
            child
                .get_current_subtree_render_object()
                .expect("Root ElementNode should never receive suspense event")
        });
        Ok(())
    }

    type LayoutMemo = ();

    fn perform_layout<'a, 'layout>(
        &'a self,
        _constraints: &'a <<Self::Element as Element>::ParentProtocol as Protocol>::Constraints,
    ) -> (
        <<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        Self::LayoutMemo,
    ) {
        unreachable!()
    }

    const PERFORM_DRY_LAYOUT: Option<epgi_core::tree::PerformDryLayout<Self>> =
        Some(<Self as DryLayout>::PERFORM_DRY_LAYOUT);

    fn perform_paint(
        &self,
        _size: &<<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        _transform: &<<Self::Element as Element>::ParentProtocol as Protocol>::Transform,
        _memo: &Self::LayoutMemo,
        _paint_ctx: &mut impl PaintContext<
            Canvas = <<Self::Element as Element>::ParentProtocol as Protocol>::Canvas,
        >,
    ) {
        unreachable!()
    }
}

impl DryLayout for RenderRootView {
    fn compute_dry_layout(
        &self,
        constraints: &<<Self::Element as Element>::ParentProtocol as Protocol>::Constraints,
    ) -> <<Self::Element as Element>::ParentProtocol as Protocol>::Size {
        todo!()
    }

    fn perform_layout<'a, 'layout>(
        &'a self,
        constraints: &'a <<Self::Element as Element>::ParentProtocol as Protocol>::Constraints,
        size: &'a <<Self::Element as Element>::ParentProtocol as Protocol>::Size,
    ) -> Self::LayoutMemo {
        // self.render_ctx.resize_surface(&mut self.surface, size.width, size.height)
    }
}

impl LayerPaint for RenderRootView {
    fn get_layer(&self) -> ArcLayerOf<Self> {
        unimplemented!()
    }

    fn get_canvas_transform_ref(
        transform: &<<Self::Element as Element>::ParentProtocol as Protocol>::Transform,
    ) -> &<<<Self::Element as Element>::ParentProtocol as Protocol>::Canvas as Canvas>::Transform
    {
        todo!()
    }

    fn get_canvas_transform(
        transform: <<Self::Element as Element>::ParentProtocol as Protocol>::Transform,
    ) -> <<<Self::Element as Element>::ParentProtocol as Protocol>::Canvas as Canvas>::Transform
    {
        todo!()
    }
}

pub struct RootLayer<P: LayerProtocol> {
    pub context: AscLayerContextNode,
    pub inner: SyncMutex<RootLayerInner<P>>,
}

pub struct RootLayerInner<P: LayerProtocol> {
    /// This field is nullable because we temporarily share implementation with RootLayer
    child_render_object: Option<ArcChildRenderObject<P>>,
    paint_cache: Option<PaintResults<P::Canvas>>,
}

impl<P> RootLayer<P>
where
    P: LayerProtocol,
{
    pub fn new(
        context: AscLayerContextNode,
        child_render_object: Option<ArcChildRenderObject<P>>,
    ) -> Self {
        Self {
            context,
            inner: SyncMutex::new(RootLayerInner {
                child_render_object,
                paint_cache: None,
            }),
        }
    }

    pub fn update_child_render_object(&self, child_render_object: ArcChildRenderObject<P>) {
        let mut inner = self.inner.lock();
        inner.child_render_object = Some(child_render_object);
        inner.paint_cache = None;
    }
}

impl<P> Layer for RootLayer<P>
where
    P: LayerProtocol,
{
    type ParentCanvas = P::Canvas;

    type ChildCanvas = P::Canvas;

    fn context(&self) -> &AscLayerContextNode {
        &self.context
    }

    fn composite_to(
        &self,
        encoding: &mut <Self::ParentCanvas as Canvas>::Encoding,
        composition_config: &LayerCompositionConfig<Self::ParentCanvas>,
    ) {
        let inner = self.inner.lock();
        let paint_cache = inner
            .paint_cache
            .as_ref()
            .expect("A layer can only be composited after it has finished painting");

        paint_cache.composite_to(encoding, composition_config)
    }

    fn repaint(&self) {
        let mut inner = self.inner.lock();
        if !self.context.needs_paint() && inner.paint_cache.is_some() {
            return;
        }
        inner.paint_cache = Some(
            inner
                .child_render_object
                .as_ref()
                .map(|child_render_object| {
                    P::Canvas::paint_render_object(child_render_object.as_ref())
                })
                .unwrap_or_default(),
        );
    }

    fn as_arc_child_layer(
        self: Arc<Self>,
    ) -> Arc<dyn ChildLayer<ParentCanvas = Self::ParentCanvas>> {
        self
    }

    fn as_arc_parent_layer(
        self: Arc<Self>,
    ) -> Arc<dyn ParentLayer<ChildCanvas = Self::ChildCanvas>> {
        self
    }

    fn as_arc_any_layer(self: Arc<Self>) -> Arc<dyn AnyLayer> {
        self
    }
}
