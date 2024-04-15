use crate::foundation::Protocol;

use super::{NoRelayoutToken, RenderBase};

#[derive(Default)]
pub(crate) struct RenderCache<R: RenderBase, LC>(
    Option<LayoutCache<R::ParentProtocol, R::LayoutMemo, LC>>,
);

impl<R: RenderBase, LC> RenderCache<R, LC> {
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

    pub(crate) fn insert_layout_results(
        &mut self,
        layout_results: LayoutResults<R::ParentProtocol, R::LayoutMemo>,
    ) -> &mut LayoutCache<R::ParentProtocol, R::LayoutMemo, LC> {
        self.0.insert(LayoutCache {
            layout_results,
            paint_offset: None,
            layer_cache: None,
        })
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

pub struct LayoutResults<P: Protocol, M> {
    pub constraints: P::Constraints,
    // pub(crate) parent_use_size: bool,
    pub size: P::Size,
    pub memo: M,
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
