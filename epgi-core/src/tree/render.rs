mod context;

pub use context::*;

use crate::foundation::{Arc, Aweak, Canvas, PaintContext, Parallel, Protocol, SyncMutex};

use super::{ArcElementContextNode, ArcLayerOf, Element, ElementContextNode, GetSuspense};

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
    type Element: Element<ArcRenderObject = Arc<RenderObject<Self>>>;

    type ChildIter: Parallel<Item = ArcChildRenderObject<<Self::Element as Element>::ChildProtocol>>
        + Send
        + Sync
        + 'static;
    fn children(&self) -> Self::ChildIter;

    fn try_create_render_object_from_element(
        element: &Self::Element,
        widget: &<Self::Element as Element>::ArcWidget,
        context: &AscRenderContextNode,
    ) -> Option<Self>;

    fn update_render_object(
        &mut self,
        widget: &<Self::Element as Element>::ArcWidget,
    ) -> RenderObjectUpdateResult;

    /// Whether [Render::update_render_object] is a no-op and always returns None
    ///
    /// When set to true, [Render::update_render_object]'s implementation will be ignored,
    /// Certain optimizations to reduce mutex usages will be applied during the commit phase.
    /// However, if [Render::update_render_object] is actually not no-op, doing this will cause unexpected behaviors.
    ///
    /// Setting to false will always guarantee the correct behavior.
    const NOOP_UPDATE_RENDER_OBJECT: bool = false;

    fn try_update_render_object_children(&mut self, element: &Self::Element) -> Result<(), ()>;

    /// Whether [Render::try_update_render_object_children] is a no-op and always succeed
    ///
    /// When set to true, [Render::try_update_render_object_children]'s implementation will be ignored,
    /// Certain optimizations to reduce mutex usages will be applied during the commit phase.
    /// However, if [Render::try_update_render_object_children] is actually not no-op, doing this will cause unexpected behaviors.
    ///
    /// Setting to false will always guarantee the correct behavior.
    /// Leaf render objects may consider setting this to true.
    const NOOP_UPDATE_RENDER_OBJECT_CHILDREN: bool = false;

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
        constraints: &<<Self::Element as Element>::ParentProtocol as Protocol>::Constraints,
    ) -> (
        <<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        Self::LayoutMemo,
    );

    /// If this is not None, then [`Self::perform_layout`]'s implementation will be ignored.
    const PERFORM_DRY_LAYOUT: Option<PerformDryLayout<Self>> = None;

    // We don't make perform paint into an associated constant because it has an generic paramter
    // Then we have to go to associated generic type, which makes the boilerplate explodes.
    fn perform_paint(
        &self,
        size: &<<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        transform: &<<Self::Element as Element>::ParentProtocol as Protocol>::Transform,
        memo: &Self::LayoutMemo,
        paint_ctx: &mut impl PaintContext<
            Canvas = <<Self::Element as Element>::ParentProtocol as Protocol>::Canvas,
        >,
    );

    /// If this is not None, then [`Self::perform_paint`]'s implementation will be ignored.
    const PERFORM_LAYER_PAINT: Option<PerformLayerPaint<Self>> = None;

    const GET_SUSPENSE: Option<GetSuspense<Self::Element>> = None;

    // fn compute_child_transformation(
    //     transformation: &<<Self::Element as Element>::SelfProtocol as Protocol>::CanvasTransformation,
    //     child_offset: &<<Self::Element as Element>::ChildProtocol as Protocol>::Offset,
    // ) -> <<Self::Element as Element>::ChildProtocol as Protocol>::CanvasTransformation;
}

pub trait DryLayout: Render {
    const PERFORM_DRY_LAYOUT: PerformDryLayout<Self> = PerformDryLayout {
        compute_dry_layout: Self::compute_dry_layout,
        perform_layout: <Self as DryLayout>::perform_layout,
    };

    fn compute_dry_layout(
        &self,
        constraints: &<<Self::Element as Element>::ParentProtocol as Protocol>::Constraints,
    ) -> <<Self::Element as Element>::ParentProtocol as Protocol>::Size;

    fn perform_layout<'a, 'layout>(
        &'a self,
        constraints: &'a <<Self::Element as Element>::ParentProtocol as Protocol>::Constraints,
        size: &'a <<Self::Element as Element>::ParentProtocol as Protocol>::Size,
    ) -> Self::LayoutMemo;
}

pub struct PerformDryLayout<R: Render> {
    pub compute_dry_layout: fn(
        &R,
        &<<R::Element as Element>::ParentProtocol as Protocol>::Constraints,
    ) -> <<R::Element as Element>::ParentProtocol as Protocol>::Size,

    pub perform_layout: for<'a, 'layout> fn(
        &'a R,
        &'a <<R::Element as Element>::ParentProtocol as Protocol>::Constraints,
        &'a <<R::Element as Element>::ParentProtocol as Protocol>::Size,
    ) -> R::LayoutMemo,
}

pub trait LayerPaint: Render {
    const PERFORM_LAYER_PAINT: Option<PerformLayerPaint<Self>> = Some(PerformLayerPaint {
        get_layer: Self::get_layer,
        get_canvas_transform: Self::get_canvas_transform,
        get_canvas_transform_ref: Self::get_canvas_transform_ref,
    });
    // Returns Arc by value. Since most likely the implementers needs Arc pointer coercion, and pointer coercion results in temporary values whose reference cannot be returned.
    fn get_layer(&self) -> ArcLayerOf<Self>;

    fn get_canvas_transform_ref(
        transform: &<<Self::Element as Element>::ParentProtocol as Protocol>::Transform,
    ) -> &<<<Self::Element as Element>::ParentProtocol as Protocol>::Canvas as Canvas>::Transform;

    fn get_canvas_transform(
        transform: <<Self::Element as Element>::ParentProtocol as Protocol>::Transform,
    ) -> <<<Self::Element as Element>::ParentProtocol as Protocol>::Canvas as Canvas>::Transform;
}
pub struct PerformLayerPaint<R: Render> {
    pub get_layer: fn(&R) -> ArcLayerOf<R>,
    pub get_canvas_transform_ref:
        fn(
            &<<R::Element as Element>::ParentProtocol as Protocol>::Transform,
        )
            -> &<<<R::Element as Element>::ParentProtocol as Protocol>::Canvas as Canvas>::Transform,
    pub get_canvas_transform:
        fn(
            <<R::Element as Element>::ParentProtocol as Protocol>::Transform,
        )
            -> <<<R::Element as Element>::ParentProtocol as Protocol>::Canvas as Canvas>::Transform,
}

pub struct RenderObject<R: Render> {
    pub(crate) element_context: ArcElementContextNode,
    pub(crate) context: AscRenderContextNode,
    pub(crate) inner: SyncMutex<RenderObjectInner<R>>,
}

impl<R> RenderObject<R>
where
    R: Render,
{
    pub fn new(render: R, element_context: ArcElementContextNode) -> Self {
        debug_assert!(
            element_context.has_render,
            "A render object node must construct a render context node in its element context ndoe"
        );
        Self {
            context: element_context.nearest_render_context.clone(),
            element_context,
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
    pub(crate) cache: Option<RenderCache<<R::Element as Element>::ParentProtocol, R::LayoutMemo>>,
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

impl<R> ChildRenderObject<<R::Element as Element>::ParentProtocol> for RenderObject<R>
where
    R: Render,
{
    fn as_arc_any_render_object(self: Arc<Self>) -> ArcAnyRenderObject {
        self
    }
}

pub trait AnyRenderObject:
    crate::sync::layout_private::AnyRenderObjectRelayoutExt
    + crate::sync::paint_private::AnyRenderObjectRepaintExt
    + Send
    + Sync
    + 'static
{
    fn element_context(&self) -> &ElementContextNode;
}

impl<R> AnyRenderObject for RenderObject<R>
where
    R: Render,
{
    fn element_context(&self) -> &ElementContextNode {
        &self.element_context
    }
}

pub trait ParentRenderObject: Send + Sync + 'static {
    type ChildProtocol: Protocol;
}
