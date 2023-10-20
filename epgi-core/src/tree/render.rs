mod context;

pub use context::*;

use crate::foundation::{Arc, Aweak, Canvas, Never, PaintContext, Parallel, Protocol, SyncMutex};

use super::{
    ArcAnyLayerNode, ArcChildLayerNode, ArcElementContextNode, AscLayerContextNode, Element,
    ElementContextNode, GetSuspense, Layer, LayerNode,
};

pub type ArcChildRenderObject<P> = Arc<dyn ChildRenderObject<P>>;
pub type ArcAnyRenderObject = Arc<dyn AnyRenderObject>;
pub type AweakAnyRenderObject = Aweak<dyn AnyRenderObject>;
pub type AweakParentRenderObject<P> = Arc<dyn ParentRenderObject<ChildProtocol = P>>;

pub enum RenderObjectUpdateResult {
    None,
    MarkNeedsPaint,
    MarkNeedsLayout,
}

impl Default for RenderObjectUpdateResult {
    fn default() -> Self {
        RenderObjectUpdateResult::None
    }
}

pub trait Render: Sized + Send + Sync + 'static {
    // type Element: Element<ArcRenderObject = Arc<RenderObject<Self>>>;

    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol;

    type ChildIter: Parallel<Item = ArcChildRenderObject<Self::ChildProtocol>>
        + Send
        + Sync
        + 'static;
    fn children(&self) -> Self::ChildIter;

    const IS_REPAINT_BOUNDARY: bool = false;

    fn detach(&mut self) {}

    /// Whether [Render::detach] is a no-op
    ///
    /// When set to true, [Render::detach]'s implementation will be ignored,
    /// Certain optimizations to reduce mutex usages will be applied during the commit phase.
    /// However, if [Render::detach] is actually not no-op, doing this will cause unexpected behaviors.
    ///
    /// Setting to false will always guarantee the correct behavior. And this is why it's left as false.
    /// Render objects that do not manage any external resources may consider setting this to true.
    const NOOP_DETACH: bool = false;

    type LayoutMemo: Send + Sync + 'static;

    fn perform_layout(
        &self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
    ) -> (<Self::ParentProtocol as Protocol>::Size, Self::LayoutMemo);

    /// If this is not None, then [`Self::perform_layout`]'s implementation will be ignored.
    const PERFORM_DRY_LAYOUT: Option<PerformDryLayout<Self>> = None;

    // We don't make perform paint into an associated constant because it has an generic paramter
    // Then we have to go to associated generic type, which makes the boilerplate explodes.
    fn perform_paint(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        transform: &<Self::ParentProtocol as Protocol>::Transform,
        memo: &Self::LayoutMemo,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    );

    /// If this is not None, then [`Self::perform_paint`]'s implementation will be ignored.
    const PERFORM_LAYER_PAINT: Option<PerformLayerPaint<Self>> = None;

    // fn compute_child_transformation(
    //     transformation: &<Self::SelfProtocol as Protocol>::CanvasTransformation,
    //     child_offset: &<Self::ChildProtocol as Protocol>::Offset,
    // ) -> <Self::ChildProtocol as Protocol>::CanvasTransformation;

    type ArcLayerNode: ArcLayerNode<Self>;
}

pub trait DryLayout: Render {
    const PERFORM_DRY_LAYOUT: PerformDryLayout<Self> = PerformDryLayout {
        compute_dry_layout: Self::compute_dry_layout,
        perform_layout: <Self as DryLayout>::perform_layout,
    };

    fn compute_dry_layout(
        &self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
    ) -> <Self::ParentProtocol as Protocol>::Size;

    fn perform_layout<'a, 'layout>(
        &'a self,
        constraints: &'a <Self::ParentProtocol as Protocol>::Constraints,
        size: &'a <Self::ParentProtocol as Protocol>::Size,
    ) -> Self::LayoutMemo;
}

pub struct PerformDryLayout<R: Render> {
    pub compute_dry_layout: fn(
        &R,
        &<R::ParentProtocol as Protocol>::Constraints,
    ) -> <R::ParentProtocol as Protocol>::Size,

    pub perform_layout: for<'a, 'layout> fn(
        &'a R,
        &'a <R::ParentProtocol as Protocol>::Constraints,
        &'a <R::ParentProtocol as Protocol>::Size,
    ) -> R::LayoutMemo,
}

pub trait LayerPaint: Render {
    const PERFORM_LAYER_PAINT: Option<PerformLayerPaint<Self>> = Some(PerformLayerPaint {
        // get_layer: Self::get_layer,
        get_canvas_transform: Self::get_canvas_transform,
        get_canvas_transform_ref: Self::get_canvas_transform_ref,
    });
    // // Returns Arc by value. Since most likely the implementers needs Arc pointer coercion, and pointer coercion results in temporary values whose reference cannot be returned.
    // fn get_layer(&self) -> ArcLayerNodeOf<Self>;

    fn get_canvas_transform_ref(
        transform: &<Self::ParentProtocol as Protocol>::Transform,
    ) -> &<<Self::ParentProtocol as Protocol>::Canvas as Canvas>::Transform;

    fn get_canvas_transform(
        transform: <Self::ParentProtocol as Protocol>::Transform,
    ) -> <<Self::ParentProtocol as Protocol>::Canvas as Canvas>::Transform;
}
pub struct PerformLayerPaint<R: Render> {
    // pub get_layer: fn(&R) -> ArcLayerNodeOf<R>,
    pub get_canvas_transform_ref:
        fn(
            &<R::ParentProtocol as Protocol>::Transform,
        ) -> &<<R::ParentProtocol as Protocol>::Canvas as Canvas>::Transform,
    pub get_canvas_transform: fn(
        <R::ParentProtocol as Protocol>::Transform,
    )
        -> <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Transform,
}

pub trait ArcLayerNode<R>: Clone + Send + Sync + 'static
where
    R: Render<ArcLayerNode = Self>,
{
    type Layer;

    const GET_LAYER_NODE: GetLayerNode<R>;
}

impl<R> ArcLayerNode<R> for ()
where
    R: Render<ArcLayerNode = Self>,
{
    type Layer = Never;

    const GET_LAYER_NODE: GetLayerNode<R> = GetLayerNode::None { create: || () };
}

impl<R, L> ArcLayerNode<R> for Arc<LayerNode<L>>
where
    R: LayerRender<Layer = L>,
    L: Layer<
        ParentCanvas = <R::ParentProtocol as Protocol>::Canvas,
        ChildCanvas = <R::ChildProtocol as Protocol>::Canvas,
    >,
{
    type Layer = L;

    const GET_LAYER_NODE: GetLayerNode<R> = GetLayerNode::LayerNode {
        as_arc_child_layer_node: |x| x,
        create_layer_node: R::create_layer_node,
        get_canvas_transform_ref: todo!(),
        get_canvas_transform: todo!(),
    };
}

pub trait LayerRender: Render<ArcLayerNode = Arc<LayerNode<Self::Layer>>> {
    type Layer: Layer;
    fn create_layer_node(&self, layer_context: &AscLayerContextNode) -> Self::ArcLayerNode;
}

pub enum GetLayerNode<R: Render> {
    LayerNode {
        as_arc_child_layer_node:
            fn(R::ArcLayerNode) -> ArcChildLayerNode<<R::ParentProtocol as Protocol>::Canvas>,
        create_layer_node: fn(&R, &AscLayerContextNode) -> R::ArcLayerNode,
        get_canvas_transform_ref:
            fn(
                &<R::ParentProtocol as Protocol>::Transform,
            ) -> &<<R::ParentProtocol as Protocol>::Canvas as Canvas>::Transform,
        get_canvas_transform: fn(
            <R::ParentProtocol as Protocol>::Transform,
        )
            -> <<R::ParentProtocol as Protocol>::Canvas as Canvas>::Transform,
    },
    // // pub update_layer_node: fn(&R, &R::ArcLayerNode) -> LayerNodeUpdateResult,
    None {
        create: fn() -> R::ArcLayerNode,
    },
}

impl<R> GetLayerNode<R>
where
    R: Render,
{
    pub const fn is_some(&self) -> bool {
        matches!(self, GetLayerNode::LayerNode { .. })
    }
}

// #[derive(Debug, Clone, Copy, Default)]
// pub enum LayerNodeUpdateResult {
//     NeedsRepaint,
//     NeedsRecomposite,
//     #[default]
//     None
// }

pub struct RenderObject<R: Render> {
    pub(crate) element_context: ArcElementContextNode,
    pub(crate) context: AscRenderContextNode,
    pub(crate) layer_node: R::ArcLayerNode,
    pub(crate) inner: SyncMutex<RenderObjectInner<R>>,
}

impl<R> RenderObject<R>
where
    R: Render,
{
    pub fn new(render: R, element_context: ArcElementContextNode) -> Self {
        debug_assert!(
            element_context.has_render,
            "A render object node must have a render context node in its element context node"
        );
        let layer = match R::ArcLayerNode::GET_LAYER_NODE {
            GetLayerNode::LayerNode {
                as_arc_child_layer_node,
                create_layer_node,
                ..
            } => {
                let render_context = &element_context.nearest_render_context;
                debug_assert!(
                    render_context.is_repaint_boundary,
                    "A render object node with layer must have a layer context node \
                     in its render context node"
                );
                create_layer_node(&render, &render_context.nearest_repaint_boundary)
            }
            GetLayerNode::None { create } => create(),
        };
        Self {
            context: element_context.nearest_render_context.clone(),
            element_context,
            layer_node: layer,
            inner: SyncMutex::new(RenderObjectInner {
                cache: None,
                render,
            }),
        }
    }
}

pub(crate) struct RenderObjectInner<R: Render> {
    // parent: Option<AweakParentRenderObject<R::SelfProtocol>>,
    // boundaries: Option<RenderObjectBoundaries>,
    pub(crate) cache: Option<RenderCache<R::ParentProtocol, R::LayoutMemo>>,
    pub(crate) render: R,
}

struct RenderObjectBoundaries {
    repaint_boundary: AweakAnyRenderObject,
    relayout_boundary: AweakAnyRenderObject,
}

pub(crate) struct RenderCache<P: Protocol, M> {
    pub(crate) constraints: P::Constraints,
    pub(crate) parent_use_size: bool,
    layout_results: Option<LayoutResults<P, M>>,
}

impl<P, M> RenderCache<P, M>
where
    P: Protocol,
{
    pub(crate) fn new(
        constraints: P::Constraints,
        parent_use_size: bool,
        layout_results: Option<LayoutResults<P, M>>,
    ) -> Self {
        Self {
            constraints,
            parent_use_size,
            layout_results,
        }
    }
    pub(crate) fn layout_results(
        &self,
        context: &RenderContextNode,
    ) -> Option<&LayoutResults<P, M>> {
        if context.needs_layout() {
            return None;
        }
        self.layout_results.as_ref()
    }
}

pub(crate) struct LayoutResults<P: Protocol, M> {
    pub(crate) size: P::Size,
    pub(crate) memo: M,
}

impl<P, M> RenderCache<P, M>
where
    P: Protocol,
{
    #[inline]
    pub fn get_layout_for(&mut self, constraints: &P::Constraints) -> Option<&P::Size> {
        let Some(layout_results) = &mut self.layout_results else {
            return None;
        };
        if &self.constraints == constraints {
            return Some(&layout_results.size);
        }
        return None;
    }

    /// An almost-zero-overhead way to write into cache while holding reference to [Size]
    pub fn insert_into(
        dst: &mut Option<Self>,
        constraints: P::Constraints,
        parent_use_size: bool,
        size: P::Size,
        memo: M,
    ) -> &P::Size {
        &dst.insert(RenderCache {
            constraints,
            parent_use_size,
            layout_results: None,
        })
        .layout_results
        .insert(LayoutResults { size, memo })
        .size
    }

    /// Return: whether a layout is needed.
    pub(crate) fn set_root_constraints(
        dst: &mut Option<Self>,
        constraints: P::Constraints,
    ) -> bool {
        match dst {
            Some(inner) => {
                debug_assert!(
                    inner.parent_use_size == false,
                    "Root render object should not have parent_use_size"
                );
                if inner.constraints.eq(&constraints) {
                    return false;
                }
                inner.constraints = constraints;
                inner.layout_results = None;
                return true;
            }
            None => {
                *dst = Some(RenderCache {
                    constraints: constraints,
                    parent_use_size: false,
                    layout_results: None,
                });
                return true;
            }
        }
    }
}

impl<R> RenderObject<R> where R: Render {}

pub trait ChildRenderObject<PP: Protocol>:
    crate::sync::layout_private::ChildRenderObjectLayoutExt<PP>
    + crate::sync::paint_private::ChildRenderObjectPaintExt<PP>
    + Send
    + Sync
    + 'static
{
    fn as_arc_any_render_object(self: Arc<Self>) -> ArcAnyRenderObject;
}

impl<R> ChildRenderObject<R::ParentProtocol> for RenderObject<R>
where
    R: Render,
{
    fn as_arc_any_render_object(self: Arc<Self>) -> ArcAnyRenderObject {
        self
    }
}

pub trait AnyRenderObject:
    crate::sync::layout_private::AnyRenderObjectRelayoutExt
    // + crate::sync::paint_private::AnyRenderObjectRepaintExt
    + Send
    + Sync
    + 'static
{
    fn element_context(&self) -> &ElementContextNode;
    fn layer(&self) -> Result<ArcAnyLayerNode, &str>;
}

impl<R> AnyRenderObject for RenderObject<R>
where
    R: Render,
{
    fn element_context(&self) -> &ElementContextNode {
        &self.element_context
    }

    fn layer(&self) -> Result<ArcAnyLayerNode, &str> {
        // let Some(PerformLayerPaint {
        //     get_layer,
        //     ..
        // }) = R::PERFORM_LAYER_PAINT else{
        //     return Err("Layer call should not be called on an RenderObject type that does not associate with a layer")
        // };
        // let inner = self.inner.lock();
        // Ok(get_layer(&inner.render).as_arc_any_layer())
        todo!()
    }
}

pub trait ParentRenderObject: Send + Sync + 'static {
    type ChildProtocol: Protocol;
}
