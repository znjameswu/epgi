use epgi_core::{
    foundation::{
        Arc, Asc, BuildSuspendedError, Canvas, InlinableDwsizeVec, LayerProtocol, Never,
        PaintContext, Protocol, Provide, SyncMutex,
    },
    tree::{
        ArcChildElementNode, ArcChildRenderObject, ArcChildWidget, ArcElementContextNode,
        AscLayerContextNode, AscRenderContextNode, BuildContext,
        CachingChildLayerProducingIterator, DryLayout, Element, Layer, LayerCompositionConfig,
        LayerPaint, PaintResults, ReconcileItem, Reconciler, Render, RenderElement, RenderObject,
        RenderObjectUpdateResult, StructuredChildLayerOrFragment, Widget, LayerNode, LayerRender,
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
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type Element = RootElement;

    fn into_arc_widget(self: std::sync::Arc<Self>) -> <Self::Element as Element>::ArcWidget {
        self
    }
}

#[derive(Clone)]
pub struct RootElement {
    pub child: Option<ArcChildElementNode<BoxProtocol>>,
}

impl Element for RootElement {
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

    type ArcRenderObject = Arc<RenderObject<RenderRoot>>;
}

impl RenderElement for RootElement {
    type Render = RenderRoot;

    fn try_create_render_object(&self, widget: &Self::ArcWidget) -> Option<Self::Render> {
        todo!()
    }

    fn update_render_object(
        render_object: &mut Self::Render,
        widget: &Self::ArcWidget,
    ) -> RenderObjectUpdateResult {
        RenderObjectUpdateResult::None
    }

    const NOOP_UPDATE_RENDER_OBJECT: bool = true;

    fn try_update_render_object_children(&self, render: &mut Self::Render) -> Result<(), ()> {
        render.child = self.child.as_ref().map(|child| {
            child
                .get_current_subtree_render_object()
                .expect("Root ElementNode should never receive suspense event")
        });
        Ok(())
    }
}

pub struct RenderRoot {
    pub child: Option<ArcChildRenderObject<BoxProtocol>>,
}

impl Render for RenderRoot {
    type ParentProtocol = BoxProtocol;

    type ChildProtocol = BoxProtocol;

    type ChildIter = Option<ArcChildRenderObject<BoxProtocol>>;

    fn children(&self) -> Self::ChildIter {
        self.child.clone()
    }

    type LayoutMemo = ();

    fn perform_layout<'a, 'layout>(
        &'a self,
        _constraints: &'a <Self::ParentProtocol as Protocol>::Constraints,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo) {
        unreachable!()
    }

    const PERFORM_DRY_LAYOUT: Option<epgi_core::tree::PerformDryLayout<Self>> =
        Some(<Self as DryLayout>::PERFORM_DRY_LAYOUT);

    fn perform_paint(
        &self,
        _size: &<Self::ParentProtocol as Protocol>::Size,
        _transform: &<Self::ParentProtocol as Protocol>::Transform,
        _memo: &Self::LayoutMemo,
        _paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    ) {
        unreachable!()
    }

    type ArcLayerNode = Arc<LayerNode<RootLayer>>; //TODO
}

impl LayerRender for RenderRoot {
    fn create_layer_node(&self, layer_context: &AscLayerContextNode) -> Self::ArcLayerNode {
        todo!()
    }
}

impl DryLayout for RenderRoot {
    fn compute_dry_layout(
        &self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
    ) -> <Self::ParentProtocol as Protocol>::Size {
        todo!()
    }

    fn perform_layout<'a, 'layout>(
        &'a self,
        constraints: &'a <Self::ParentProtocol as Protocol>::Constraints,
        size: &'a <Self::ParentProtocol as Protocol>::Size,
    ) -> Self::LayoutMemo {
        // self.render_ctx.resize_surface(&mut self.surface, size.width, size.height)
    }
}

impl LayerPaint for RenderRoot {
    fn get_canvas_transform_ref(
        transform: &<Self::ParentProtocol as Protocol>::Transform,
    ) -> &<<Self::ParentProtocol as Protocol>::Canvas as Canvas>::Transform {
        todo!()
    }

    fn get_canvas_transform(
        transform: <Self::ParentProtocol as Protocol>::Transform,
    ) -> <<Self::ParentProtocol as Protocol>::Canvas as Canvas>::Transform {
        todo!()
    }
}

pub struct RootLayer {
    pub context: AscLayerContextNode,
    pub inner: SyncMutex<RootLayerInner>,
}

pub struct RootLayerInner {
    /// This field is nullable because we temporarily share implementation with RootLayer
    child_render_object: Option<ArcChildRenderObject<BoxProtocol>>,
    paint_cache: Option<PaintResults<Affine2dCanvas>>,
}

impl RootLayer {
    pub fn new(
        context: AscLayerContextNode,
        child_render_object: Option<ArcChildRenderObject<BoxProtocol>>,
    ) -> Self {
        Self {
            context,
            inner: SyncMutex::new(RootLayerInner {
                child_render_object,
                paint_cache: None,
            }),
        }
    }

    pub fn update_child_render_object(&self, child_render_object: ArcChildRenderObject<BoxProtocol>) {
        let mut inner = self.inner.lock();
        inner.child_render_object = Some(child_render_object);
        inner.paint_cache = None;
    }
}

impl Layer for RootLayer {
    type ParentCanvas = Affine2dCanvas;

    type ChildCanvas = Affine2dCanvas;

    fn context(&self) -> &AscLayerContextNode {
        todo!()
    }

    fn composite_to(
        &self,
        encoding: &mut <Self::ParentCanvas as Canvas>::Encoding,
        child_iterator: &mut impl epgi_core::tree::ChildLayerProducingIterator<Self::ChildCanvas>,
        composition_config: &LayerCompositionConfig<Self::ParentCanvas>,
    ) {
        todo!()
    }

    fn transform_config(
        self_config: &LayerCompositionConfig<Self::ParentCanvas>,
        child_config: &LayerCompositionConfig<Self::ChildCanvas>,
    ) -> LayerCompositionConfig<Self::ParentCanvas> {
        todo!()
    }

    fn repaint(
        &self,
        old_results: Option<&PaintResults<Self::ChildCanvas>>,
    ) -> PaintResults<Self::ChildCanvas> {
        todo!()
    }

    fn key(&self) -> Option<&Arc<dyn epgi_core::foundation::Key>> {
        todo!()
    }

    type CachedComposition = ();
}
