mod context;
mod layer_or_unit;
mod mark;

pub use context::*;
pub use layer_or_unit::*;
pub use mark::*;

use std::sync::atomic::AtomicBool;

use crate::foundation::{Arc, Aweak, HktContainer, PaintContext, Protocol, SyncMutex};

use super::{ArcAnyLayerNode, ArcElementContextNode, ElementContextNode, Layer};

pub type ArcChildRenderObject<P> = Arc<dyn ChildRenderObject<P>>;
pub type ArcAnyRenderObject = Arc<dyn AnyRenderObject>;
pub type AweakAnyRenderObject = Aweak<dyn AnyRenderObject>;
pub type AweakParentRenderObject<P> = Arc<dyn ParentRenderObject<ChildProtocol = P>>;

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug)]
pub enum RenderAction {
    None,
    Recomposite,
    Repaint,
    Relayout,
}

impl Default for RenderAction {
    fn default() -> Self {
        RenderAction::None
    }
}

pub trait Render: Sized + Send + Sync + 'static {
    // type Element: Element<ArcRenderObject = Arc<RenderObject<Self>>>;

    type ParentProtocol: Protocol;
    type ChildProtocol: Protocol;

    type ChildContainer: HktContainer;

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
    const DRY_LAYOUT_FUNCTION_TABLE: Option<DryLayoutFunctionTable<Self>> = None;

    // We don't make perform paint into an associated constant because it has an generic paramter
    // Then we have to go to associated generic type, which makes the boilerplate explodes.
    fn perform_paint(
        &self,
        size: &<Self::ParentProtocol as Protocol>::Size,
        transform: &<Self::ParentProtocol as Protocol>::Transform,
        memo: &Self::LayoutMemo,
        paint_ctx: &mut impl PaintContext<Canvas = <Self::ParentProtocol as Protocol>::Canvas>,
    );

    type LayerOrUnit: LayerOrUnit<Self>;
}

pub trait DryLayout: Render {
    const DRY_LAYOUT_FUNCTION_TABLE: Option<DryLayoutFunctionTable<Self>> =
        Some(DryLayoutFunctionTable {
            compute_dry_layout: Self::compute_dry_layout,
            compute_layout_memo: Self::compute_layout_memo,
        });

    fn compute_dry_layout(
        &self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
    ) -> <Self::ParentProtocol as Protocol>::Size;

    fn compute_layout_memo(
        &self,
        constraints: &<Self::ParentProtocol as Protocol>::Constraints,
        size: &<Self::ParentProtocol as Protocol>::Size,
    ) -> Self::LayoutMemo;
}

pub struct DryLayoutFunctionTable<R: Render> {
    pub compute_dry_layout: fn(
        &R,
        &<R::ParentProtocol as Protocol>::Constraints,
    ) -> <R::ParentProtocol as Protocol>::Size,

    pub compute_layout_memo: for<'a, 'layout> fn(
        &'a R,
        &'a <R::ParentProtocol as Protocol>::Constraints,
        &'a <R::ParentProtocol as Protocol>::Size,
    ) -> R::LayoutMemo,
}

pub trait LayerRender<
    L: Layer<
        ParentCanvas = <Self::ParentProtocol as Protocol>::Canvas,
        ChildCanvas = <Self::ChildProtocol as Protocol>::Canvas,
    >,
>: Render<LayerOrUnit = L>
{
    fn create_layer(&self) -> L;
}

pub struct RenderObject<R: Render> {
    pub(crate) element_context: ArcElementContextNode,
    pub(crate) mark: RenderMark,
    pub(crate) context: AscRenderContextNode,
    pub(crate) layer_node: ArcLayerNodeOf<R>,
    pub(crate) inner: SyncMutex<RenderObjectInner<R>>,
}

impl<R> RenderObject<R>
where
    R: Render,
{
    pub fn new(
        render: R,
        children: <R::ChildContainer as HktContainer>::Container<
            ArcChildRenderObject<R::ChildProtocol>,
        >,
        element_context: ArcElementContextNode,
    ) -> Self {
        debug_assert!(
            element_context.has_render,
            "A render object node must have a render context node in its element context node"
        );
        let layer = match layer_render_function_table_of::<R>() {
            LayerRenderFunctionTable::LayerNode {
                as_arc_child_layer_node,
                create_arc_layer_node: create_layer_node,
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
            LayerRenderFunctionTable::None { create } => create(),
        };
        Self {
            context: element_context.nearest_render_context.clone(),
            element_context,
            mark: RenderMark::new(),
            layer_node: layer,
            inner: SyncMutex::new(RenderObjectInner {
                cache: None,
                render,
                children,
            }),
        }
    }
}

pub(crate) struct RenderObjectInner<R: Render> {
    // parent: Option<AweakParentRenderObject<R::SelfProtocol>>,
    // boundaries: Option<RenderObjectBoundaries>,
    pub(crate) cache: Option<RenderCache<R::ParentProtocol, R::LayoutMemo>>,
    pub(crate) render: R,
    pub(crate) children:
        <R::ChildContainer as HktContainer>::Container<ArcChildRenderObject<R::ChildProtocol>>,
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
    fn detach(&self);
}

impl<R> ChildRenderObject<R::ParentProtocol> for RenderObject<R>
where
    R: Render,
{
    fn as_arc_any_render_object(self: Arc<Self>) -> ArcAnyRenderObject {
        self
    }

    fn detach(&self) {
        todo!()
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
    fn layer(&self) -> Option<ArcAnyLayerNode>;
    fn detach(&self);
}

impl<R> AnyRenderObject for RenderObject<R>
where
    R: Render,
{
    fn element_context(&self) -> &ElementContextNode {
        &self.element_context
    }

    fn layer(&self) -> Option<ArcAnyLayerNode> {
        if let LayerRenderFunctionTable::LayerNode {
            as_arc_any_layer_node,
            ..
        } = layer_render_function_table_of::<R>()
        {
            Some(as_arc_any_layer_node(self.layer_node.clone()))
        } else {
            None
        }
    }

    fn detach(&self) {
        todo!()
    }
}

pub trait ParentRenderObject: Send + Sync + 'static {
    type ChildProtocol: Protocol;
}
