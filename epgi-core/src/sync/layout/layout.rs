use hashbrown::HashSet;

use crate::{
    tree::{
        AweakAnyRenderObject, Element, PerformDryLayout, Render, RenderCache, RenderObject,
        RenderObjectInner,
    },
    foundation::{Arc, Protocol, PtrEq},
    scheduler::get_current_scheduler,
    sync::TreeScheduler,
};

impl TreeScheduler {
    pub(crate) fn perform_layout(
        &mut self,
        boundaries_needing_relayout: HashSet<PtrEq<AweakAnyRenderObject>>,
    ) {
        let mut boundaries_needing_relayout = boundaries_needing_relayout
            .into_iter()
            .filter_map(|x| {
                x.0.upgrade()
                    .filter(|x| !x.element_context().is_unmounted())
            })
            .collect::<Vec<_>>();

        boundaries_needing_relayout.sort_unstable_by_key(|object| object.element_context().depth);

        for boundary in boundaries_needing_relayout {
            boundary.layout_without_resize()
        }
    }
}

impl<R> RenderObjectInner<R>
where
    R: Render,
{
    fn is_relayout_boundary(&self) -> bool {
        R::PERFORM_DRY_LAYOUT.is_some() || self.cache.as_ref().is_some_and(|x| x.parent_use_size)
    }
}

impl<R> RenderObject<R>
where
    R: Render,
{
    fn is_relayout_boundary(&self) -> bool {
        R::PERFORM_DRY_LAYOUT.is_some()
            || self
                .inner
                .lock()
                .cache
                .as_ref()
                .is_some_and(|x| x.parent_use_size)
    }
    fn layout_without_resize(&self) {
        let mut inner = self.inner.lock();
        debug_assert!(inner.is_relayout_boundary());
        let Some(cache) = inner.cache.as_mut() else {
            panic!("Relayout should only be called on relayout boundaries which must retain their layout caches")
        };
        if cache.layout_results(&self.element_context).is_some() {
            return;
        }
        let constraints = cache.constraints.clone();
        let parent_use_size = cache.parent_use_size;
        inner.perform_wet_layout(constraints, parent_use_size);
    }

    fn layout(
        &self,
        constraints: &<<R::Element as Element>::ParentProtocol as Protocol>::Constraints,
    ) {
        let mut inner = self.inner.lock();
        if let Some(cache) = &mut inner.cache {
            if cache.get_layout_for(&constraints).is_some() {
                cache.parent_use_size = false;
                return;
            }
        }
        inner.perform_wet_layout(constraints.clone(), false);
    }

    fn layout_use_size(
        &self,
        constraints: &<<R::Element as Element>::ParentProtocol as Protocol>::Constraints,
    ) -> <<R::Element as Element>::ParentProtocol as Protocol>::Size {
        let mut inner = self.inner.lock();

        if let Some(cache) = &mut inner.cache {
            if let Some(size) = cache.get_layout_for(&constraints) {
                let size = size.clone();
                cache.parent_use_size = false;
                return size;
            }
        }
        inner.perform_wet_layout(constraints.clone(), true).clone()
    }
}
impl<R> RenderObjectInner<R>
where
    R: Render,
{
    #[inline(always)]
    fn perform_wet_layout(
        &mut self,
        constraints: <<R::Element as Element>::ParentProtocol as Protocol>::Constraints,
        parent_use_size: bool,
    ) -> &<<R::Element as Element>::ParentProtocol as Protocol>::Size {
        let (size, memo) = if let Some(PerformDryLayout {
            compute_dry_layout,
            perform_layout,
        }) = R::PERFORM_DRY_LAYOUT
        {
            let size = compute_dry_layout(&self.render, &constraints);
            let memo = perform_layout(&self.render, &constraints, &size);
            (size, memo)
        } else {
            self.render.perform_layout(&constraints)
        };

        return RenderCache::insert_into(&mut self.cache, constraints, parent_use_size, size, memo);
    }
}

pub(crate) mod layout_private {
    use crate::tree::RenderObject;

    use super::*;

    pub trait ChildRenderObjectLayoutExt<PP: Protocol> {
        fn layout_use_size(&self, constraints: &PP::Constraints) -> PP::Size;

        fn layout(&self, constraints: &PP::Constraints);
    }

    impl<R> ChildRenderObjectLayoutExt<<R::Element as Element>::ParentProtocol> for RenderObject<R>
    where
        R: Render,
    {
        fn layout_use_size(
            &self,
            constraints: &<<R::Element as Element>::ParentProtocol as Protocol>::Constraints,
        ) -> <<R::Element as Element>::ParentProtocol as Protocol>::Size {
            self.layout_use_size(constraints)
        }

        fn layout(
            &self,
            constraints: &<<R::Element as Element>::ParentProtocol as Protocol>::Constraints,
        ) {
            self.layout(constraints)
        }
    }

    pub trait AnyRenderObjectRelayoutExt {
        fn layout_without_resize(&self);
    }

    impl<R> AnyRenderObjectRelayoutExt for RenderObject<R>
    where
        R: Render,
    {
        fn layout_without_resize(&self) {
            self.layout_without_resize()
        }
    }
}
