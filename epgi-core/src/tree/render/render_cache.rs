use crate::foundation::{Intrinsics, Protocol};

use super::{NoRelayoutToken, RenderBase};

#[derive(Default)]
pub(crate) struct RenderCache<R: RenderBase, LC> {
    layout_cache: Option<LayoutCache<R::ParentProtocol, R::LayoutMemo, LC>>,
    intrinsics_cache: Vec<<R::ParentProtocol as Protocol>::Intrinsics>,
}

#[derive(Clone, Copy)]
pub struct LayoutCacheFreshToken(());

impl From<NoRelayoutToken> for LayoutCacheFreshToken {
    fn from(_value: NoRelayoutToken) -> Self {
        Self(())
    }
}

impl<R: RenderBase, LC> RenderCache<R, LC> {
    pub(crate) fn new() -> Self {
        Self {
            layout_cache: None,
            intrinsics_cache: Default::default(),
        }
    }

    pub(crate) fn clear(&mut self) -> LayoutCacheFreshToken {
        self.layout_cache = None;
        self.intrinsics_cache.clear();
        LayoutCacheFreshToken(())
    }

    // The ZST token guards against accidentally accessing staled layout results
    #[inline(always)]
    pub(crate) fn layout_cache_ref(
        &self,
        _token: LayoutCacheFreshToken,
    ) -> Option<&LayoutCache<R::ParentProtocol, R::LayoutMemo, LC>> {
        self.layout_cache.as_ref()
    }

    // The ZST token guards against accidentally accessing staled layout results
    #[inline(always)]
    pub(crate) fn layout_cache_mut(
        &mut self,
        _token: LayoutCacheFreshToken,
    ) -> Option<&mut LayoutCache<R::ParentProtocol, R::LayoutMemo, LC>> {
        self.layout_cache.as_mut()
    }

    // The ZST token guards against accidentally leaving staled intrinsics
    pub(crate) fn insert_layout_results(
        &mut self,
        layout_results: LayoutResults<R::ParentProtocol, R::LayoutMemo>,
        _token: LayoutCacheFreshToken,
    ) -> &mut LayoutCache<R::ParentProtocol, R::LayoutMemo, LC> {
        self.layout_cache.insert(LayoutCache {
            layout_results,
            paint_offset: None,
            layer_cache: None,
        })
    }

    // The ZST token guards against accidentally accessing staled layout results
    pub(crate) fn get_intrinsics_or_insert_with(
        &mut self,
        intrinsics: &mut <R::ParentProtocol as Protocol>::Intrinsics,
        f: impl FnOnce(&mut <R::ParentProtocol as Protocol>::Intrinsics),
        _token: LayoutCacheFreshToken,
    ) {
        if let Some(entry) = self
            .intrinsics_cache
            .iter()
            .find(|entry| intrinsics.eq_param(entry))
        {
            *intrinsics = entry.clone();
        } else {
            f(intrinsics);
            let intrinsics_clone = intrinsics.clone();
            if let Some(entry) = self
                .intrinsics_cache
                .iter_mut()
                .find(|entry| intrinsics.eq_tag(&entry))
            {
                *entry = intrinsics_clone;
            } else {
                self.intrinsics_cache.push(intrinsics_clone);
            }
        }
    }

    // #[inline(always)]
    // pub(crate) fn last_layout_constraints_ref(
    //     &self,
    // ) -> Option<&<R::ParentProtocol as Protocol>::Constraints> {
    //     self.0
    //         .as_ref()
    //         .map(|cache| &cache.layout_results.constraints)
    // }

    // #[inline(always)]
    // pub(crate) fn last_layout_constraints_mut(
    //     &mut self,
    // ) -> Option<&mut <R::ParentProtocol as Protocol>::Constraints> {
    //     self.0
    //         .as_mut()
    //         .map(|cache| &mut cache.layout_results.constraints)
    // }

    #[inline(always)]
    pub(crate) fn last_layout_results_mut(
        &mut self,
    ) -> Option<&mut LayoutResults<R::ParentProtocol, R::LayoutMemo>> {
        self.layout_cache
            .as_mut()
            .map(|cache| &mut cache.layout_results)
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

#[non_exhaustive]
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
    pub fn new(
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
