// use crate::foundation::{
//     Arc, Aweak, BuildSuspendedError, Canvas, Identity, InlinableDwsizeVec, LayerProtocol, Never,
//     PaintContext, Protocol, Provide, SyncMutex,
// };

// use crate::tree::{
//     AnyLayer, ArcAnyLayer, ArcChildElementNode, ArcChildRenderObject, ArcChildWidget,
//     ArcElementContextNode, ArcLayerOf, AscLayerContextNode, AscRenderContextNode, ChildLayer,
//     ChildLayerOrFragment, ComposableChildLayer, Element, Layer, LayerCompositionConfig, LayerPaint,
//     PaintResults, ParentLayer, Reconciler, Render, RenderObject, RenderObjectUpdateResult, Widget,
// };

// #[derive(Debug)]
// pub struct RepaintBoundary<P: LayerProtocol> {
//     child: ArcChildWidget<P>,
// }

// impl<P> Widget for RepaintBoundary<P>
// where
//     P: LayerProtocol,
// {
//     type Element = RepaintBoundaryElement<P>;

//     fn into_arc_widget(self: Arc<Self>) -> Self::ArcWidget {
//         self
//     }
// }

// #[derive(Clone)]
// pub struct RepaintBoundaryElement<P: LayerProtocol> {
//     child: ArcChildElementNode<P>,
// }

// impl<P> Element for RepaintBoundaryElement<P>
// where
//     P: LayerProtocol,
// {
//     type ArcWidget = Arc<RepaintBoundary<P>>;

//     type ParentProtocol = P;

//     type ChildProtocol = P;

//     type Provided = Never;

//     fn perform_rebuild_element(
//         self,
//         widget: &Self::ArcWidget,
//         provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
//         reconciler: impl Reconciler<Self::ChildProtocol>,
//     ) -> Result<Self, (Self, BuildSuspendedError)> {
//         todo!()
//     }

//     fn perform_inflate_element(
//         widget: &Self::ArcWidget,
//         provider_values: InlinableDwsizeVec<Arc<dyn Provide>>,
//         reconciler: impl Reconciler<Self::ChildProtocol>,
//     ) -> Result<Self, BuildSuspendedError> {
//         todo!()
//     }

//     type ChildIter = [ArcChildElementNode<P>; 1];

//     fn children(&self) -> Self::ChildIter {
//         [self.child.clone()]
//     }

//     type ArcRenderObject = Arc<RenderObject<RenderRepaintBoundary<P>>>;
// }

// pub struct RenderRepaintBoundary<P: LayerProtocol> {
//     layer: Arc<RepaintBoundaryLayer<P>>,
//     pub(crate) child: ArcChildRenderObject<P>,
// }

// impl<P> Render for RenderRepaintBoundary<P>
// where
//     P: LayerProtocol,
// {
//     type Element = RepaintBoundaryElement<P>;

//     type ChildIter = [ArcChildRenderObject<P>; 1];

//     fn children(&self) -> Self::ChildIter {
//         todo!()
//     }

//     fn try_create_render_object_from_element(
//         element: &Self::Element,
//         _widget: &Self::ArcWidget,
//         context: &AscRenderContextNode,
//     ) -> Option<Self> {
//         let child = element.child.get_current_subtree_render_object()?;
//         assert!(
//             context.is_repaint_boundary(),
//             concat!(
//                 "Repaint boundaries must be registered in its RenderContextNode. \n",
//                 "If this assertion failed, you have encountered a framework bug."
//             )
//         );
//         let layer = Arc::new(RepaintBoundaryLayer::new(
//             context.nearest_repaint_boundary.clone(),
//             child.clone(),
//         ));
//         Some(Self { layer, child })
//     }

//     fn update_render_object(
//         &mut self,
//         _widget: &Self::ArcWidget,
//     ) -> RenderObjectUpdateResult {
//         RenderObjectUpdateResult::None
//     }

//     fn try_update_render_object_children(&mut self, element: &Self::Element) -> Result<(), ()> {
//         let Some(child) = element.child.get_current_subtree_render_object() else {
//             return Err(())
//         };
//         self.layer.update_child_render_object(child.clone());
//         self.child = child;
//         Ok(())
//     }

//     const NOOP_DETACH: bool = true;

//     type LayoutMemo = ();

//     fn perform_layout<'a, 'layout>(
//         &'a self,
//         constraints: &'a <Self::ParentProtocol as Protocol>::Constraints,
//     ) -> (
//         <Self::ParentProtocol as Protocol>::Size,
//         Self::LayoutMemo,
//     ) {
//         todo!()
//     }

//     fn perform_paint(
//         &self,
//         size: &<Self::ParentProtocol as Protocol>::Size,
//         transform: &<Self::ParentProtocol as Protocol>::Transform,
//         memo: &Self::LayoutMemo,
//         paint_ctx: &mut impl PaintContext<
//             Canvas = <Self::ParentProtocol as Protocol>::Canvas,
//         >,
//     ) {
//         todo!()
//     }
// }

// impl<P: Protocol> LayerPaint for RenderRepaintBoundary<P>
// where
//     P: LayerProtocol,
// {
//     fn get_layer(&self) -> ArcLayerOf<Self> {
//         self.layer.clone() as _
//         // if let Some(layer) = &self.layer {
//         //     return layer.clone() as _;
//         // }

//         // assert!(
//         //     node.context.is_repaint_boundary(),
//         //     concat!(
//         //         "Repaint boundaries must be registered in its RenderContextNode. \n",
//         //         "If this assertion failed, you have encountered a framework bug."
//         //     )
//         // );
//         // let layer = Arc::new(RepaintBoundaryLayer {
//         //     render_object: Arc::downgrade(node),
//         //     context: node.context.nearest_repaint_boundary.clone(),
//         //     inner: SyncMutex::new(RepaintBoundaryLayerInner { paint_cache: None }),
//         // });
//         // self.layer = Some(layer.clone());
//         // return layer;
//     }

//     fn get_canvas_transform_ref(
//         transform: &<Self::ParentProtocol as Protocol>::Transform,
//     ) -> &<<Self::ParentProtocol as Protocol>::Canvas as Canvas>::Transform
//     {
//         transform
//     }

//     fn get_canvas_transform(
//         transform: <Self::ParentProtocol as Protocol>::Transform,
//     ) -> <<Self::ParentProtocol as Protocol>::Canvas as Canvas>::Transform
//     {
//         transform
//     }
// }

// pub struct RepaintBoundaryLayer<P: LayerProtocol> {
//     pub context: AscLayerContextNode,
//     pub inner: SyncMutex<RepaintBoundaryLayerInner<P>>,
// }

// pub struct RepaintBoundaryLayerInner<P: LayerProtocol> {
//     /// This field is nullable because we temporarily share implementation with RootLayer
//     child_render_object: ArcChildRenderObject<P>,
//     paint_cache: Option<PaintResults<P::Canvas>>,
// }

// impl<P> RepaintBoundaryLayer<P>
// where
//     P: LayerProtocol,
// {
//     pub fn new(context: AscLayerContextNode, child_render_object: ArcChildRenderObject<P>) -> Self {
//         Self {
//             context,
//             inner: SyncMutex::new(RepaintBoundaryLayerInner {
//                 child_render_object,
//                 paint_cache: None,
//             }),
//         }
//     }

//     pub fn update_child_render_object(&self, child_render_object: ArcChildRenderObject<P>) {
//         let mut inner = self.inner.lock();
//         inner.child_render_object = child_render_object;
//         inner.paint_cache = None;
//     }
// }

// impl<P> Layer for RepaintBoundaryLayer<P>
// where
//     P: LayerProtocol,
// {
//     type ParentCanvas = P::Canvas;

//     type ChildCanvas = P::Canvas;

//     fn context(&self) -> &AscLayerContextNode {
//         &self.context
//     }

//     fn composite(
//         &self,
//         encoding: &mut <Self::ParentCanvas as Canvas>::Encoding,
//         composition_config: &LayerCompositionConfig<Self::ParentCanvas>,
//     ) {
//         let inner = self.inner.lock();
//         let paint_cache = inner
//             .paint_cache
//             .as_ref()
//             .expect("A layer can only be composited after it has finished painting");

//         paint_cache.composite_to(encoding, composition_config)
//     }

//     fn repaint(&self) {
//         let mut inner = self.inner.lock();
//         if !self.context.needs_paint() && inner.paint_cache.is_some() {
//             return;
//         }
//         inner.paint_cache = Some(P::Canvas::paint_render_object(
//             inner.child_render_object.as_ref(),
//         ));
//     }

//     fn painted_children(&self) -> Option<Vec<ArcAnyLayer>> {
//         let inner = self.inner.lock();
//         let paint_cache = inner.paint_cache.as_ref()?;
//         let children = paint_cache
//             .structured_children
//             .iter()
//             .filter_map(|child| match child {
//                 ChildLayerOrFragment::Fragment(_) => None,
//                 ChildLayerOrFragment::Layer(ComposableChildLayer { layer, .. }) => {
//                     Some(layer.clone().as_arc_any_layer())
//                 }
//             })
//             .chain(
//                 paint_cache
//                     .detached_children
//                     .iter()
//                     .map(|child| child.layer.clone()),
//             )
//             .collect::<Vec<_>>();
//         Some(children)
//     }

//     fn composited_children(&self) -> Option<Vec<ComposableChildLayer<Self::ChildCanvas>>> {
//         let inner = self.inner.lock();
//         let paint_cache = inner.paint_cache.as_ref()?;
//         let composition_results = paint_cache.composition_results.as_ref()?;
//         let children = paint_cache
//             .structured_children
//             .iter()
//             .filter_map(|child| match child {
//                 ChildLayerOrFragment::Fragment(_) => None,
//                 ChildLayerOrFragment::Layer(layer) => {
//                     Some(layer.clone().as_arc_any_layer())
//                 }
//             })
//             .chain(
//                 paint_cache
//                     .detached_children
//                     .iter()
//                     .map(|child| child.layer.clone()),
//             )
//             .collect::<Vec<_>>();
//         Some(children)
//     }

//     fn as_arc_child_layer(
//         self: Arc<Self>,
//     ) -> Arc<dyn ChildLayer<ParentCanvas = Self::ParentCanvas>> {
//         self
//     }

//     fn as_arc_parent_layer(
//         self: Arc<Self>,
//     ) -> Arc<dyn ParentLayer<ChildCanvas = Self::ChildCanvas>> {
//         self
//     }

//     fn as_arc_any_layer(self: Arc<Self>) -> Arc<dyn AnyLayer> {
//         self
//     }
// }
