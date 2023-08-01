use std::any::Any;

use crate::foundation::{
    Arc, Asc, Aweak, BoolExpectExt, Canvas, PaintContext, Parallel, Protocol, SyncMutex,
};

use super::{
    ArcElementContextNode, ArcLayerOf, Element, ElementContextNode, Layer, LayerFragment,
    LayerScope, RenderElement,
};

pub type ArcChildRenderObject<P> = Arc<dyn ChildRenderObject<P>>;
pub type ArcAnyRenderObject = Arc<dyn AnyRenderObject>;
pub type AweakAnyRenderObject = Aweak<dyn AnyRenderObject>;
pub type AweakParentRenderObject<P> = Arc<dyn ParentRenderObject<ChildProtocol = P>>;

pub trait Render: Sized + Send + Sync + 'static {
    type Element: RenderElement<Render = Self>;

    type ChildIter: Parallel<Item = ArcChildRenderObject<<Self::Element as Element>::ChildProtocol>>
        + Send
        + Sync
        + 'static;
    fn get_children(&self) -> Self::ChildIter;
    fn set_children(&mut self, new_children: Self::ChildIter);

    type LayoutMemo: Send + Sync + 'static;

    fn perform_layout<'a, 'layout>(
        &'a self,
        constraints: &'a <<Self::Element as Element>::ParentProtocol as Protocol>::Constraints,
        executor: LayoutExecutor<'a, 'layout>,
    ) -> (
        <<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        Self::LayoutMemo,
    );

    /// If this is not None, then [`Self::perform_paint`]'s implementation will be ignored.
    const PERFORM_LAYER_PAINT: Option<PerformLayerPaint<Self>> = None;

    // We don't make perform paint into an associated constant because it has an generic paramter
    // Then we have to go to associated generic type, which makes the boilerplate explodes.
    fn perform_paint(
        &self,
        size: &<<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        transformation: &<<Self::Element as Element>::ParentProtocol as Protocol>::SelfTransform,
        memo: &Self::LayoutMemo,
        paint_ctx: impl PaintContext<
            Canvas = <<Self::Element as Element>::ParentProtocol as Protocol>::Canvas,
        >,
    );

    /// If this is not None, then [`Self::perform_layout`]'s implementation will be ignored.
    const PERFORM_DRY_LAYOUT: Option<PerformDryLayout<Self>> = None;

    // fn mark_needs_recompositing(&self) {}

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
        executor: LayoutExecutor<'a, 'layout>,
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
        LayoutExecutor<'a, 'layout>,
    ) -> R::LayoutMemo,
}

trait LayerPaint: Render {
    const PERFORM_LAYER_PAINT: Option<PerformLayerPaint<Self>> = Some(PerformLayerPaint {
        create_layer: Self::create_layer,
        update_layer: Self::update_layer,
        child: Self::child,
    });
    fn create_layer(
        &mut self,
        size: &<<Self::Element as Element>::ParentProtocol as Protocol>::Size,
        transformation: &<<Self::Element as Element>::ParentProtocol as Protocol>::SelfTransform,
        memo: &Self::LayoutMemo,
        parent_layer: &Arc<
            LayerScope<<<Self::Element as Element>::ParentProtocol as Protocol>::Canvas>,
        >,
    ) -> &ArcLayerOf<Self>;
    fn update_layer(
        &mut self,
        transformation: &<<Self::Element as Element>::ParentProtocol as Protocol>::SelfTransform,
    ) -> &ArcLayerOf<Self>;
    fn child(&self) -> &ArcChildRenderObject<<Self::Element as Element>::ChildProtocol>;
}
pub struct PerformLayerPaint<R: Render> {
    pub create_layer: for<'a> fn(
        render: &'a mut R,
        size: &<<R::Element as Element>::ParentProtocol as Protocol>::Size,
        transformation: &<<R::Element as Element>::ParentProtocol as Protocol>::SelfTransform,
        memo: &R::LayoutMemo,
        parent_layer: &Arc<
            LayerScope<<<R::Element as Element>::ParentProtocol as Protocol>::Canvas>,
        >,
    ) -> &'a ArcLayerOf<R>,
    pub update_layer: for<'a> fn(
        render: &'a mut R,
        transformation: &<<R::Element as Element>::ParentProtocol as Protocol>::SelfTransform,
    ) -> &'a ArcLayerOf<R>,
    pub child: fn(render: &R) -> &ArcChildRenderObject<<R::Element as Element>::ChildProtocol>,
}

#[derive(Clone, Copy)]
pub struct LayoutExecutor<'a, 'layout> {
    pub scope: &'a rayon::Scope<'layout>,
}

pub struct RenderObject<R: Render> {
    element_context: ArcElementContextNode,
    pub(crate) inner: SyncMutex<RenderObjectInner<R>>,
}

pub(crate) struct RenderObjectInner<R: Render> {
    // parent: Option<AweakParentRenderObject<R::SelfProtocol>>,
    boundaries: Option<RenderObjectBoundaries>,
    pub(crate) cache: RenderCache<<R::Element as Element>::ParentProtocol, R::LayoutMemo>,
    pub(crate) render: R,
}

struct RenderObjectBoundaries {
    repaint_boundary: AweakAnyRenderObject,
    relayout_boundary: AweakAnyRenderObject,
}

pub(crate) struct RenderCache<P: Protocol, M> {
    pub(crate) inner: Option<RenderCacheInner<P, M>>,
}

pub(crate) struct RenderCacheInner<P: Protocol, M> {
    pub(crate) constraints: P::Constraints,
    pub(crate) parent_use_size: bool,
    pub(crate) layout: Option<LayoutResults<P, M>>,
}

impl<P, M> RenderCache<P, M>
where
    P: Protocol,
{
    /// Return: whether a layout is needed.
    pub(crate) fn set_root_constraints(&mut self, constraints: &P::Constraints) -> bool {
        match &mut self.inner {
            Some(inner) => {
                debug_assert!(
                    inner.parent_use_size == false,
                    "Root render object should not have parent_use_size"
                );
                if inner.constraints.eq(constraints) {
                    return false;
                }
                inner.constraints = constraints.clone();
                inner.layout = None;
                return true;
            }
            None => {
                self.inner = Some(RenderCacheInner {
                    constraints: constraints.clone(),
                    parent_use_size: false,
                    layout: None,
                });
                return true;
            }
        }
    }
}

pub(crate) struct LayoutResults<P: Protocol, M> {
    pub(crate) size: P::Size,
    pub(crate) memo: M,
    pub(crate) paint: Option<PaintResults<P>>,
}

pub(crate) struct PaintResults<P: Protocol> {
    pub(crate) transform_abs: P::SelfTransform,
}

impl<P, M> RenderCache<P, M>
where
    P: Protocol,
{
    #[inline]
    pub(crate) fn parent_use_size(&self) -> Option<bool> {
        self.inner.as_ref().map(|inner| inner.parent_use_size)
    }

    #[inline]
    pub(crate) fn layout_results_ref(&self) -> Option<&LayoutResults<P, M>> {
        self.inner.as_ref().and_then(|inner| inner.layout.as_ref())
    }

    #[inline]
    pub(crate) fn layout_results_mut(&mut self) -> Option<&mut LayoutResults<P, M>> {
        self.inner.as_mut().and_then(|inner| inner.layout.as_mut())
    }

    #[inline]
    pub fn get_layout_for(
        &mut self,
        constraints: &P::Constraints,
        parent_use_size: bool,
    ) -> Option<&P::Size> {
        let Some(inner) = &mut self.inner else {
            return None;
        };
        let Some(layout) = &mut inner.layout else {
            return None;
        };
        if &inner.constraints == constraints {
            inner.parent_use_size = parent_use_size;
            return Some(&layout.size);
        }
        return None;
    }

    pub fn insert_layout_results(
        &mut self,
        constraints: P::Constraints,
        parent_use_size: bool,
        size: P::Size,
        memo: M,
    ) -> &P::Size {
        &self
            .inner
            .insert(RenderCacheInner {
                constraints,
                parent_use_size,
                layout: None,
            })
            .layout
            .insert(LayoutResults {
                size,
                memo,
                paint: None,
            })
            .size
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
}

impl<R> ChildRenderObject<<R::Element as Element>::ParentProtocol> for RenderObject<R> where
    R: Render
{
}

pub trait AnyRenderObject:
    crate::sync::layout_private::AnyRenderObjectRelayoutExt + Send + Sync + 'static
{
    fn element_context(&self) -> &ElementContextNode;

    /// Returns whether the root needs layout after updating constraints.
    ///
    /// Will panic if supplied with wrong type of constraints.
    fn set_root_constraints(&self, constraints: &dyn Any) -> bool;
}

impl<R> AnyRenderObject for RenderObject<R>
where
    R: Render,
{
    fn element_context(&self) -> &ElementContextNode {
        &self.element_context
    }

    fn set_root_constraints(&self, constraints: &dyn Any) -> bool {
        debug_assert!(
            self.element_context.parent.is_none(),
            "set_root_constraints should only be called on tree root"
        );
        let constraints = constraints
            .downcast_ref::<<<R::Element as Element>::ParentProtocol as Protocol>::Constraints>()
            .expect("A correct type of constraints should be passed to root");
        self.inner.lock().cache.set_root_constraints(constraints)
    }
}

pub trait ParentRenderObject: Send + Sync + 'static {
    type ChildProtocol: Protocol;
}
