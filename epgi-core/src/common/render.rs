use crate::foundation::{Arc, Aweak, Parallel, Protocol, SyncMutex};

use super::{ArcElementContextNode, Element, ElementContextNode};

pub type ArcChildRenderObject<P> = Arc<dyn ChildRenderObject<P>>;
pub type AweakAnyRenderObject = Aweak<dyn AnyRenderObject>;
pub type AweakParentRenderObject<P> = Arc<dyn ParentRenderObject<ChildProtocol = P>>;

pub trait Render: Sized + Send + Sync + 'static {
    type Element: Element<ArcRenderObject = Arc<RenderObject<Self>>>;

    type ChildIter: Parallel<Item = ArcChildRenderObject<<Self::Element as Element>::ChildProtocol>>
        + Send
        + Sync
        + 'static;
    fn get_children(&self) -> Self::ChildIter;
    fn set_children(&mut self, new_children: Self::ChildIter);

    type LayoutMemo: Send + Sync + 'static;

    // fn perform_layout(
    //     &self,
    //     constraints: &<<Self::Element as Element>::SelfProtocol as Protocol>::Constraints,
    // ) -> (
    //     <<Self::Element as Element>::SelfProtocol as Protocol>::Size,
    //     Self::LayoutMemo,
    // );

    const PERFORM_LAYOUT: PerformLayout<Self>;

    fn perform_paint(
        &self,
        size: &<<Self::Element as Element>::SelfProtocol as Protocol>::Size,
        transformation: &<<Self::Element as Element>::SelfProtocol as Protocol>::CanvasTransformation,
        memo: &Self::LayoutMemo,
        canvas: &mut <<Self::Element as Element>::SelfProtocol as Protocol>::Canvas,
    );

    // fn compute_child_transformation(
    //     transformation: &<<Self::Element as Element>::SelfProtocol as Protocol>::CanvasTransformation,
    //     child_offset: &<<Self::Element as Element>::ChildProtocol as Protocol>::Offset,
    // ) -> <<Self::Element as Element>::ChildProtocol as Protocol>::CanvasTransformation;
}

pub enum PerformLayout<R: Render> {
    WetLayout {
        perform_layout: for<'a, 'layout> fn(
            &'a R,
            &'a <<R::Element as Element>::SelfProtocol as Protocol>::Constraints,
            LayoutExecutor<'a, 'layout>,
        ) -> (
            <<R::Element as Element>::SelfProtocol as Protocol>::Size,
            R::LayoutMemo,
        ),
    },
    /// sized_by_parent == true
    DryLayout {
        compute_dry_layout: fn(
            &R,
            &<<R::Element as Element>::SelfProtocol as Protocol>::Constraints,
        )
            -> <<R::Element as Element>::SelfProtocol as Protocol>::Size,

        perform_layout: for<'a, 'layout> fn(
            &'a R,
            &'a <<R::Element as Element>::SelfProtocol as Protocol>::Constraints,
            &'a <<R::Element as Element>::SelfProtocol as Protocol>::Size,
            LayoutExecutor<'a, 'layout>,
        ) -> R::LayoutMemo,
    },
}

#[derive(Clone, Copy)]
pub struct LayoutExecutor<'a, 'layout> {
    pub scope: &'a rayon::Scope<'layout>,
}

trait WetLayout: Render {
    fn perform_layout<'a, 'layout>(
        &'a self,
        constraints: &<<Self::Element as Element>::SelfProtocol as Protocol>::Constraints,
        executor: LayoutExecutor<'a, 'layout>,
    ) -> (
        <<Self::Element as Element>::SelfProtocol as Protocol>::Size,
        Self::LayoutMemo,
    );

    const PERFORM_LAYOUT: PerformLayout<Self> = PerformLayout::WetLayout {
        perform_layout: Self::perform_layout,
    };
}

impl<R> PerformLayout<R>
where
    R: Render,
{
    pub const fn sized_by_parent(&self) -> bool {
        matches!(self, PerformLayout::DryLayout { .. })
    }
}

pub struct RenderObject<R: Render> {
    element_context: ArcElementContextNode,
    pub(crate) inner: SyncMutex<RenderObjectInner<R>>,
}

pub(crate) struct RenderObjectInner<R: Render> {
    // parent: Option<AweakParentRenderObject<R::SelfProtocol>>,
    boundaries: Option<RenderObjectBoundaries>,
    pub(crate) cache: RenderCache<<R::Element as Element>::SelfProtocol, R::LayoutMemo>,
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

pub struct CacheEntry<K, V> {
    key: K,
    value: Option<V>,
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
        let Some(inner) = &mut self.inner else {return None};
        let Some(layout) = &mut inner.layout else {return None};
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

pub(crate) struct LayoutResults<P: Protocol, M> {
    pub(crate) size: P::Size,
    pub(crate) memo: M,
    pub(crate) paint: Option<PaintResults<P>>,
}

pub(crate) struct PaintResults<P: Protocol> {
    pub(crate) transformation: P::CanvasTransformation,
    pub(crate) encoding_slice: Option<()>, //TODO
}

impl<R> RenderObject<R> where R: Render {}

pub trait ChildRenderObject<SP: Protocol>:
    crate::sync::layout_private::ChildRenderObjectLayoutExt<SP>
    + crate::sync::paint_private::ChildRenderObjectPaintExt<SP>
    + Send
    + Sync
    + 'static
{
}

pub trait AnyRenderObject:
    crate::sync::layout_private::AnyRenderObjectRelayoutExt + Send + Sync + 'static
{
    fn element_context(&self) -> &ElementContextNode;
}

pub trait ParentRenderObject: Send + Sync + 'static {
    type ChildProtocol: Protocol;
}
