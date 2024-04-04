use crate::{
    foundation::{HktContainer, Protocol, SyncMutex},
    tree::{LayerCache, LayerMark},
};

use super::{
    ArcChildRenderObject, ArcElementContextNode, CachedComposite, NoRelayoutToken, Render,
    RenderImpl, RenderMark,
};

pub struct RenderObject<R>
where
    R: Render,
{
    pub(crate) element_context: ArcElementContextNode,
    pub(crate) mark: RenderMark,
    pub(crate) layer_mark: <R::RenderImpl as ImplRenderObject<R>>::LayerMark,
    pub(crate) inner:
        SyncMutex<RenderObjectInner<R, <R::RenderImpl as ImplRenderObject<R>>::LayerCache>>,
}

pub(crate) struct RenderObjectInner<R, C>
where
    R: Render,
{
    // parent: Option<AweakParentRenderObject<R::SelfProtocol>>,
    // boundaries: Option<RenderObjectBoundaries>,
    pub(crate) cache: RenderCache<R, C>,
    pub(crate) render: R,
    pub(crate) children:
        <R::ChildContainer as HktContainer>::Container<ArcChildRenderObject<R::ChildProtocol>>,
}

pub trait ImplRenderObject<R: Render> {
    type LayerMark: Default + Send + Sync;
    type LayerCache: Send + Sync;
}

impl<R: Render, const DRY_LAYOUT: bool, const CACHED_COMPOSITE: bool, const ORPHAN_LAYER: bool>
    ImplRenderObject<R> for RenderImpl<R, DRY_LAYOUT, false, CACHED_COMPOSITE, ORPHAN_LAYER>
{
    type LayerMark = ();
    type LayerCache = ();
}

impl<R: Render, const DRY_LAYOUT: bool, const ORPHAN_LAYER: bool> ImplRenderObject<R>
    for RenderImpl<R, DRY_LAYOUT, true, false, ORPHAN_LAYER>
{
    type LayerMark = LayerMark;
    type LayerCache = LayerCache<<R::ChildProtocol as Protocol>::Canvas, ()>;
}

impl<R: Render, const DRY_LAYOUT: bool, const ORPHAN_LAYER: bool> ImplRenderObject<R>
    for RenderImpl<R, DRY_LAYOUT, true, true, ORPHAN_LAYER>
where
    R: CachedComposite,
{
    type LayerMark = LayerMark;
    type LayerCache = LayerCache<<R::ChildProtocol as Protocol>::Canvas, R::CompositionCache>;
}

#[derive(Default)]
pub(crate) struct RenderCache<R, LC>(Option<LayoutCache<R::ParentProtocol, R::LayoutMemo, LC>>)
where
    R: Render;

impl<R, LC> RenderCache<R, LC>
where
    R: Render,
{
    pub(crate) fn new() -> Self {
        Self(None)
    }

    // The ZST token guards against accidentally accessing staled layout results
    #[inline(always)]
    pub(crate) fn layout_cache_ref(
        &self,
        _token: NoRelayoutToken,
    ) -> Option<&LayoutCache<R::ParentProtocol, R::LayoutMemo, LC>> {
        self.0.as_ref()
    }

    // The ZST token guards against accidentally accessing staled layout results
    #[inline(always)]
    pub(crate) fn layout_cache_mut(
        &mut self,
        _token: NoRelayoutToken,
    ) -> Option<&mut LayoutCache<R::ParentProtocol, R::LayoutMemo, LC>> {
        self.0.as_mut()
    }

    pub(crate) fn insert_layout_cache(
        &mut self,
        cache: LayoutCache<R::ParentProtocol, R::LayoutMemo, LC>,
    ) -> &mut LayoutCache<R::ParentProtocol, R::LayoutMemo, LC> {
        self.0.insert(cache)
    }

    #[inline(always)]
    pub(crate) fn last_layout_constraints_ref(
        &self,
    ) -> Option<&<R::ParentProtocol as Protocol>::Constraints> {
        self.0
            .as_ref()
            .map(|cache| &cache.layout_results.constraints)
    }

    #[inline(always)]
    pub(crate) fn last_layout_constraints_mut(
        &mut self,
    ) -> Option<&mut <R::ParentProtocol as Protocol>::Constraints> {
        self.0
            .as_mut()
            .map(|cache| &mut cache.layout_results.constraints)
    }

    #[inline(always)]
    pub(crate) fn last_layout_results_mut(
        &mut self,
    ) -> Option<&mut LayoutResults<R::ParentProtocol, R::LayoutMemo>> {
        self.0.as_mut().map(|cache| &mut cache.layout_results)
    }
}

pub(crate) struct LayoutCache<P: Protocol, M, LC> {
    pub(crate) layout_results: LayoutResults<P, M>,
    // Because the layer paint is designed to be parallel over dirty render object
    // Therefore we can never guarantee the order between a layer being given its offset, and it being painted into cache
    // Therefore we separate the offset and the layer paint cache into two separate fields.
    pub(crate) paint_offset: Option<P::Offset>,
    pub(crate) layer_cache: Option<LC>,
}

impl<P, M, LC> LayoutCache<P, M, LC>
where
    P: Protocol,
{
    pub(crate) fn new(
        layout_results: LayoutResults<P, M>,
        paint_offset: Option<P::Offset>,
        layer_cache: Option<LC>,
    ) -> Self {
        Self {
            layout_results,
            paint_offset,
            layer_cache,
        }
    }
}

pub(crate) struct LayoutResults<P: Protocol, M> {
    pub(crate) constraints: P::Constraints,
    // pub(crate) parent_use_size: bool,
    pub(crate) size: P::Size,
    pub(crate) memo: M,
}

impl<P, M> LayoutResults<P, M>
where
    P: Protocol,
{
    pub(crate) fn new(
        constraints: P::Constraints,
        // parent_use_size: bool,
        size: P::Size,
        memo: M,
    ) -> Self {
        Self {
            constraints,
            // parent_use_size,
            size,
            memo,
        }
    }
}

// pub(crate) struct RenderCache<P: Protocol, M, PR = ()> {
//     pub(crate) constraints: P::Constraints,
//     pub(crate) parent_use_size: bool,
//     layout_results: Option<LayoutResults<P, M, PR>>,
// }

// impl<P, M, PR> RenderCache<P, M, PR>
// where
//     P: Protocol,
// {
//     pub(crate) fn new(
//         constraints: P::Constraints,
//         parent_use_size: bool,
//         layout_results: Option<LayoutResults<P, M, PR>>,
//     ) -> Self {
//         Self {
//             constraints,
//             parent_use_size,
//             layout_results,
//         }
//     }
//     pub(crate) fn layout_results(&self, mark: &RenderMark) -> Option<&LayoutResults<P, M, PR>> {
//         if mark.needs_layout() {
//             return None;
//         }
//         self.layout_results.as_ref()
//     }
// }

// impl<P, M> RenderCache<P, M>
// where
//     P: Protocol,
// {
//     #[inline]
//     pub fn get_layout_for(&mut self, constraints: &P::Constraints) -> Option<&P::Size> {
//         let Some(layout_results) = &mut self.layout_results else {
//             return None;
//         };
//         if &self.constraints == constraints {
//             return Some(&layout_results.size);
//         }
//         return None;
//     }

//     /// An almost-zero-overhead way to write into cache while holding reference to [Size]
//     pub fn insert_into(
//         dst: &mut Option<Self>,
//         constraints: P::Constraints,
//         parent_use_size: bool,
//         size: P::Size,
//         memo: M,
//     ) -> &P::Size {
//         &dst.insert(RenderCache {
//             constraints,
//             parent_use_size,
//             layout_results: None,
//         })
//         .layout_results
//         .insert(LayoutResults {
//             size,
//             memo,
//             paint_results: None,
//         })
//         .size
//     }

//     /// Return: whether a layout is needed.
//     pub(crate) fn set_root_constraints(
//         dst: &mut Option<Self>,
//         constraints: P::Constraints,
//     ) -> bool {
//         match dst {
//             Some(inner) => {
//                 debug_assert!(
//                     inner.parent_use_size == false,
//                     "Root render object should not have parent_use_size"
//                 );
//                 if inner.constraints.eq(&constraints) {
//                     return false;
//                 }
//                 inner.constraints = constraints;
//                 inner.layout_results = None;
//                 return true;
//             }
//             None => {
//                 *dst = Some(RenderCache {
//                     constraints: constraints,
//                     parent_use_size: false,
//                     layout_results: None,
//                 });
//                 return true;
//             }
//         }
//     }
// }
